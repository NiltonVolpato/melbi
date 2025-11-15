use crate::types::Type;
use crate::parser::Span;
use crate::types::constraint_set::{ConstraintSet, TypeClassConstraint};
use crate::types::traits::TypeView;
use crate::types::type_class::has_instance;
use crate::types::unification::Unification;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// Error type for constraint resolution failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintError {
    /// Error message
    pub message: String,

    /// Source location of the operation
    pub span: Span,
}

/// Type class constraint resolver with associated types.
///
/// Resolves relational constraints like:
/// - Indexable(container, index, result): container[index] => result
/// - Numeric(left, right, result): left + right => result
///
/// Resolution involves:
/// 1. Resolving all types in the constraint through substitution
/// 2. Looking up the type class instance for the resolved types
/// 3. Unifying associated types based on the instance
///
/// For example, `Indexable(Array[Int], _idx, _result)` should unify:
/// - _idx with Int (arrays are indexed by Int)
/// - _result with Int (the element type)
pub struct TypeClassResolver<'types> {
    /// The constraint set with relational constraints
    constraints: ConstraintSet<'types>,
}

impl<'types> TypeClassResolver<'types> {
    /// Creates a new resolver with an empty constraint set.
    pub fn new() -> Self {
        Self {
            constraints: ConstraintSet::new(),
        }
    }

    /// Adds an indexable constraint: container[index] => result
    pub fn add_indexable_constraint(
        &mut self,
        container: &'types Type<'types>,
        index: &'types Type<'types>,
        result: &'types Type<'types>,
        span: Span,
    ) {
        self.constraints.add_indexable(container, index, result, span);
    }

    /// Adds a numeric constraint: left op right => result
    pub fn add_numeric_constraint(
        &mut self,
        left: &'types Type<'types>,
        right: &'types Type<'types>,
        result: &'types Type<'types>,
        span: Span,
    ) {
        self.constraints.add_numeric(left, right, result, span);
    }

    /// Adds a hashable constraint: ty must be hashable
    pub fn add_hashable_constraint(&mut self, ty: &'types Type<'types>, span: Span) {
        self.constraints.add_hashable(ty, span);
    }

    /// Adds an ord constraint: ty must support ordering
    pub fn add_ord_constraint(&mut self, ty: &'types Type<'types>, span: Span) {
        self.constraints.add_ord(ty, span);
    }

    /// Resolves all constraints with unification.
    ///
    /// This is called after type inference is complete. It:
    /// 1. Resolves all types in constraints through substitution
    /// 2. Checks type class instances for resolved concrete types
    /// 3. Performs additional unification for associated types
    ///
    /// # Arguments
    ///
    /// * `unification` - The unification instance for resolving and unifying types
    ///
    /// # Returns
    ///
    /// * `Ok(())` if all constraints are satisfied
    /// * `Err(errors)` with all unsatisfied constraints
    pub fn resolve_all<B>(
        &self,
        unification: &mut Unification<'types, B>,
    ) -> Result<(), Vec<ConstraintError>>
    where
        B: crate::types::traits::TypeBuilder<'types, Repr = &'types Type<'types>> + 'types,
    {
        let mut errors = Vec::new();

        for constraint in self.constraints.iter() {
            if let Err(err) = self.resolve_constraint(constraint, unification) {
                errors.push(err);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Resolves a single constraint, performing unification if needed.
    fn resolve_constraint<B>(
        &self,
        constraint: &TypeClassConstraint<'types>,
        unification: &mut Unification<'types, B>,
    ) -> Result<(), ConstraintError>
    where
        B: crate::types::traits::TypeBuilder<'types, Repr = &'types Type<'types>> + 'types,
    {
        match constraint {
            TypeClassConstraint::Indexable { container, index, result, span } => {
                self.resolve_indexable(*container, *index, *result, unification, span)
            }
            TypeClassConstraint::Numeric { left, right, result, span } => {
                self.resolve_numeric(*left, *right, *result, unification, span)
            }
            TypeClassConstraint::Hashable { ty, span } => {
                self.resolve_hashable(*ty, span)
            }
            TypeClassConstraint::Ord { ty, span } => {
                self.resolve_ord(*ty, span)
            }
        }
    }

    /// Resolves an indexable constraint: container[index] => result
    ///
    /// Based on the container type, unifies index and result with the appropriate types:
    /// - Array[E]: index=Int, result=E
    /// - Map[K,V]: index=K, result=V
    /// - Bytes: index=Int, result=Int
    fn resolve_indexable<B>(
        &self,
        container: &'types Type<'types>,
        index: &'types Type<'types>,
        result: &'types Type<'types>,
        unification: &mut Unification<'types, B>,
        span: &Span,
    ) -> Result<(), ConstraintError>
    where
        B: crate::types::traits::TypeBuilder<'types, Repr = &'types Type<'types>> + 'types,
    {
        use crate::types::traits::TypeKind;

        // Resolve all types first
        let container_resolved = unification.resolve(container);
        let index_resolved = unification.resolve(index);
        let result_resolved = unification.resolve(result);

        match container_resolved.view() {
            TypeKind::Array(elem_ty) => {
                // Array[E]: index must be Int, result must be E
                let int_ty = unification.builder().int();

                // Unify index with Int
                unification.unifies_to(index_resolved, int_ty)
                    .map_err(|_| ConstraintError {
                        message: format!("Array index must be Int"),
                        span: span.clone(),
                    })?;

                // Unify result with element type
                unification.unifies_to(result_resolved, elem_ty)
                    .map_err(|_| ConstraintError {
                        message: format!("Array indexing type mismatch"),
                        span: span.clone(),
                    })?;

                Ok(())
            }
            TypeKind::Map(key_ty, value_ty) => {
                // Map[K,V]: index must be K, result must be V

                // Unify index with key type
                unification.unifies_to(index_resolved, key_ty)
                    .map_err(|_| ConstraintError {
                        message: format!("Map index type mismatch"),
                        span: span.clone(),
                    })?;

                // Unify result with value type
                unification.unifies_to(result_resolved, value_ty)
                    .map_err(|_| ConstraintError {
                        message: format!("Map indexing type mismatch"),
                        span: span.clone(),
                    })?;

                Ok(())
            }
            TypeKind::Bytes => {
                // Bytes: index must be Int, result must be Int
                let int_ty = unification.builder().int();

                unification.unifies_to(index_resolved, int_ty)
                    .map_err(|_| ConstraintError {
                        message: format!("Bytes index must be Int"),
                        span: span.clone(),
                    })?;

                unification.unifies_to(result_resolved, int_ty)
                    .map_err(|_| ConstraintError {
                        message: format!("Bytes indexing must produce Int"),
                        span: span.clone(),
                    })?;

                Ok(())
            }
            TypeKind::TypeVar(_) => {
                // Still unresolved - this is OK, constraint will be checked later
                // This can happen in polymorphic contexts
                Ok(())
            }
            _ => {
                Err(ConstraintError {
                    message: format!("Type is not indexable"),
                    span: span.clone(),
                })
            }
        }
    }

    /// Resolves a numeric constraint: left op right => result
    ///
    /// All three types must be the same numeric type (Int or Float).
    fn resolve_numeric<B>(
        &self,
        left: &'types Type<'types>,
        right: &'types Type<'types>,
        result: &'types Type<'types>,
        unification: &mut Unification<'types, B>,
        span: &Span,
    ) -> Result<(), ConstraintError>
    where
        B: crate::types::traits::TypeBuilder<'types, Repr = &'types Type<'types>> + 'types,
    {
        use crate::types::traits::TypeKind;

        // Resolve all types
        let left_resolved = unification.resolve(left);
        let right_resolved = unification.resolve(right);
        let result_resolved = unification.resolve(result);

        // Unify left with right
        unification.unifies_to(left_resolved, right_resolved)
            .map_err(|_| ConstraintError {
                message: format!("Numeric operands must have the same type"),
                span: span.clone(),
            })?;

        // Unify result with left (which is now unified with right)
        let unified_operand = unification.resolve(left_resolved);
        unification.unifies_to(result_resolved, unified_operand)
            .map_err(|_| ConstraintError {
                message: format!("Numeric result must match operand type"),
                span: span.clone(),
            })?;

        // Check that the final type is numeric (if resolved to concrete type)
        let final_ty = unification.resolve(unified_operand);
        match final_ty.view() {
            TypeKind::Int | TypeKind::Float => Ok(()),
            TypeKind::TypeVar(_) => Ok(()), // Still polymorphic, OK
            _ => Err(ConstraintError {
                message: format!("Numeric operations require Int or Float"),
                span: span.clone(),
            }),
        }
    }

    /// Resolves a hashable constraint: ty must be hashable
    fn resolve_hashable(
        &self,
        ty: &'types Type<'types>,
        span: &Span,
    ) -> Result<(), ConstraintError> {
        use crate::types::type_class::TypeClassId;

        if has_instance(ty, TypeClassId::Hashable) {
            Ok(())
        } else {
            Err(ConstraintError {
                message: format!("Type {} does not implement Hashable", ty),
                span: span.clone(),
            })
        }
    }

    /// Resolves an ord constraint: ty must support ordering
    fn resolve_ord(
        &self,
        ty: &'types Type<'types>,
        span: &Span,
    ) -> Result<(), ConstraintError> {
        use crate::types::type_class::TypeClassId;

        if has_instance(ty, TypeClassId::Ord) {
            Ok(())
        } else {
            Err(ConstraintError {
                message: format!("Type {} does not implement Ord", ty),
                span: span.clone(),
            })
        }
    }

    /// Returns a reference to the constraint set.
    pub fn constraint_set(&self) -> &ConstraintSet<'types> {
        &self.constraints
    }

    /// Clears all constraints.
    pub fn clear(&mut self) {
        self.constraints.clear();
    }
}

impl<'types> Default for TypeClassResolver<'types> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::manager::TypeManager;
    use crate::types::unification::Unification;
    use bumpalo::Bump;

    #[test]
    fn test_indexable_array() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);
        let mut resolver = TypeClassResolver::new();
        let mut unify = Unification::new(tm);

        // Constraint: Array[Int][Int] => Int
        let arr = tm.array(tm.int());
        let result = tm.fresh_type_var();

        resolver.add_indexable_constraint(arr, tm.int(), result, Span(0..1));

        // Should resolve successfully and unify result with Int
        assert!(resolver.resolve_all(&mut unify).is_ok());
        assert_eq!(format!("{}", unify.resolve(result)), "Int");
    }

    #[test]
    fn test_indexable_map() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);
        let mut resolver = TypeClassResolver::new();
        let mut unify = Unification::new(tm);

        // Constraint: Map[Int, Str][Int] => Str
        let map = tm.map(tm.int(), tm.str());
        let result = tm.fresh_type_var();

        resolver.add_indexable_constraint(map, tm.int(), result, Span(0..1));

        // Should resolve successfully and unify result with Str
        assert!(resolver.resolve_all(&mut unify).is_ok());
        assert_eq!(format!("{}", unify.resolve(result)), "Str");
    }

    #[test]
    fn test_numeric_constraint() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);
        let mut resolver = TypeClassResolver::new();
        let mut unify = Unification::new(tm);

        // Constraint: Int + Int => Int
        let result = tm.fresh_type_var();

        resolver.add_numeric_constraint(tm.int(), tm.int(), result, Span(0..1));

        // Should resolve successfully and unify result with Int
        assert!(resolver.resolve_all(&mut unify).is_ok());
        assert_eq!(format!("{}", unify.resolve(result)), "Int");
    }
}
