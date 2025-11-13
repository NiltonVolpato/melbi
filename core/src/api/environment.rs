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
/// ```
/// use melbi_core::api::{Engine, EngineOptions};
/// use melbi_core::values::dynamic::Value;
/// use bumpalo::Bump;
///
/// let arena = Bump::new();
///
/// // EnvironmentBuilder is used inside Engine::new
/// let engine = Engine::new(&arena, EngineOptions::default(), |_arena, type_mgr, env| {
///     // Register constant
///     env.register("pi", Value::float(type_mgr, std::f64::consts::PI));
/// });
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
    /// ```
    /// use melbi_core::api::{Engine, EngineOptions};
    /// use melbi_core::values::dynamic::Value;
    /// use bumpalo::Bump;
    ///
    /// let arena = Bump::new();
    /// let engine = Engine::new(&arena, EngineOptions::default(), |_arena, type_mgr, env| {
    ///     env.register("pi", Value::float(type_mgr, std::f64::consts::PI));
    /// });
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
