use alloc::string::ToString;
use core::marker::PhantomData;

use hashbrown::{HashMap, HashSet};

use crate::{
    String, Vec,
    types::{
        TypeScheme,
        manager::TypeManager,
        traits::{TypeBuilder, TypeKind, TypeTransformer, TypeView, display_type},
    },
};

/// Types of unification errors.
#[derive(Debug)]
pub enum Error {
    OccursCheckFailed { type_var: String, ty: String },
    FieldCountMismatch { expected: usize, found: usize },
    FieldNameMismatch { expected: String, found: String },
    FunctionParamCountMismatch { expected: usize, found: usize },
    TypeMismatch { left: String, right: String },
}

/// Generic unification for types.
///
/// Performs Hindley-Milner style unification over any `TypeView` representation,
/// building unified types using the provided `TypeBuilder`.
///
/// # Example
///
/// ```ignore
/// use crate::types::{TypeManager, unification::Unification};
/// use bumpalo::Bump;
///
/// let bump = Bump::new();
/// let manager = TypeManager::new(&bump);
/// let mut unify = Unification::new(manager);
///
/// let t1 = manager.array(manager.type_var(0));
/// let t2 = manager.array(manager.int());
///
/// let result = unify.unifies_to(t1, t2)?;
/// // Result: Array[Int], with substitution {0 -> Int}
/// ```
pub struct Unification<'a, B: TypeBuilder<'a>> {
    builder: B,
    subst: HashMap<u16, B::Repr>,
    _phantom: PhantomData<&'a ()>,
}

impl<'a, B: TypeBuilder<'a>> Unification<'a, B>
where
    B::Repr: TypeView<'a>,
{
    /// Create a new unification instance with the given type constructor.
    pub fn new(builder: B) -> Self {
        Self {
            builder,
            subst: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    /// Resolve a type by following the substitution chain.
    ///
    /// Iteratively resolves type variables until a non-variable type is found
    /// or a variable with no substitution is reached.
    pub fn resolve(&self, mut ty: B::Repr) -> B::Repr {
        // TODO: add path compression
        loop {
            if let TypeKind::TypeVar(id) = ty.view() {
                if let Some(&t) = self.subst.get(&id) {
                    ty = t;
                    continue;
                }
            }
            break;
        }
        ty
    }

    /// Check if type variable tv occurs in type t.
    ///
    /// Prevents creating infinite types like `a = Array[a]`.
    fn occurs_in(&self, id: u16, t: B::Repr) -> bool {
        use TypeKind::*;

        let resolved = self.resolve(t).view();

        if let TypeVar(resolved_id) = resolved {
            if resolved_id == id {
                return true;
            }
        }

        // Recursively check for occurrence in composite types
        match resolved {
            Array(e) => self.occurs_in(id, e),
            Map(k, v) => self.occurs_in(id, k) || self.occurs_in(id, v),
            Record(mut fields) => fields.any(|(_, field_ty)| self.occurs_in(id, field_ty)),
            Function { mut params, ret } => {
                params.any(|p| self.occurs_in(id, p)) || self.occurs_in(id, ret)
            }
            Symbol(_) | Int | Float | Bool | Str | Bytes | TypeVar(_) => false,
        }
    }

    /// Unify two types, returning the unified type or an error.
    ///
    /// This implements Hindley-Milner unification:
    /// - Type variables unify with any type (occurs check prevents infinite types)
    /// - Primitives unify only with identical primitives
    /// - Composite types unify recursively
    ///
    /// The substitution map is updated with any new type variable bindings.
    pub fn unifies_to(&mut self, t1: B::Repr, t2: B::Repr) -> Result<B::Repr, Error> {
        let t1 = self.resolve(t1);
        let t2 = self.resolve(t2);

        // Fast path: equality (works via TypeView: Eq bound)
        if t1 == t2 {
            return Ok(t1);
        }

        use Error::*;
        use TypeKind::*;

        match (t1.view(), t2.view()) {
            // Type variable cases - bind variable to the other type
            (TypeVar(id), _) => {
                if self.occurs_in(id, t2) {
                    return Err(OccursCheckFailed {
                        type_var: display_type(t1),
                        ty: display_type(t2),
                    });
                }
                self.subst.insert(id, t2);
                Ok(t2)
            }
            (_, TypeVar(id)) => {
                if self.occurs_in(id, t1) {
                    return Err(OccursCheckFailed {
                        type_var: display_type(t2),
                        ty: display_type(t1),
                    });
                }
                self.subst.insert(id, t1);
                Ok(t1)
            }

            // Primitives - must match exactly
            (Int, Int) | (Float, Float) | (Bool, Bool) | (Str, Str) | (Bytes, Bytes) => Ok(t1),

            // Array - unify element types
            (Array(e1), Array(e2)) => {
                let elem = self.unifies_to(e1, e2)?;
                Ok(self.builder.array(elem))
            }

            // Map - unify key and value types
            (Map(k1, v1), Map(k2, v2)) => {
                let k = self.unifies_to(k1, k2)?;
                let v = self.unifies_to(v1, v2)?;
                Ok(self.builder.map(k, v))
            }

            // Record - unify field by field
            (Record(fields1), Record(fields2)) => {
                // Collect fields into vectors to check length
                let f1: Vec<_> = fields1.collect();
                let f2: Vec<_> = fields2.collect();

                if f1.len() != f2.len() {
                    return Err(FieldCountMismatch {
                        expected: f1.len(),
                        found: f2.len(),
                    });
                }

                let mut unified_fields = Vec::with_capacity(f1.len());
                for ((n1, t1), (n2, t2)) in f1.iter().zip(f2.iter()) {
                    if n1 != n2 {
                        return Err(FieldNameMismatch {
                            expected: n1.to_string(),
                            found: n2.to_string(),
                        });
                    }
                    let u = self.unifies_to(*t1, *t2)?;
                    unified_fields.push((*n1, u));
                }
                Ok(self.builder.record(unified_fields.iter().copied()))
            }

            // Function - unify parameters and return type
            (
                Function {
                    params: p1,
                    ret: r1,
                },
                Function {
                    params: p2,
                    ret: r2,
                },
            ) => {
                // Collect params to check length
                let params1: Vec<_> = p1.collect();
                let params2: Vec<_> = p2.collect();

                if params1.len() != params2.len() {
                    return Err(FunctionParamCountMismatch {
                        expected: params1.len(),
                        found: params2.len(),
                    });
                }

                let mut unified_params = Vec::with_capacity(params1.len());
                for (a, b) in params1.iter().zip(params2.iter()) {
                    let u = self.unifies_to(*a, *b)?;
                    unified_params.push(u);
                }

                let r = self.unifies_to(r1, r2)?;
                Ok(self.builder.function(unified_params.iter().copied(), r))
            }

            // Symbol - must have identical parts
            (Symbol(parts1), Symbol(parts2)) => {
                let p1: Vec<_> = parts1.collect();
                let p2: Vec<_> = parts2.collect();
                if p1 == p2 {
                    Ok(t1)
                } else {
                    Err(TypeMismatch {
                        left: display_type(t1),
                        right: display_type(t2),
                    })
                }
            }

            // Mismatch - types don't unify
            _ => Err(TypeMismatch {
                left: display_type(t1),
                right: display_type(t2),
            }),
        }
    }

    /// Collect all free type variables in a type (resolution-aware).
    ///
    /// Returns the set of type variable IDs that appear in the type after resolving
    /// through the current unification substitutions.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let free_vars = unify.free_type_vars(some_type);
    /// // free_vars contains all unresolved type variable IDs
    /// ```
    pub fn free_type_vars(&self, ty: B::Repr) -> HashSet<u16> {
        let mut vars = HashSet::new();
        self.collect_free_vars(ty, &mut vars);
        vars
    }

    /// Helper to recursively collect free variables
    fn collect_free_vars(&self, ty: B::Repr, vars: &mut HashSet<u16>) {
        use TypeKind::*;

        let resolved = self.resolve(ty);
        match resolved.view() {
            TypeVar(id) => {
                vars.insert(id);
            }
            Array(elem) => {
                self.collect_free_vars(elem, vars);
            }
            Map(key, val) => {
                self.collect_free_vars(key, vars);
                self.collect_free_vars(val, vars);
            }
            Record(fields) => {
                for (_, field_ty) in fields {
                    self.collect_free_vars(field_ty, vars);
                }
            }
            Function { params, ret } => {
                for param in params {
                    self.collect_free_vars(param, vars);
                }
                self.collect_free_vars(ret, vars);
            }
            Int | Float | Bool | Str | Bytes | Symbol(_) => {
                // No type variables in these
            }
        }
    }

    /// Apply a substitution to a type, replacing type variables according to the given map.
    ///
    /// This resolves types through unification substitutions at each step, then applies
    /// the provided instantiation substitution. Resolution happens recursively to handle
    /// nested substitutions correctly.
    ///
    /// Uses `TypeTransformer` to recursively walk and rebuild the type structure.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut inst_subst = HashMap::new();
    /// inst_subst.insert(0, fresh_var_0);
    /// inst_subst.insert(1, fresh_var_1);
    /// let instantiated = unify.substitute(some_type, &inst_subst);
    /// ```
    fn substitute(&self, ty: B::Repr, inst_subst: &HashMap<u16, B::Repr>) -> B::Repr
    where
        B: Copy,
    {
        // Helper struct that implements TypeTransformer for substitution
        struct Substitutor<'a, 'b, B: TypeBuilder<'a>> {
            unification: &'b Unification<'a, B>,
            inst_subst: &'b HashMap<u16, B::Repr>,
        }

        impl<'a, 'b, B: TypeBuilder<'a>> TypeTransformer<'a, B> for Substitutor<'a, 'b, B>
        where
            B::Repr: TypeView<'a>,
            B: Copy,
        {
            type Input = B::Repr;

            fn builder(&self) -> &B {
                &self.unification.builder
            }

            fn transform(&self, ty: Self::Input) -> B::Repr {
                // CRITICAL: Resolve at each step to handle nested substitutions
                // For example, if ty = Array[_0] and unification has {0: Array[_1]}
                // and inst_subst has {1: _50}, we need to:
                // 1. Resolve Array[_0] -> Array[Array[_1]]
                // 2. Recursively transform Array[_1], which will resolve _1 and substitute it
                let resolved = self.unification.resolve(ty);

                match resolved.view() {
                    TypeKind::TypeVar(id) => {
                        // Check instantiation substitution
                        if let Some(&subst_ty) = self.inst_subst.get(&id) {
                            subst_ty
                        } else {
                            // Not in substitution map, keep as-is
                            resolved
                        }
                    }
                    // All other cases handled by default recursive implementation
                    _ => self.transform_default(resolved),
                }
            }
        }

        let substitutor = Substitutor {
            unification: self,
            inst_subst,
        };
        substitutor.transform(ty)
    }
}

// Additional methods specific to TypeManager
impl<'a> Unification<'a, &'a TypeManager<'a>> {

    /// Generalize a type into a type scheme by quantifying free variables.
    ///
    /// Creates a type scheme by quantifying all type variables that are free in the type
    /// but not free in the environment (env_vars). This implements the generalization
    /// step of Algorithm W.
    ///
    /// # Arguments
    ///
    /// * `ty` - The type to generalize
    /// * `env_vars` - Type variables that are free in the environment (should not be quantified)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let env_vars = HashSet::new(); // Empty environment
    /// let scheme = unify.generalize(identity_fn_type, &env_vars);
    /// // scheme is ∀a. a → a
    /// ```
    pub fn generalize(
        &self,
        ty: &'a crate::types::Type<'a>,
        env_vars: &HashSet<u16>,
    ) -> TypeScheme<'a> {
        // Get all free variables in the type
        let type_vars = self.free_type_vars(ty);

        // Remove environment variables to get variables to quantify
        let to_quantify: Vec<u16> = type_vars.difference(env_vars).copied().collect();

        // Sort for deterministic output
        let mut sorted_vars = to_quantify;
        sorted_vars.sort_unstable();

        // Allocate in the arena
        let quantified = self.builder.alloc_u16_slice(&sorted_vars);

        TypeScheme::new(quantified, ty)
    }

    /// Instantiate a type scheme with fresh type variables.
    ///
    /// Creates a fresh instance of a polymorphic type by replacing each quantified
    /// variable with a fresh type variable. This implements the instantiation step
    /// of Algorithm W.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let id_scheme = TypeScheme::new(&[0], identity_type); // ∀a. a → a
    /// let instance1 = unify.instantiate(&id_scheme); // TypeVar(42) → TypeVar(42)
    /// let instance2 = unify.instantiate(&id_scheme); // TypeVar(43) → TypeVar(43)
    /// // Each instantiation gets fresh variables
    /// ```
    pub fn instantiate(&self, scheme: &TypeScheme<'a>) -> &'a crate::types::Type<'a> {
        if scheme.is_monomorphic() {
            // No quantified variables, return type as-is
            return scheme.ty;
        }

        // Create fresh type variables for each quantified variable
        let mut inst_subst = HashMap::new();
        for &var_id in scheme.quantified {
            let fresh = self.builder.fresh_type_var();
            inst_subst.insert(var_id, fresh);
        }

        // Apply substitution to the type
        self.substitute(scheme.ty, &inst_subst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::manager::TypeManager;

    #[test]
    fn test_unifies_to_success() {
        let arena = bumpalo::Bump::new();
        let type_manager = TypeManager::new(&arena);
        let mut unify = Unification::new(type_manager);

        // Example: unify Map[Int, String] with Map[Int, String] (should succeed)
        let int_ty = type_manager.int();
        let str_ty = type_manager.str();
        let map_ty1 = type_manager.map(int_ty, str_ty);
        let map_ty2 = type_manager.map(int_ty, str_ty);

        let result = unify.unifies_to(map_ty1, map_ty2);
        assert!(result.is_ok(), "Expected types to unify");
    }

    #[test]
    fn test_unifies_to_failure() {
        let arena = bumpalo::Bump::new();
        let type_manager = TypeManager::new(&arena);
        let mut unify = Unification::new(type_manager);

        // Example: unify Map[Int, Int] with Map[Int, String] (should fail)
        let int_ty = type_manager.int();
        let str_ty = type_manager.str();
        let map_ty1 = type_manager.map(int_ty, int_ty);
        let map_ty2 = type_manager.map(int_ty, str_ty);

        let result = unify.unifies_to(map_ty1, map_ty2);
        assert!(result.is_err(), "Expected types not to unify");

        if let Err(err) = result {
            // Print error for debugging
            println!("Type error: {err:#?}");
        }
    }

    #[test]
    fn test_free_type_vars_empty() {
        let arena = bumpalo::Bump::new();
        let type_manager = TypeManager::new(&arena);
        let unify = Unification::new(type_manager);

        let int_ty = type_manager.int();
        let vars = unify.free_type_vars(int_ty);

        assert!(vars.is_empty(), "Int should have no free type variables");
    }

    #[test]
    fn test_free_type_vars_single() {
        let arena = bumpalo::Bump::new();
        let type_manager = TypeManager::new(&arena);
        let unify = Unification::new(type_manager);

        let var_ty = type_manager.type_var(42);
        let vars = unify.free_type_vars(var_ty);

        assert_eq!(vars.len(), 1);
        assert!(vars.contains(&42));
    }

    #[test]
    fn test_free_type_vars_function() {
        let arena = bumpalo::Bump::new();
        let type_manager = TypeManager::new(&arena);
        let unify = Unification::new(type_manager);

        // (TypeVar(0), TypeVar(1)) -> TypeVar(0)
        let var0 = type_manager.type_var(0);
        let var1 = type_manager.type_var(1);
        let func = type_manager.function(&[var0, var1], var0);

        let vars = unify.free_type_vars(func);

        assert_eq!(vars.len(), 2);
        assert!(vars.contains(&0));
        assert!(vars.contains(&1));
    }

    #[test]
    fn test_free_type_vars_after_unification() {
        let arena = bumpalo::Bump::new();
        let type_manager = TypeManager::new(&arena);
        let mut unify = Unification::new(type_manager);

        let var0 = type_manager.type_var(0);
        let int_ty = type_manager.int();

        // Unify TypeVar(0) = Int
        let _ = unify.unifies_to(var0, int_ty);

        // Now TypeVar(0) should resolve to Int, so free_vars should be empty
        let vars = unify.free_type_vars(var0);

        assert!(vars.is_empty(), "TypeVar(0) resolved to Int, no free vars");
    }

    #[test]
    fn test_generalize_monomorphic() {
        let arena = bumpalo::Bump::new();
        let type_manager = TypeManager::new(&arena);
        let unify = Unification::new(type_manager);

        let int_ty = type_manager.int();
        let env_vars = HashSet::new();

        let scheme = unify.generalize(int_ty, &env_vars);

        assert!(scheme.is_monomorphic());
        assert!(core::ptr::eq(scheme.ty, int_ty));
    }

    #[test]
    fn test_generalize_polymorphic() {
        let arena = bumpalo::Bump::new();
        let type_manager = TypeManager::new(&arena);
        let unify = Unification::new(type_manager);

        // Identity function: TypeVar(0) -> TypeVar(0)
        let var0 = type_manager.type_var(0);
        let func = type_manager.function(&[var0], var0);

        let env_vars = HashSet::new();
        let scheme = unify.generalize(func, &env_vars);

        assert!(!scheme.is_monomorphic());
        assert_eq!(scheme.quantified.len(), 1);
        assert_eq!(scheme.quantified[0], 0);
        assert!(core::ptr::eq(scheme.ty, func));
    }

    #[test]
    fn test_generalize_with_env_vars() {
        let arena = bumpalo::Bump::new();
        let type_manager = TypeManager::new(&arena);
        let unify = Unification::new(type_manager);

        // Function: TypeVar(0) -> TypeVar(1)
        let var0 = type_manager.type_var(0);
        let var1 = type_manager.type_var(1);
        let func = type_manager.function(&[var0], var1);

        // TypeVar(0) is in environment, so only TypeVar(1) should be quantified
        let mut env_vars = HashSet::new();
        env_vars.insert(0);

        let scheme = unify.generalize(func, &env_vars);

        assert!(!scheme.is_monomorphic());
        assert_eq!(scheme.quantified.len(), 1);
        assert_eq!(scheme.quantified[0], 1);
    }

    #[test]
    fn test_instantiate_monomorphic() {
        let arena = bumpalo::Bump::new();
        let type_manager = TypeManager::new(&arena);
        let unify = Unification::new(type_manager);

        let int_ty = type_manager.int();
        let scheme = TypeScheme::new(&[], int_ty);

        let instance = unify.instantiate(&scheme);

        assert!(core::ptr::eq(instance, int_ty));
    }

    #[test]
    fn test_instantiate_polymorphic() {
        let arena = bumpalo::Bump::new();
        let type_manager = TypeManager::new(&arena);
        let unify = Unification::new(type_manager);

        // Scheme: ∀a. a -> a
        let var0 = type_manager.type_var(0);
        let func = type_manager.function(&[var0], var0);
        let quantified = arena.alloc_slice_copy(&[0u16]);
        let scheme = TypeScheme::new(quantified, func);

        let instance1 = unify.instantiate(&scheme);
        let instance2 = unify.instantiate(&scheme);

        // Both instances should be function types
        assert!(matches!(instance1.view(), TypeKind::Function { .. }));
        assert!(matches!(instance2.view(), TypeKind::Function { .. }));

        // But they should have different fresh type variables
        // (We can't easily test this without inspecting the types,
        // but we can at least verify they're function types)
    }

    #[test]
    fn test_instantiate_creates_fresh_vars() {
        let arena = bumpalo::Bump::new();
        let type_manager = TypeManager::new(&arena);
        let mut unify = Unification::new(type_manager);

        // Scheme: ∀a. a -> a
        let var0 = type_manager.type_var(0);
        let func = type_manager.function(&[var0], var0);
        let quantified = arena.alloc_slice_copy(&[0u16]);
        let scheme = TypeScheme::new(quantified, func);

        // Instantiate twice
        let instance1 = unify.instantiate(&scheme);
        let instance2 = unify.instantiate(&scheme);

        // Now unify instance1's param with Int
        let int_ty = type_manager.int();
        if let TypeKind::Function { mut params, .. } = instance1.view() {
            let param1 = params.next().unwrap();
            let _ = unify.unifies_to(param1, int_ty);
        }

        // instance1 should now be Int -> Int when resolved
        // instance2 should still be TypeVar(?) -> TypeVar(?)
        // We verify by checking that instance2's param is still a type variable
        if let TypeKind::Function { mut params, .. } = instance2.view() {
            let param2 = params.next().unwrap();
            let resolved = unify.resolve(param2);
            assert!(matches!(resolved.view(), TypeKind::TypeVar(_)));
        }
    }

    #[test]
    fn test_substitute_with_nested_unification() {
        let arena = bumpalo::Bump::new();
        let type_manager = TypeManager::new(&arena);
        let mut unify = Unification::new(type_manager);

        // Create types:
        // _0, _1, _50
        let var0 = type_manager.type_var(0);
        let var1 = type_manager.type_var(1);
        let var50 = type_manager.type_var(50);

        // Create: Array[_0]
        let array_var0 = type_manager.array(var0);

        // Unify _0 = Array[_1] in the unification context
        // So unify.subst now has: {0: Array[_1]}
        let array_var1 = type_manager.array(var1);
        let _ = unify.unifies_to(var0, array_var1);

        // Now call substitute with:
        // ty: Array[_0]  (which resolves to Array[Array[_1]])
        // inst_subst: {1: _50}
        //
        // Expected result: Array[Array[_50]]
        // The bug would give us: Array[Array[_1]] (missing the substitution of _1)

        let mut inst_subst = HashMap::new();
        inst_subst.insert(1, var50);

        let result = unify.substitute(array_var0, &inst_subst);

        // Verify the result is Array[Array[_50]]
        if let crate::types::Type::Array(inner) = result {
            if let crate::types::Type::Array(innermost) = inner {
                if let crate::types::Type::TypeVar(id) = innermost {
                    assert_eq!(
                        *id, 50,
                        "Expected innermost type var to be _50, got _{}",
                        id
                    );
                } else {
                    panic!("Expected TypeVar(_50) as innermost type, got {:?}", innermost);
                }
            } else {
                panic!("Expected Array as inner type, got {:?}", inner);
            }
        } else {
            panic!("Expected Array as outer type, got {:?}", result);
        }
    }
}
