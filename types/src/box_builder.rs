use crate::ir::{TyData, TypeBuilder, TypeKind};
use alloc::rc::Rc;
use alloc::vec::Vec;

/// Interner that uses reference counting (no deduplication).
///
/// Types are allocated with `Rc` and no interning is performed.
/// This is useful for:
/// - Testing (simpler than arena)
/// - Situations where deduplication isn't needed
/// - Comparing performance with/without interning
///
/// Following Chalk's design, we compute type flags during interning and
/// wrap the TyKind in TyData.
///
/// # Example
///
/// ```
/// use melbi_types::{TypeBuilder, BoxBuilder, Scalar, TypeKind};
///
/// let builder = BoxBuilder::new();
/// let int_ty = TypeKind::Scalar(Scalar::Int).intern(builder);
/// let arr_ty = TypeKind::Array(int_ty).intern(builder);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BoxBuilder;

impl BoxBuilder {
    /// Create a new box builder.
    pub fn new() -> Self {
        Self
    }
}

impl TypeBuilder for BoxBuilder {
    type TypeView = crate::Ty<Self>;
    type InternedTy = Rc<TyData<Self>>;
    type InternedStr = Rc<str>;
    type InternedTypes = Rc<[crate::Ty<Self>]>;
    type InternedFieldTypes = Rc<[(Rc<str>, crate::Ty<Self>)]>;
    type InternedSymbolParts = Rc<[Rc<str>]>;

    // ========================================================================
    // High-level type constructors
    // ========================================================================

    fn type_var(self, id: u16) -> Self::TypeView {
        TypeKind::TypeVar(id).intern(self)
    }

    fn int(self) -> Self::TypeView {
        TypeKind::Scalar(crate::Scalar::Int).intern(self)
    }

    fn float(self) -> Self::TypeView {
        TypeKind::Scalar(crate::Scalar::Float).intern(self)
    }

    fn bool(self) -> Self::TypeView {
        TypeKind::Scalar(crate::Scalar::Bool).intern(self)
    }

    fn str(self) -> Self::TypeView {
        TypeKind::Scalar(crate::Scalar::Str).intern(self)
    }

    fn bytes(self) -> Self::TypeView {
        TypeKind::Scalar(crate::Scalar::Bytes).intern(self)
    }

    fn array(self, elem: Self::TypeView) -> Self::TypeView {
        TypeKind::Array(elem).intern(self)
    }

    fn map(self, key: Self::TypeView, val: Self::TypeView) -> Self::TypeView {
        TypeKind::Map(key, val).intern(self)
    }

    fn record(
        self,
        fields: impl IntoIterator<Item = (impl AsRef<str>, Self::TypeView)>,
    ) -> Self::TypeView {
        TypeKind::Record(self.intern_field_types(fields)).intern(self)
    }

    fn function(
        self,
        params: impl IntoIterator<Item = Self::TypeView>,
        ret: Self::TypeView,
    ) -> Self::TypeView {
        TypeKind::Function {
            params: self.intern_types(params),
            ret,
        }
        .intern(self)
    }

    fn symbol(self, parts: impl IntoIterator<Item = impl AsRef<str>>) -> Self::TypeView {
        TypeKind::Symbol(self.intern_symbol_parts(parts)).intern(self)
    }

    // ========================================================================
    // Low-level internals
    // ========================================================================

    fn intern_ty(self, kind: TypeKind<Self>) -> Self::InternedTy {
        // Compute flags from the type kind
        let flags = kind.compute_flags(self);

        // Wrap in TyData and allocate
        Rc::new(TyData { kind, flags })
    }

    fn ty_data(self, ty: &Self::InternedTy) -> &TyData<Self> {
        ty
    }

    fn intern_types<E>(self, data: impl IntoIterator<Item = E>) -> Self::InternedTypes
    where
        E: Into<crate::Ty<Self>>,
    {
        let types: Vec<_> = data.into_iter().map(|e| e.into()).collect();
        types.into()
    }

    fn types_data(self, types: &Self::InternedTypes) -> &[crate::Ty<Self>] {
        types
    }

    fn intern_field_types(
        self,
        data: impl IntoIterator<Item = (impl AsRef<str>, crate::Ty<Self>)>,
    ) -> Self::InternedFieldTypes {
        // Intern all field name strings as Rc<str>
        let mut fields: Vec<(Rc<str>, crate::Ty<Self>)> = data
            .into_iter()
            .map(|(name, ty)| (Rc::from(name.as_ref()), ty))
            .collect();

        // Sort by field name for canonical representation
        fields.sort_by(|(a, _), (b, _)| a.cmp(b));

        fields.into()
    }

    fn field_types_data(
        self,
        fields: &Self::InternedFieldTypes,
    ) -> &[(Self::InternedStr, crate::Ty<Self>)] {
        fields.as_ref()
    }

    fn intern_symbol_parts(
        self,
        data: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Self::InternedSymbolParts {
        // Intern all strings as Rc<str>
        let mut parts: Vec<Rc<str>> = data
            .into_iter()
            .map(|part| Rc::from(part.as_ref()))
            .collect();

        // Sort for canonical representation
        parts.sort();

        parts.into()
    }

    fn symbol_parts_data(self, parts: &Self::InternedSymbolParts) -> &[Self::InternedStr] {
        parts.as_ref()
    }
}
