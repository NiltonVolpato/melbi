//! TypeBuilder trait - unified type construction and storage abstraction.
//!
//! This trait combines:
//! - High-level type constructors (int(), array(), record(), etc.)
//! - Low-level storage management (interning, deduplication)
//! - Pluggable storage strategies (arena, RC, etc.)

use crate::{TyData, TypeKind};
use core::fmt::Debug;
use core::hash::Hash;

/// Abstraction over type construction and storage.
///
/// This trait allows different implementations to choose how types
/// are built and stored in memory (arena, box, encoded bytes, etc.)
/// while keeping the type system logic generic.
///
/// # Design
///
/// The builder pattern separates:
/// - **What a type is** (`TypeKind<B>`) - the logical structure
/// - **How types are stored** (`B::InternedTy`) - the representation
/// - **How to construct types** (`B::int()`, `B::array()`, etc.) - the API
///
/// This enables:
/// - Multiple storage strategies (arena, RC-based, encoded)
/// - Ergonomic type construction with builder methods
/// - Zero-cost abstraction via monomorphization
/// - Easy testing with lightweight mock builders
///
/// # Requirements
///
/// Builders must be `Copy` so they can be passed around cheaply.
/// All interned types must implement basic traits for comparison and debugging.
///
/// # Example
///
/// ```ignore
/// use melbi_types::{TypeBuilder, ArenaBuilder, Scalar};
/// use bumpalo::Bump;
///
/// let arena = Bump::new();
/// let builder = ArenaBuilder::new(&arena);
///
/// // High-level API
/// let int_ty = builder.int();
/// let arr_ty = builder.array(int_ty);
/// ```
pub trait TypeBuilder: Copy + Clone + Debug + Eq {
    /// The type handle returned by this builder.
    ///
    /// This is what users interact with - it implements `TypeView`.
    /// Could be:
    /// - `&'arena Type` (arena)
    /// - `TyHandle` wrapping `Rc<TypeData>` (RC-based)
    /// - `TypeId` (encoded)
    type TypeView: crate::TypeView<Self> + Clone + Debug + Eq + Hash;

    /// The internal interned representation of a type.
    ///
    /// This is opaque to generic code - could be:
    /// - `&'arena TyData<B>` (arena)
    /// - `Rc<TyData<B>>` (box/rc)
    /// - `TypeId` (encoded)
    type InternedTy: Clone + Debug + Eq + Hash;

    /// The interned representation of a string.
    ///
    /// Different builders can use different representations:
    /// - `&'arena str` (arena)
    /// - `Rc<str>` (RC-based)
    /// - `&'static str` (for compile-time strings)
    type InternedStr: Clone + Debug + Eq + Hash + AsRef<str> + core::fmt::Display;

    /// Interned list of types (for function parameters).
    ///
    /// Stores a sequence of types, typically `&[TypeView]` or similar.
    type InternedTypes: Clone + Debug + Eq + Hash;

    /// Interned list of field definitions (name + type pairs for records).
    ///
    /// Stores a sequence of `(InternedStr, TypeView)` tuples.
    /// Fields should be stored sorted by name for canonical representation.
    type InternedFieldTypes: Clone + Debug + Eq + Hash;

    /// Interned list of strings (for symbol parts).
    ///
    /// Stores a sequence of interned strings.
    /// Parts should be stored sorted for canonical representation.
    type InternedSymbolParts: Clone + Debug + Eq + Hash;

    // ========================================================================
    // High-level type constructors (ergonomic API)
    // ========================================================================

    /// Construct a type variable.
    fn type_var(self, id: u16) -> Self::TypeView;

    /// Construct the Int scalar type.
    fn int(self) -> Self::TypeView;

    /// Construct the Float scalar type.
    fn float(self) -> Self::TypeView;

    /// Construct the Bool scalar type.
    fn bool(self) -> Self::TypeView;

    /// Construct the Str scalar type.
    fn str(self) -> Self::TypeView;

    /// Construct the Bytes scalar type.
    fn bytes(self) -> Self::TypeView;

    /// Construct an Array type with the given element type.
    fn array(self, elem: Self::TypeView) -> Self::TypeView;

    /// Construct a Map type with the given key and value types.
    fn map(self, key: Self::TypeView, val: Self::TypeView) -> Self::TypeView;

    /// Construct a Record type with the given fields.
    ///
    /// Fields are automatically sorted by name for canonical representation.
    fn record(
        self,
        fields: impl IntoIterator<Item = (impl AsRef<str>, Self::TypeView)>,
    ) -> Self::TypeView;

    /// Construct a Function type with parameters and return type.
    fn function(
        self,
        params: impl IntoIterator<Item = Self::TypeView>,
        ret: Self::TypeView,
    ) -> Self::TypeView;

    /// Construct a Symbol type with the given parts.
    ///
    /// Parts are automatically sorted for canonical representation.
    fn symbol(self, parts: impl IntoIterator<Item = impl AsRef<str>>) -> Self::TypeView;

    // ========================================================================
    // Low-level internals (implementation details)
    // ========================================================================

    /// Intern a type kind, returning a handle.
    ///
    /// The implementation is responsible for:
    /// 1. Computing the type flags via `kind.compute_flags(self)`
    /// 2. Wrapping the kind in TyData { kind, flags }
    /// 3. Storing the TyData and returning a handle
    ///
    /// Implementations may:
    /// - Store the type in an arena and return a reference
    /// - Box the type and return an Rc
    /// - Encode the type and return an ID
    /// - Deduplicate identical types (true interning)
    fn intern_ty(self, kind: TypeKind<Self>) -> Self::InternedTy;

    /// Retrieve the type data (kind + flags) for an interned type.
    ///
    /// This is the inverse of `intern_ty` - given a handle,
    /// get back the full type structure with cached flags.
    fn ty_data(self, ty: &Self::InternedTy) -> &TyData<Self>;

    /// Intern a list of types (e.g., for function parameters).
    fn intern_types<E>(self, data: impl IntoIterator<Item = E>) -> Self::InternedTypes
    where
        E: Into<Self::TypeView>;

    /// Retrieve the interned type list.
    fn types_data(self, types: &Self::InternedTypes) -> &[Self::TypeView];

    /// Intern a list of field definitions (name + type pairs).
    ///
    /// The implementation should:
    /// - Intern all field name strings
    /// - Sort fields by name for canonical representation
    fn intern_field_types(
        self,
        data: impl IntoIterator<Item = (impl AsRef<str>, Self::TypeView)>,
    ) -> Self::InternedFieldTypes;

    /// Retrieve the interned field type list.
    fn field_types_data(
        self,
        fields: &Self::InternedFieldTypes,
    ) -> &[(Self::InternedStr, Self::TypeView)];

    /// Intern a list of symbol parts (strings).
    ///
    /// The implementation should:
    /// - Intern all strings
    /// - Sort parts for canonical representation
    fn intern_symbol_parts(
        self,
        data: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Self::InternedSymbolParts;

    /// Retrieve the interned symbol parts list.
    fn symbol_parts_data(self, parts: &Self::InternedSymbolParts) -> &[Self::InternedStr];
}
