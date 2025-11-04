/// TypeView trait enables zero-copy pattern matching over type representations.
///
/// This trait abstracts over different type representations (arena-allocated,
/// encoded bytes, indexed database) allowing generic algorithms to work with
/// any representation.
///
/// # Bounds
///
/// - `Sized`: Required for return types and avoiding trait objects
/// - `Copy`: All implementations are lightweight handles (references, indices)
/// - `Eq`: Enables equality checks (pointer equality for `&Type`, byte equality for encoded)
pub trait TypeView<'a>: Sized + Copy + Eq {
    type Iter: Iterator<Item = Self>;
    type NamedIter: Iterator<Item = (&'a str, Self)>;
    type StrIter: Iterator<Item = &'a str>;

    fn view(self) -> TypeKind<'a, Self>;
}

#[repr(C, u8)]
pub enum TypeKind<'a, T: TypeView<'a>> {
    TypeVar(u16) = 0,
    Int = 1,
    Float = 2,
    Bool = 3,
    Str = 4,
    Bytes = 5,
    Array(T) = 6,
    Map(T, T) = 7,
    Record(T::NamedIter) = 8, // Must be sorted by field name.
    Function { params: T::Iter, ret: T } = 9,
    Symbol(T::StrIter) = 10, // Must be sorted.
}

/// TypeConstructor trait enables building type representations.
///
/// This trait abstracts over type construction, allowing generic algorithms
/// to build types in any representation (arena-allocated `&Type`, encoded bytes,
/// AST nodes, etc.).
///
/// # Example
///
/// ```ignore
/// impl<'a> TypeConstructor<'a> for &'a TypeManager<'a> {
///     type Repr = &'a Type<'a>;
///
///     fn int(&mut self) -> Self::Repr {
///         self.intern(Type::Int)
///     }
///     // ... other methods
/// }
/// ```
///
/// Used with `TypeTransformer` to enable generic type transformations:
/// - Alpha-conversion (variable renaming)
/// - Type substitution
/// - Format conversion (EncodedType → &Type)
pub trait TypeConstructor<'a> {
    /// The type representation this constructor builds
    type Repr: TypeView<'a>;

    // Primitives
    fn int(&self) -> Self::Repr;
    fn float(&self) -> Self::Repr;
    fn bool(&self) -> Self::Repr;
    fn str(&self) -> Self::Repr;
    fn bytes(&self) -> Self::Repr;

    // Type variable
    fn typevar(&self, id: u16) -> Self::Repr;

    // Collections
    fn array(&self, elem: Self::Repr) -> Self::Repr;
    fn map(&self, key: Self::Repr, val: Self::Repr) -> Self::Repr;

    // Structural types
    //
    // Note: These accept `impl Iterator` rather than the specific iterator types from TypeView
    // (e.g., `<Self::Repr as TypeView>::NamedIter`) because TypeTransformer needs to map over
    // the iterators, producing new iterator types. Using `impl Iterator` provides the flexibility
    // needed for transformations while maintaining type safety on the item types.
    fn record(&self, fields: impl Iterator<Item = (&'a str, Self::Repr)>) -> Self::Repr;
    fn function(&self, params: impl Iterator<Item = Self::Repr>, ret: Self::Repr) -> Self::Repr;
    fn symbol(&self, parts: impl Iterator<Item = &'a str>) -> Self::Repr;
}

/// TypeTransformer trait enables generic type transformations.
///
/// This trait provides a framework for walking type structures and rebuilding them,
/// optionally transforming parts along the way. The default `transform` method
/// recursively walks the type structure using `TypeView` and rebuilds it using
/// `TypeConstructor`.
///
/// # Customization
///
/// Implementations can override specific cases by implementing custom transformation
/// logic. For example:
///
/// ```ignore
/// struct AlphaConverter<'a, C> {
///     constructor: C,
///     mapping: HashMap<u16, u16>,
///     _phantom: PhantomData<&'a ()>,
/// }
///
/// impl<'a, C: TypeConstructor<'a>> TypeTransformer<'a, C> for AlphaConverter<'a, C> {
///     fn constructor(&mut self) -> &mut C {
///         &mut self.constructor
///     }
///
///     fn transform<V: TypeView<'a>>(&mut self, ty: V) -> C::Repr {
///         match ty.view() {
///             TypeKind::TypeVar(id) => {
///                 // Custom handling: rename variable
///                 let new_id = self.mapping.get(&id).copied().unwrap_or(id);
///                 self.constructor().typevar(new_id)
///             }
///             // All other cases handled by default implementation
///             _ => self.transform_default(ty),
///         }
///     }
/// }
/// ```
///
/// # Common Use Cases
///
/// - **Alpha-conversion**: Renaming type variables
/// - **Type substitution**: Replacing type variables with concrete types
/// - **Format conversion**: Converting between representations (e.g., `EncodedType → &Type`)
/// - **Type normalization**: Simplifying or canonicalizing types
pub trait TypeTransformer<'a, C: TypeConstructor<'a>> {
    /// Access the underlying type constructor
    fn constructor(&self) -> &C;

    /// Transform a type from any representation to the constructor's representation.
    ///
    /// The default implementation recursively walks the type structure:
    /// - Primitives are reconstructed as-is
    /// - Type variables are preserved (override to customize)
    /// - Collections and structural types are recursively transformed
    fn transform<V: TypeView<'a>>(&self, ty: V) -> C::Repr {
        self.transform_default(ty)
    }

    /// Default transformation logic (used by default `transform` and available for
    /// custom implementations that want to delegate some cases).
    fn transform_default<V: TypeView<'a>>(&self, ty: V) -> C::Repr {
        match ty.view() {
            // Primitives - reconstruct as-is
            TypeKind::Int => self.constructor().int(),
            TypeKind::Float => self.constructor().float(),
            TypeKind::Bool => self.constructor().bool(),
            TypeKind::Str => self.constructor().str(),
            TypeKind::Bytes => self.constructor().bytes(),

            // Type variable - preserve ID (override transform() to customize)
            TypeKind::TypeVar(id) => self.constructor().typevar(id),

            // Collections - recursively transform elements
            TypeKind::Array(elem) => {
                let elem_transformed = self.transform(elem);
                self.constructor().array(elem_transformed)
            }
            TypeKind::Map(key, val) => {
                let key_transformed = self.transform(key);
                let val_transformed = self.transform(val);
                self.constructor().map(key_transformed, val_transformed)
            }

            // Structural types - recursively transform all parts
            TypeKind::Record(fields) => {
                let fields_transformed = fields.map(|(name, ty)| (name, self.transform(ty)));
                self.constructor().record(fields_transformed)
            }
            TypeKind::Function { params, ret } => {
                let params_transformed = params.map(|ty| self.transform(ty));
                let ret_transformed = self.transform(ret);
                self.constructor()
                    .function(params_transformed, ret_transformed)
            }
            TypeKind::Symbol(parts) => {
                // Symbol parts are just strings, no transformation needed
                self.constructor().symbol(parts)
            }
        }
    }
}

// ============================================================================
// Generic Type Display
// ============================================================================

/// Format a type for display, works with any `TypeView` implementation.
///
/// This function produces the same output as `Display for Type`, but works
/// generically over any type representation that implements `TypeView`.
///
/// # Format
///
/// - Primitives: `Int`, `Float`, `Bool`, `Str`, `Bytes`
/// - Type variables: `_0`, `_42`, etc.
/// - Collections: `Array[Int]`, `Map[Str, Int]`
/// - Records: `Record[x: Int, y: Float]`
/// - Functions: `(Int, Float) => Str`
/// - Symbols: `Symbol[foo|bar|baz]`
///
/// # Example
///
/// ```ignore
/// use crate::types::{TypeManager, type_traits::display_type};
/// use bumpalo::Bump;
///
/// let bump = Bump::new();
/// let mgr = TypeManager::new(&bump);
/// let int_ty = mgr.int();
/// let arr_ty = mgr.array(int_ty);
///
/// assert_eq!(display_type(arr_ty), "Array[Int]");
/// ```
pub fn display_type<'a, V: TypeView<'a>>(ty: V) -> alloc::string::String {
    use alloc::string::ToString;

    match ty.view() {
        TypeKind::Int => "Int".to_string(),
        TypeKind::Float => "Float".to_string(),
        TypeKind::Bool => "Bool".to_string(),
        TypeKind::Str => "Str".to_string(),
        TypeKind::Bytes => "Bytes".to_string(),

        TypeKind::TypeVar(id) => alloc::format!("_{}", id),

        TypeKind::Array(elem) => {
            alloc::format!("Array[{}]", display_type(elem))
        }

        TypeKind::Map(key, val) => {
            alloc::format!("Map[{}, {}]", display_type(key), display_type(val))
        }

        TypeKind::Record(fields) => {
            let field_strs: alloc::vec::Vec<alloc::string::String> = fields
                .map(|(name, field_ty)| alloc::format!("{}: {}", name, display_type(field_ty)))
                .collect();
            alloc::format!("Record[{}]", field_strs.join(", "))
        }

        TypeKind::Function { params, ret } => {
            let param_strs: alloc::vec::Vec<alloc::string::String> =
                params.map(|param_ty| display_type(param_ty)).collect();
            alloc::format!("({}) => {}", param_strs.join(", "), display_type(ret))
        }

        TypeKind::Symbol(parts) => {
            let part_strs: alloc::vec::Vec<alloc::string::String> =
                parts.map(|p| p.to_string()).collect();
            alloc::format!("Symbol[{}]", part_strs.join("|"))
        }
    }
}
