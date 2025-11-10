use super::Type;

/// A type scheme representing a polymorphic type with universally quantified type variables.
///
/// For example, the identity function has type scheme `∀a. a → a`, which is represented as:
/// ```
/// TypeScheme {
///     quantified: &[0],  // Type variable 'a' with ID 0
///     ty: Function { params: [TypeVar(0)], ret: TypeVar(0) }
/// }
/// ```
///
/// Monomorphic types are represented with an empty quantified list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeScheme<'a> {
    /// List of quantified type variable IDs (e.g., [0, 1] for ∀a,b. ...)
    /// Should be sorted and deduplicated.
    pub quantified: &'a [u16],

    /// The type containing the quantified variables
    pub ty: &'a Type<'a>,
}

impl<'a> TypeScheme<'a> {
    /// Create a new type scheme.
    pub fn new(quantified: &'a [u16], ty: &'a Type<'a>) -> Self {
        TypeScheme { quantified, ty }
    }

    /// Returns true if this is a monomorphic type (no quantified variables).
    pub fn is_monomorphic(&self) -> bool {
        self.quantified.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::manager::TypeManager;
    use bumpalo::Bump;

    #[test]
    fn test_monomorphic_scheme() {
        let bump = Bump::new();
        let mgr = TypeManager::new(&bump);

        let int_ty = mgr.int();
        let scheme = TypeScheme::new(&[], int_ty);

        assert!(scheme.is_monomorphic());
        assert_eq!(scheme.quantified.len(), 0);
        assert!(core::ptr::eq(scheme.ty, int_ty));
    }

    #[test]
    fn test_polymorphic_scheme() {
        let bump = Bump::new();
        let mgr = TypeManager::new(&bump);

        let type_var = mgr.type_var(0);
        let quantified = bump.alloc_slice_copy(&[0u16]);
        let scheme = TypeScheme::new(quantified, type_var);

        assert!(!scheme.is_monomorphic());
        assert_eq!(scheme.quantified.len(), 1);
        assert_eq!(scheme.quantified[0], 0);
    }

    #[test]
    fn test_polymorphic_function_scheme() {
        let bump = Bump::new();
        let mgr = TypeManager::new(&bump);

        // ∀a. a → a (identity function)
        let type_var = mgr.type_var(0);
        let func_ty = mgr.function(&[type_var], type_var);
        let quantified = bump.alloc_slice_copy(&[0u16]);
        let scheme = TypeScheme::new(quantified, func_ty);

        assert!(!scheme.is_monomorphic());
        assert_eq!(scheme.quantified, &[0]);
    }
}
