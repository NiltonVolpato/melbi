use crate::ir::{TyData, TypeBuilder, TypeKind};
use alloc::vec::Vec;
use bumpalo::Bump;

/// Interner that uses arena allocation.
///
/// Types are allocated in a `Bump` arena. For now, we don't do actual
/// interning (deduplication), just allocation. This keeps the implementation
/// simple for the POC.
///
/// Following Chalk's design, we compute type flags during interning and
/// wrap the TyKind in TyData.
///
/// # Example
///
/// ```
/// use melbi_types::{TypeBuilder, ArenaBuilder, Scalar, TypeKind};
/// use bumpalo::Bump;
///
/// let arena = Bump::new();
/// let builder = ArenaBuilder::new(&arena);
///
/// let int_ty = TypeKind::Scalar(Scalar::Int).intern(builder);
/// let arr_ty = TypeKind::Array(int_ty).intern(builder);
/// ```
#[derive(Copy, Clone, Debug)]
pub struct ArenaBuilder<'arena> {
    arena: &'arena Bump,
}

// Manual implementations since Bump doesn't implement PartialEq/Eq/Hash
// We use pointer equality - two builders are equal if they point to the same arena
impl<'arena> PartialEq for ArenaBuilder<'arena> {
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self.arena, other.arena)
    }
}

impl<'arena> Eq for ArenaBuilder<'arena> {}

impl<'arena> core::hash::Hash for ArenaBuilder<'arena> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        core::ptr::hash(self.arena, state)
    }
}

impl<'arena> ArenaBuilder<'arena> {
    /// Create a new arena builder.
    pub fn new(arena: &'arena Bump) -> Self {
        Self { arena }
    }
}

impl<'arena> TypeBuilder for ArenaBuilder<'arena> {
    type TypeView = crate::Ty<Self>;
    type InternedTy = &'arena TyData<Self>;
    type InternedStr = &'arena str;
    type InternedTypes = &'arena [crate::Ty<Self>];
    type InternedFieldTypes = &'arena [(&'arena str, crate::Ty<Self>)];
    type InternedSymbolParts = &'arena [&'arena str];

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
        // For now, just allocate - no deduplication
        // We can add a HashMap for interning later if needed
        self.arena.alloc(TyData { kind, flags })
    }

    fn ty_data(self, ty: &Self::InternedTy) -> &TyData<Self> {
        ty
    }

    fn intern_types<E>(self, data: impl IntoIterator<Item = E>) -> Self::InternedTypes
    where
        E: Into<crate::Ty<Self>>,
    {
        let types: Vec<_> = data.into_iter().map(|e| e.into()).collect();
        self.arena.alloc_slice_copy(&types)
    }

    fn types_data(self, types: &Self::InternedTypes) -> &[crate::Ty<Self>] {
        types
    }

    fn intern_field_types(
        self,
        data: impl IntoIterator<Item = (impl AsRef<str>, crate::Ty<Self>)>,
    ) -> Self::InternedFieldTypes {
        // Intern all field name strings in the arena
        let mut interned_fields: Vec<(&'arena str, crate::Ty<Self>)> = data
            .into_iter()
            .map(|(name, ty)| {
                let interned_name: &'arena str = self.arena.alloc_str(name.as_ref());
                (interned_name, ty)
            })
            .collect();

        // Sort by field name for canonical representation
        interned_fields.sort_by(|(a, _), (b, _)| a.cmp(b));

        self.arena
            .alloc_slice_fill_iter(interned_fields.into_iter())
    }

    fn field_types_data(
        self,
        fields: &Self::InternedFieldTypes,
    ) -> &[(Self::InternedStr, crate::Ty<Self>)] {
        fields
    }

    fn intern_symbol_parts(
        self,
        data: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Self::InternedSymbolParts {
        // Intern all strings in the arena
        let mut interned_parts: Vec<&'arena str> = data
            .into_iter()
            .map(|part| {
                let s: &'arena str = self.arena.alloc_str(part.as_ref());
                s
            })
            .collect();

        // Sort for canonical representation
        interned_parts.sort();

        self.arena.alloc_slice_copy(&interned_parts)
    }

    fn symbol_parts_data(self, parts: &Self::InternedSymbolParts) -> &[Self::InternedStr] {
        parts
    }
}
