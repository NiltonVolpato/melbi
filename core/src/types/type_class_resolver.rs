/// Type class constraint resolver.
///
/// This module is responsible for:
/// 1. Collecting constraints on type variables during type inference
/// 2. Checking whether resolved types satisfy their constraints
/// 3. Reporting constraint violations with helpful error messages
///
/// # Workflow
///
/// ```text
/// Type Inference → Constraint Collection → Unification → Constraint Resolution
///     (x + y)          (x: Numeric)         (x = Int)      (Int: Numeric? ✓)
/// ```

use crate::types::constraint_set::ConstraintSet;
use crate::types::traits::TypeView;
use crate::types::type_class::{TypeClassId, has_instance};
use crate::types::Type;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// The status of a constraint check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintStatus {
    /// The type definitely satisfies the constraint
    Satisfied,

    /// The type definitely does not satisfy the constraint
    Unsatisfied,

    /// Cannot determine yet (type still contains unresolved variables)
    Unknown,
}

/// Error type for constraint resolution failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintError {
    /// The type that failed to satisfy the constraint
    pub ty: String,

    /// The type class constraint that was not satisfied
    pub type_class: TypeClassId,

    /// Source location of the operation that required this constraint
    pub span: (usize, usize),
}

impl ConstraintError {
    /// Creates a user-friendly error message.
    pub fn message(&self) -> String {
        format!(
            "Type '{}' does not implement {}\n\
             note: {} requires {}\n\
             help: {} is implemented for: {}",
            self.ty,
            self.type_class.name(),
            self.type_class.name(),
            self.type_class.description(),
            self.type_class.name(),
            self.type_class.instances()
        )
    }
}

/// Resolves type class constraints after unification.
///
/// This is the main entry point for constraint checking. After type inference
/// and unification complete, this resolver checks whether all constrained type
/// variables have been resolved to types that satisfy their constraints.
pub struct TypeClassResolver {
    /// The set of constraints collected during type inference
    constraint_set: ConstraintSet,
}

impl TypeClassResolver {
    /// Creates a new resolver with an empty constraint set.
    pub fn new() -> Self {
        Self {
            constraint_set: ConstraintSet::new(),
        }
    }

    /// Adds a constraint to a type variable.
    ///
    /// This should be called during type inference when an operation requires
    /// a type class capability.
    ///
    /// # Arguments
    ///
    /// * `type_var` - The type variable ID
    /// * `type_class` - The required type class
    /// * `span` - Source location for error reporting
    pub fn add_constraint(
        &mut self,
        type_var: u16,
        type_class: TypeClassId,
        span: (usize, usize),
    ) {
        self.constraint_set.add(type_var, type_class, span);
    }

    /// Checks if a type satisfies a type class constraint.
    ///
    /// Returns:
    /// - `Satisfied` if the type has an instance of the type class
    /// - `Unsatisfied` if the type does not have an instance
    /// - `Unknown` if the type still contains unresolved type variables
    pub fn check_constraint<'a>(
        &self,
        ty: &'a Type<'a>,
        class: TypeClassId,
    ) -> ConstraintStatus {
        use crate::types::traits::TypeKind;

        // If type is still a variable, we can't determine yet
        if matches!(ty.view(), TypeKind::TypeVar(_)) {
            return ConstraintStatus::Unknown;
        }

        // Check if the concrete type has an instance
        if has_instance(ty, class) {
            ConstraintStatus::Satisfied
        } else {
            ConstraintStatus::Unsatisfied
        }
    }

    /// Resolves all constraints with a substitution from unification.
    ///
    /// This applies the substitution to resolve type variables, then checks
    /// whether each resolved type satisfies its constraints.
    ///
    /// # Arguments
    ///
    /// * `resolve_fn` - A function that resolves type variables to their final types
    ///
    /// # Returns
    ///
    /// * `Ok(())` if all constraints are satisfied
    /// * `Err(errors)` with all unsatisfied constraints
    pub fn resolve_all<'a, F>(
        &self,
        resolve_fn: F,
    ) -> Result<(), Vec<ConstraintError>>
    where
        F: Fn(u16) -> &'a Type<'a>,
    {
        let mut errors = Vec::new();

        for (type_var, constraints) in self.constraint_set.iter() {
            let resolved_ty = resolve_fn(type_var);

            for constraint in constraints {
                match self.check_constraint(resolved_ty, constraint.type_class) {
                    ConstraintStatus::Satisfied => {
                        // Good! Constraint is satisfied
                    }
                    ConstraintStatus::Unknown => {
                        // Still unknown after resolution - this can happen if:
                        // 1. The type is still generic (in a polymorphic function)
                        // 2. The substitution is incomplete
                        //
                        // For now, we accept it. In the future, we may want to:
                        // - Store constraints in type schemes for polymorphic functions
                        // - Apply defaulting rules (e.g., numeric literals default to Int)
                    }
                    ConstraintStatus::Unsatisfied => {
                        // Constraint violation - add to errors
                        errors.push(ConstraintError {
                            ty: format!("{}", resolved_ty),
                            type_class: constraint.type_class,
                            span: constraint.span,
                        });
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Returns a reference to the constraint set.
    pub fn constraint_set(&self) -> &ConstraintSet {
        &self.constraint_set
    }

    /// Clears all constraints.
    ///
    /// This is useful when analyzing multiple independent expressions.
    pub fn clear(&mut self) {
        self.constraint_set.clear();
    }
}

impl Default for TypeClassResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::manager::TypeManager;
    use bumpalo::Bump;

    #[test]
    fn test_constraint_status() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);
        let resolver = TypeClassResolver::new();

        // Int satisfies Numeric
        assert_eq!(
            resolver.check_constraint(tm.int(), TypeClassId::Numeric),
            ConstraintStatus::Satisfied
        );

        // Bool does not satisfy Numeric
        assert_eq!(
            resolver.check_constraint(tm.bool(), TypeClassId::Numeric),
            ConstraintStatus::Unsatisfied
        );

        // TypeVar is unknown
        let type_var = tm.fresh_type_var();
        assert_eq!(
            resolver.check_constraint(type_var, TypeClassId::Numeric),
            ConstraintStatus::Unknown
        );
    }

    #[test]
    fn test_add_and_resolve_constraints() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);
        let mut resolver = TypeClassResolver::new();

        // Add a Numeric constraint to type var 0
        resolver.add_constraint(0, TypeClassId::Numeric, (1, 10));

        // Simulate resolution where type var 0 -> Int
        let resolve_fn = |var: u16| -> &Type {
            if var == 0 {
                tm.int()
            } else {
                tm.fresh_type_var()
            }
        };

        // Should succeed because Int implements Numeric
        let result = resolver.resolve_all(resolve_fn);
        assert!(result.is_ok());
    }

    #[test]
    fn test_constraint_violation() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);
        let mut resolver = TypeClassResolver::new();

        // Add a Numeric constraint to type var 0
        resolver.add_constraint(0, TypeClassId::Numeric, (1, 10));

        // Simulate resolution where type var 0 -> Bool
        let resolve_fn = |var: u16| -> &Type {
            if var == 0 {
                tm.bool()
            } else {
                tm.fresh_type_var()
            }
        };

        // Should fail because Bool does not implement Numeric
        let result = resolver.resolve_all(resolve_fn);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].type_class, TypeClassId::Numeric);
    }

    #[test]
    fn test_multiple_constraints() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);
        let mut resolver = TypeClassResolver::new();

        // Add multiple constraints to the same type var
        resolver.add_constraint(0, TypeClassId::Hashable, (1, 5));
        resolver.add_constraint(0, TypeClassId::Ord, (2, 10));

        // Resolve to Int (which implements both Hashable and Ord)
        let resolve_fn = |var: u16| -> &Type {
            if var == 0 {
                tm.int()
            } else {
                tm.fresh_type_var()
            }
        };

        let result = resolver.resolve_all(resolve_fn);
        assert!(result.is_ok());
    }

    #[test]
    fn test_constraint_error_message() {
        let error = ConstraintError {
            ty: "Bool".to_string(),
            type_class: TypeClassId::Numeric,
            span: (1, 10),
        };

        let message = error.message();
        assert!(message.contains("Bool"));
        assert!(message.contains("Numeric"));
        assert!(message.contains("Int, Float"));
    }

    #[test]
    fn test_clear() {
        let mut resolver = TypeClassResolver::new();
        resolver.add_constraint(0, TypeClassId::Numeric, (1, 1));
        assert!(!resolver.constraint_set().is_empty());

        resolver.clear();
        assert!(resolver.constraint_set().is_empty());
    }
}
