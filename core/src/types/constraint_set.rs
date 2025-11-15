/// Constraint set for tracking type class requirements with associated types.
///
/// Type classes represent relationships between types:
///   - Numeric(left, right, result): left + right => result
///   - Indexable(container, index, result): container[index] => result
///   - Hashable(ty): ty can be hashed
///
/// During type inference, operations add relational constraints. After unification,
/// the constraint solver verifies these relationships and may perform additional
/// unification to resolve associated types.
use crate::types::Type;
use crate::parser::Span;
use alloc::vec::Vec;

/// A type class constraint with associated types.
///
/// Represents relationships between types enforced by type classes.
#[derive(Debug, Clone)]
pub enum TypeClassConstraint<'types> {
    /// Numeric operation: left op right => result
    /// Instances: (Int, Int) => Int, (Float, Float) => Float
    Numeric {
        left: &'types Type<'types>,
        right: &'types Type<'types>,
        result: &'types Type<'types>,
        span: Span,
    },

    /// Indexing operation: container[index] => result
    /// Instances: (Array[E], Int) => E, (Map[K,V], K) => V, (Bytes, Int) => Int
    Indexable {
        container: &'types Type<'types>,
        index: &'types Type<'types>,
        result: &'types Type<'types>,
        span: Span,
    },

    /// Hashable type: ty can be used as a map key
    /// Instances: Int, Float, Bool, Str, Bytes, Symbol, Array[E] where E: Hashable
    Hashable {
        ty: &'types Type<'types>,
        span: Span,
    },

    /// Ord type: ty supports ordering operations
    /// Instances: Int, Float, Str, Bytes
    Ord {
        ty: &'types Type<'types>,
        span: Span,
    },
}

impl<'types> TypeClassConstraint<'types> {
    pub fn span(&self) -> &Span {
        match self {
            TypeClassConstraint::Numeric { span, .. } => span,
            TypeClassConstraint::Indexable { span, .. } => span,
            TypeClassConstraint::Hashable { span, .. } => span,
            TypeClassConstraint::Ord { span, .. } => span,
        }
    }
}

/// A set of type class constraints.
///
/// Unlike simple predicate constraints, these track relationships between
/// multiple types (e.g., "indexing this container with this index produces this result").
#[derive(Debug, Clone)]
pub struct ConstraintSet<'types> {
    /// List of all constraints
    constraints: Vec<TypeClassConstraint<'types>>,
}

impl<'types> ConstraintSet<'types> {
    /// Creates a new empty constraint set.
    pub fn new() -> Self {
        Self {
            constraints: Vec::new(),
        }
    }

    /// Adds a numeric constraint: left op right => result
    pub fn add_numeric(
        &mut self,
        left: &'types Type<'types>,
        right: &'types Type<'types>,
        result: &'types Type<'types>,
        span: Span,
    ) {
        self.constraints.push(TypeClassConstraint::Numeric { left, right, result, span });
    }

    /// Adds an indexable constraint: container[index] => result
    pub fn add_indexable(
        &mut self,
        container: &'types Type<'types>,
        index: &'types Type<'types>,
        result: &'types Type<'types>,
        span: Span,
    ) {
        self.constraints.push(TypeClassConstraint::Indexable { container, index, result, span });
    }

    /// Adds a hashable constraint: ty must be hashable
    pub fn add_hashable(&mut self, ty: &'types Type<'types>, span: Span) {
        self.constraints.push(TypeClassConstraint::Hashable { ty, span });
    }

    /// Adds an ord constraint: ty must support ordering
    pub fn add_ord(&mut self, ty: &'types Type<'types>, span: Span) {
        self.constraints.push(TypeClassConstraint::Ord { ty, span });
    }

    /// Returns an iterator over all constraints.
    pub fn iter(&self) -> impl Iterator<Item = &TypeClassConstraint<'types>> {
        self.constraints.iter()
    }

    /// Returns true if there are no constraints.
    pub fn is_empty(&self) -> bool {
        self.constraints.is_empty()
    }

    /// Returns the number of constraints.
    pub fn len(&self) -> usize {
        self.constraints.len()
    }

    /// Clears all constraints.
    pub fn clear(&mut self) {
        self.constraints.clear();
    }
}

impl<'types> Default for ConstraintSet<'types> {
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
    fn test_add_numeric() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);
        let mut cs = ConstraintSet::new();

        cs.add_numeric(tm.int(), tm.int(), tm.int(), Span(1..10));

        assert_eq!(cs.len(), 1);
        assert!(!cs.is_empty());
    }

    #[test]
    fn test_add_indexable() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);
        let mut cs = ConstraintSet::new();

        let arr = tm.array(tm.int());
        cs.add_indexable(arr, tm.int(), tm.int(), Span(1..10));

        assert_eq!(cs.len(), 1);
    }

    #[test]
    fn test_clear() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);
        let mut cs = ConstraintSet::new();

        cs.add_hashable(tm.int(), Span(1..1));
        assert!(!cs.is_empty());

        cs.clear();
        assert!(cs.is_empty());
    }
}
