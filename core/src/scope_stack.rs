//! Generic scope stack for variable bindings.
//!
//! Used by both the analyzer (binds types) and evaluator (binds values).
//! Supports two kinds of scopes:
//! - **Complete scopes**: Immutable, pre-populated (globals, variables)
//! - **Incomplete scopes**: Mutable, filled incrementally (where, lambda)
//!
//! The incomplete scope design enables sequential binding semantics where
//! later bindings can reference earlier ones:
//! ```melbi
//! b where { a = 1, b = a + 1 }  // `b` can see `a`
//! ```

use alloc::string::ToString;
use bumpalo::Bump;
use core::fmt;

/// A stack of scopes for variable lookup.
///
/// Maintains two separate stacks:
/// - `complete`: Immutable scopes (globals, variables)
/// - `incomplete`: Mutable scopes being built (where, lambda)
///
/// Lookups search incomplete scopes first (innermost to outermost),
/// then complete scopes (innermost to outermost).
pub struct ScopeStack<'arena, T> {
    /// Complete, immutable scopes (globals, variables).
    /// Each scope is a sorted slice for binary search.
    complete: alloc::vec::Vec<&'arena [(&'arena str, T)]>,

    /// Incomplete, mutable scopes being built (where, lambda).
    /// Names are pre-sorted, values start as None and are filled incrementally.
    incomplete: alloc::vec::Vec<&'arena mut [(&'arena str, Option<T>)]>,
}

impl<'arena, T: Copy> ScopeStack<'arena, T> {
    /// Create a new empty scope stack.
    pub fn new() -> Self {
        Self {
            complete: alloc::vec::Vec::new(),
            incomplete: alloc::vec::Vec::new(),
        }
    }

    /// Push a complete scope (globals, variables).
    ///
    /// The scope must be sorted by name for binary search.
    /// No ownership transfer - just stores a reference.
    pub fn push_complete(&mut self, scope: &'arena [(&'arena str, T)]) {
        debug_assert!(is_sorted(scope), "Scope must be sorted by name");
        self.complete.push(scope);
    }

    /// Push an incomplete scope with pre-declared names (where, lambda).
    ///
    /// Names are sorted and allocated in the arena.
    /// Values start as `None` and must be filled via `bind_in_current()`.
    ///
    /// Returns an error if there are duplicate names.
    pub fn push_incomplete(
        &mut self,
        arena: &'arena Bump,
        names: &[&'arena str],
    ) -> Result<(), DuplicateError> {
        let mut sorted_names = alloc::vec::Vec::from(names);
        sorted_names.sort_unstable();

        // Check for duplicates
        for window in sorted_names.windows(2) {
            if window[0] == window[1] {
                return Err(DuplicateError(window[0].to_string()));
            }
        }

        // Allocate mutable slice with None values
        let scope = arena.alloc_slice_fill_with(sorted_names.len(), |i| (sorted_names[i], None));

        self.incomplete.push(scope);
        Ok(())
    }

    /// Bind a value in the topmost incomplete scope.
    ///
    /// The name must have been declared when the scope was pushed.
    /// Returns an error if:
    /// - There is no incomplete scope
    /// - The name was not declared in the current scope
    /// - The name has already been bound
    pub fn bind_in_current(&mut self, name: &str, value: T) -> Result<(), BindError> {
        let scope = self
            .incomplete
            .last_mut()
            .ok_or(BindError::NoIncompleteScope)?;

        match scope.binary_search_by_key(&name, |(n, _)| *n) {
            Ok(idx) => {
                if scope[idx].1.is_some() {
                    return Err(BindError::AlreadyBound(name.to_string()));
                }
                scope[idx].1 = Some(value);
                Ok(())
            }
            Err(_) => Err(BindError::NameNotDeclared(name.to_string())),
        }
    }

    /// Pop the topmost incomplete scope.
    ///
    /// Returns an error if there are no incomplete scopes.
    pub fn pop_incomplete(&mut self) -> Result<(), PopError> {
        self.incomplete.pop().ok_or(PopError::NoIncompleteScope)?;
        Ok(())
    }

    /// Pop the topmost complete scope.
    ///
    /// Returns an error if there are no complete scopes.
    pub fn pop_complete(&mut self) -> Result<(), PopError> {
        self.complete.pop().ok_or(PopError::NoCompleteScope)?;
        Ok(())
    }

    /// Look up a name, searching incomplete scopes first, then complete.
    ///
    /// Searches in reverse order (innermost to outermost).
    /// For incomplete scopes, only returns values that have been bound.
    pub fn lookup(&self, name: &str) -> Option<&T> {
        // Search incomplete scopes (innermost to outermost)
        for scope in self.incomplete.iter().rev() {
            match scope.binary_search_by_key(&name, |(n, _)| *n) {
                Ok(idx) => {
                    // Only return if the value has been bound
                    if let Some(ref val) = scope[idx].1 {
                        return Some(val);
                    }
                }
                Err(_) => continue,
            }
        }

        // Search complete scopes (innermost to outermost)
        for scope in self.complete.iter().rev() {
            match scope.binary_search_by_key(&name, |(n, _)| *n) {
                Ok(idx) => return Some(&scope[idx].1),
                Err(_) => continue,
            }
        }

        None
    }
}

/// Check if a slice is sorted by name (for debug assertions).
fn is_sorted<T>(slice: &[(&str, T)]) -> bool {
    slice.windows(2).all(|w| w[0].0 <= w[1].0)
}

/// Error when trying to bind a value in an incomplete scope.
#[derive(Debug, Clone)]
pub enum BindError {
    /// No incomplete scope exists to bind in.
    NoIncompleteScope,
    /// The name has already been bound in the current scope.
    AlreadyBound(alloc::string::String),
    /// The name was not declared when the scope was created.
    NameNotDeclared(alloc::string::String),
}

impl fmt::Display for BindError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BindError::NoIncompleteScope => write!(f, "No incomplete scope to bind in"),
            BindError::AlreadyBound(name) => {
                write!(f, "Name '{}' already bound in current scope", name)
            }
            BindError::NameNotDeclared(name) => {
                write!(f, "Name '{}' not declared in current scope", name)
            }
        }
    }
}

/// Error when trying to pop a scope.
#[derive(Debug, Clone)]
pub enum PopError {
    /// No incomplete scope to pop.
    NoIncompleteScope,
    /// No complete scope to pop.
    NoCompleteScope,
}

impl fmt::Display for PopError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PopError::NoIncompleteScope => write!(f, "No incomplete scope to pop"),
            PopError::NoCompleteScope => write!(f, "No complete scope to pop"),
        }
    }
}

/// Error when duplicate names are found in a scope.
#[derive(Debug, Clone)]
pub struct DuplicateError(pub alloc::string::String);

impl fmt::Display for DuplicateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Duplicate name '{}' in scope", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complete_scope_lookup() {
        let bump = Bump::new();
        let mut stack = ScopeStack::new();

        let scope = bump.alloc_slice_copy(&[("a", 1), ("b", 2), ("c", 3)]);
        stack.push_complete(scope);

        assert_eq!(stack.lookup("a"), Some(&1));
        assert_eq!(stack.lookup("b"), Some(&2));
        assert_eq!(stack.lookup("c"), Some(&3));
        assert_eq!(stack.lookup("d"), None);
    }

    #[test]
    fn test_incomplete_scope_sequential_binding() {
        let bump = Bump::new();
        let mut stack = ScopeStack::new();

        // Push incomplete scope with names
        stack.push_incomplete(&bump, &["a", "b"]).unwrap();

        // Before binding, lookup returns None
        assert_eq!(stack.lookup("a"), None);
        assert_eq!(stack.lookup("b"), None);

        // Bind 'a'
        stack.bind_in_current("a", 1).unwrap();
        assert_eq!(stack.lookup("a"), Some(&1));
        assert_eq!(stack.lookup("b"), None);

        // Bind 'b'
        stack.bind_in_current("b", 2).unwrap();
        assert_eq!(stack.lookup("a"), Some(&1));
        assert_eq!(stack.lookup("b"), Some(&2));
    }

    #[test]
    fn test_shadowing() {
        let bump = Bump::new();
        let mut stack = ScopeStack::new();

        // Push complete scope
        let scope1 = bump.alloc_slice_copy(&[("a", 1), ("b", 2)]);
        stack.push_complete(scope1);

        // Push incomplete scope that shadows 'a'
        stack.push_incomplete(&bump, &["a"]).unwrap();
        stack.bind_in_current("a", 10).unwrap();

        // 'a' is shadowed, 'b' is not
        assert_eq!(stack.lookup("a"), Some(&10));
        assert_eq!(stack.lookup("b"), Some(&2));

        // Pop incomplete scope
        stack.pop_incomplete().unwrap();

        // Original 'a' is visible again
        assert_eq!(stack.lookup("a"), Some(&1));
    }

    #[test]
    fn test_duplicate_names_error() {
        let bump = Bump::new();
        let mut stack: ScopeStack<i32> = ScopeStack::new();

        let result = stack.push_incomplete(&bump, &["a", "b", "a"]);
        assert!(result.is_err());
        assert!(matches!(result, Err(DuplicateError(_))));
    }

    #[test]
    fn test_bind_already_bound_error() {
        let bump = Bump::new();
        let mut stack = ScopeStack::new();

        stack.push_incomplete(&bump, &["a"]).unwrap();
        stack.bind_in_current("a", 1).unwrap();

        let result = stack.bind_in_current("a", 2);
        assert!(result.is_err());
        assert!(matches!(result, Err(BindError::AlreadyBound(_))));
    }

    #[test]
    fn test_bind_undeclared_name_error() {
        let bump = Bump::new();
        let mut stack = ScopeStack::new();

        stack.push_incomplete(&bump, &["a"]).unwrap();

        let result = stack.bind_in_current("b", 1);
        assert!(result.is_err());
        assert!(matches!(result, Err(BindError::NameNotDeclared(_))));
    }
}
