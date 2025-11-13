//! The Melbi compilation engine.

use super::{CompilationOptions, CompiledExpression, EngineOptions, EnvironmentBuilder, Error};
use crate::types::{Type, manager::TypeManager};
use crate::values::dynamic::Value;
use crate::{Vec, analyzer, parser};
use bumpalo::Bump;

/// The Melbi compilation and execution engine.
///
/// The engine manages:
/// - Type system (TypeManager)
/// - Global environment (constants, functions, packages)
/// - Runtime configuration (EngineOptions)
///
/// # Lifetimes
///
/// - `'arena`: Lifetime of the arena holding types and environment data.
///   All compiled expressions borrow from this arena.
///
/// # Example
///
/// ```ignore
/// use melbi_core::api::{Engine, EngineOptions};
/// use melbi_core::values::{NativeFunction, dynamic::Value};
/// use bumpalo::Bump;
///
/// let arena = Bump::new();
/// let options = EngineOptions::default();
///
/// let engine = Engine::new(&arena, options, |arena, type_mgr, env| {
///     // Register a constant
///     env.register("pi", Value::float(type_mgr, 3.14159));
///
///     // Register a function
///     fn add(arena: &Bump, type_mgr: &TypeManager, args: &[Value]) -> Result<Value, EvalError> {
///         let a = args[0].as_int()?;
///         let b = args[1].as_int()?;
///         Ok(Value::int(type_mgr, a + b))
///     }
///
///     let add_ty = type_mgr.function(&[type_mgr.int(), type_mgr.int()], type_mgr.int());
///     env.register("add", Value::function(arena, NativeFunction::new(add_ty, add)).unwrap());
/// });
///
/// // Compile an expression
/// let expr = engine.compile("add(40, 2)", &[]).unwrap();
///
/// // Execute
/// let val_arena = Bump::new();
/// let result = expr.run(&val_arena, &[]).unwrap();
/// assert_eq!(result.as_int().unwrap(), 42);
/// ```
pub struct Engine<'arena> {
    arena: &'arena Bump,
    type_manager: &'arena TypeManager<'arena>,
    environment: &'arena [(&'arena str, Value<'arena, 'arena>)],
    /// Precomputed globals for analyzer (name, type) pairs
    /// TODO: Switch to TypeScheme when generic functions are supported
    globals_for_analyzer: &'arena [(&'arena str, &'arena Type<'arena>)],
    options: EngineOptions,
}

impl<'arena> Engine<'arena> {
    /// Create a new engine with a custom environment.
    ///
    /// The initialization closure receives:
    /// - `arena`: The arena for allocating environment data
    /// - `type_mgr`: The type builder for creating types
    /// - `env`: The environment builder for registering globals
    ///
    /// # Example
    ///
    /// ```ignore
    /// let engine = Engine::new(&arena, options, |arena, type_mgr, env| {
    ///     env.register("pi", Value::float(type_mgr, 3.14159));
    /// });
    /// ```
    pub fn new(
        arena: &'arena Bump,
        options: EngineOptions,
        init: impl FnOnce(&'arena Bump, &'arena TypeManager<'arena>, &mut EnvironmentBuilder<'arena>),
    ) -> Self
    {
        // Create type manager
        let type_manager = TypeManager::new(arena);

        // Build environment using the initialization closure
        let mut env_builder = EnvironmentBuilder::new(arena);
        init(arena, type_manager, &mut env_builder);
        let environment = env_builder.build(arena);

        // Precompute globals for analyzer (convert Value to Type)
        // TODO: Switch to TypeScheme when generic functions are supported
        let globals: Vec<(&'arena str, &'arena Type<'arena>)> = environment
            .iter()
            .map(|(name, value)| (*name, value.ty))
            .collect();
        let globals_for_analyzer = arena.alloc_slice_copy(&globals);

        Self {
            arena,
            type_manager,
            environment,
            globals_for_analyzer,
            options,
        }
    }

    /// Access the type manager.
    ///
    /// Useful for creating types when building expressions programmatically.
    pub fn type_manager(&self) -> &'arena TypeManager<'arena> {
        self.type_manager
    }

    /// Access the global environment.
    ///
    /// Returns a sorted slice of (name, value) pairs.
    pub fn environment(&self) -> &[(&'arena str, Value<'arena, 'arena>)] {
        self.environment
    }

    /// Access the engine options.
    pub fn options(&self) -> &EngineOptions {
        &self.options
    }

    /// Compile a Melbi expression.
    ///
    /// # Parameters
    ///
    /// - `options`: Compilation options (use `CompilationOptions::default()` for defaults)
    /// - `source`: The source code of the expression
    /// - `params`: Parameters for the expression as (name, type) pairs
    ///
    /// # Returns
    ///
    /// A compiled expression ready for execution, or a compilation error.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Compile a parameterized expression
    /// let int_ty = engine.type_manager().int();
    /// let options = CompilationOptions::default();
    /// let expr = engine.compile(options, "x + y", &[("x", int_ty), ("y", int_ty)])?;
    ///
    /// // Execute with arguments
    /// let result = expr.run(&arena, &[Value::int(int_ty, 10), Value::int(int_ty, 32)])?;
    /// assert_eq!(result.as_int()?, 42);
    /// ```
    pub fn compile(
        &self,
        _options: CompilationOptions,
        source: &'arena str,
        params: &[(&'arena str, &'arena Type<'arena>)],
    ) -> Result<CompiledExpression<'arena>, Error> {
        // Parse the source
        let parsed = parser::parse(self.arena, source)?;

        // Prepare parameters for analysis - copy to arena
        // Since params is already (&str, &Type), we can just copy the slice directly
        let params_slice = self.arena.alloc_slice_copy(params);

        // Type check the expression using precomputed globals
        let typed_expr = analyzer::analyze(
            self.type_manager,
            self.arena,
            &parsed,
            self.globals_for_analyzer,
            params_slice,
        )?;

        // Create compiled expression with default execution options
        Ok(CompiledExpression::new(
            typed_expr,
            self.type_manager,
            params_slice,
            self.environment,
            self.options.default_execution_options.clone(),
        ))
    }
}
