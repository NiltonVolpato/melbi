/// Constraint set for tracking type class requirements on type variables.
///
/// During type inference, operations add constraints to type variables:
///   - Binary operations (+, -, etc.) add Numeric constraints
///   - Indexing operations add Indexable constraints
///   - Map key types add Hashable constraints
///
/// After unification resolves type variables to concrete types, the constraint
/// set is checked to ensure all constraints are satisfied.
use crate::types::type_class::TypeClassId;
use crate::parser::Span;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// A constraint requiring a type variable to implement a type class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Constraint {
    /// The type class that must be implemented
    pub type_class: TypeClassId,

    /// Source location for error reporting
    pub span: Span,
}

impl Constraint {
    /// Creates a new constraint.
    pub fn new(type_class: TypeClassId, span: Span) -> Self {
        Self { type_class, span }
    }
}

/// A set of constraints on type variables.
///
/// Maps type variable IDs to the list of type class constraints they must satisfy.
/// Multiple constraints can be added to the same type variable (e.g., a type might
/// need to be both Hashable and Ord).
#[derive(Debug, Clone)]
pub struct ConstraintSet {
    /// Maps type variable ID -> list of constraints
    /// Using BTreeMap for deterministic ordering (helps with testing/debugging)
    constraints: BTreeMap<u16, Vec<Constraint>>,
}

impl ConstraintSet {
    /// Creates a new empty constraint set.
    pub fn new() -> Self {
        Self {
            constraints: BTreeMap::new(),
        }
    }

    /// Adds a constraint to a type variable.
    ///
    /// # Arguments
    ///
    /// * `type_var` - The type variable ID
    /// * `type_class` - The type class constraint to add
    /// * `span` - Source location for error reporting
    pub fn add(&mut self, type_var: u16, type_class: TypeClassId, span: Span) {
        let constraint = Constraint::new(type_class, span);

        self.constraints
            .entry(type_var)
            .or_insert_with(Vec::new)
            .push(constraint);
    }

    /// Gets all constraints for a type variable.
    ///
    /// Returns an empty slice if no constraints exist for this variable.
    pub fn get(&self, type_var: u16) -> &[Constraint] {
        self.constraints
            .get(&type_var)
            .map_or(&[], |v| v.as_slice())
    }

    /// Returns an iterator over all (type_var, constraints) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (u16, &[Constraint])> {
        self.constraints
            .iter()
            .map(|(var, constraints)| (*var, constraints.as_slice()))
    }

    /// Returns true if there are no constraints.
    pub fn is_empty(&self) -> bool {
        self.constraints.is_empty()
    }

    /// Returns the total number of type variables with constraints.
    pub fn len(&self) -> usize {
        self.constraints.len()
    }

    /// Clears all constraints.
    pub fn clear(&mut self) {
        self.constraints.clear();
    }

    /// Merges another constraint set into this one.
    ///
    /// This is useful when combining constraints from multiple sources.
    pub fn merge(&mut self, other: &ConstraintSet) {
        for (type_var, constraints) in other.iter() {
            for constraint in constraints {
                self.add(type_var, constraint.type_class, constraint.span.clone());
            }
        }
    }
}

impl Default for ConstraintSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_get() {
        let mut cs = ConstraintSet::new();

        // Add constraint to type var 0
        cs.add(0, TypeClassId::Numeric, Span(1..10));

        let constraints = cs.get(0);
        assert_eq!(constraints.len(), 1);
        assert_eq!(constraints[0].type_class, TypeClassId::Numeric);
        assert_eq!(constraints[0].span, Span(1..10));

        // Non-existent type var returns empty slice
        assert_eq!(cs.get(99).len(), 0);
    }

    #[test]
    fn test_multiple_constraints_same_var() {
        let mut cs = ConstraintSet::new();

        // Add multiple constraints to same type var
        cs.add(0, TypeClassId::Hashable, Span(1..5));
        cs.add(0, TypeClassId::Ord, Span(2..10));

        let constraints = cs.get(0);
        assert_eq!(constraints.len(), 2);
        assert_eq!(constraints[0].type_class, TypeClassId::Hashable);
        assert_eq!(constraints[1].type_class, TypeClassId::Ord);
    }

    #[test]
    fn test_multiple_vars() {
        let mut cs = ConstraintSet::new();

        cs.add(0, TypeClassId::Numeric, Span(1..5));
        cs.add(1, TypeClassId::Indexable, Span(2..10));
        cs.add(2, TypeClassId::Hashable, Span(3..15));

        assert_eq!(cs.len(), 3);
        assert_eq!(cs.get(0)[0].type_class, TypeClassId::Numeric);
        assert_eq!(cs.get(1)[0].type_class, TypeClassId::Indexable);
        assert_eq!(cs.get(2)[0].type_class, TypeClassId::Hashable);
    }

    #[test]
    fn test_is_empty() {
        let mut cs = ConstraintSet::new();
        assert!(cs.is_empty());

        cs.add(0, TypeClassId::Numeric, Span(1..1));
        assert!(!cs.is_empty());

        cs.clear();
        assert!(cs.is_empty());
    }

    #[test]
    fn test_iter() {
        let mut cs = ConstraintSet::new();

        cs.add(0, TypeClassId::Numeric, Span(1..5));
        cs.add(1, TypeClassId::Indexable, Span(2..10));

        let collected: Vec<_> = cs.iter().collect();
        assert_eq!(collected.len(), 2);
        assert_eq!(collected[0].0, 0);
        assert_eq!(collected[1].0, 1);
    }

    #[test]
    fn test_merge() {
        let mut cs1 = ConstraintSet::new();
        cs1.add(0, TypeClassId::Numeric, Span(1..5));

        let mut cs2 = ConstraintSet::new();
        cs2.add(0, TypeClassId::Hashable, Span(2..10));
        cs2.add(1, TypeClassId::Indexable, Span(3..15));

        cs1.merge(&cs2);

        assert_eq!(cs1.len(), 2);
        assert_eq!(cs1.get(0).len(), 2); // Both Numeric and Hashable
        assert_eq!(cs1.get(1).len(), 1); // Indexable
    }
}
