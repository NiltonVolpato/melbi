//! Environment builder for registering global values.

use crate::{Vec, values::dynamic::Value};
use bumpalo::Bump;

/// Builder for constructing the global environment.
///
/// The environment contains constants, functions, and packages that are
/// globally available to all expressions compiled with the engine.
///
/// # Example
///
/// ```ignore
/// use melbi_core::api::EnvironmentBuilder;
/// use melbi_core::values::{NativeFunction, dynamic::Value};
/// use bumpalo::Bump;
///
/// let arena = Bump::new();
/// let type_mgr = TypeManager::new(&arena);
/// let mut env = EnvironmentBuilder::new(&arena);
///
/// // Register constant
/// env.register("pi", Value::float(type_mgr, 3.14159));
///
/// // Register function
/// let add_ty = type_mgr.function(&[type_mgr.int(), type_mgr.int()], type_mgr.int());
/// env.register("add", Value::function(&arena, NativeFunction::new(add_ty, add_fn)).unwrap());
/// ```
pub struct EnvironmentBuilder<'arena> {
    arena: &'arena Bump,
    entries: Vec<(&'arena str, Value<'arena, 'arena>)>,
}

impl<'arena> EnvironmentBuilder<'arena> {
    /// Create a new environment builder.
    pub fn new(arena: &'arena Bump) -> Self {
        Self {
            arena,
            entries: Vec::new(),
        }
    }

    /// Register a global value (constant, function, or package).
    ///
    /// The name is interned in the arena. Values are sorted by name at build time
    /// for efficient binary search during compilation and evaluation.
    ///
    /// # Example
    ///
    /// ```ignore
    /// env.register("pi", Value::float(type_mgr, 3.14159));
    /// ```
    pub fn register(&mut self, name: &str, value: Value<'arena, 'arena>) {
        let name = self.arena.alloc_str(name);
        self.entries.push((name, value));
    }

    /// Build the final sorted environment slice.
    ///
    /// This is called internally by Engine::new(). The resulting slice is
    /// sorted by name for efficient binary search during lookups.
    pub(crate) fn build(
        mut self,
        arena: &'arena Bump,
    ) -> &'arena [(&'arena str, Value<'arena, 'arena>)] {
        // Sort by name for efficient binary search during lookup
        self.entries.sort_by_key(|(name, _)| *name);
        arena.alloc_slice_copy(&self.entries)
    }
}
