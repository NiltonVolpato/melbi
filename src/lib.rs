//! Melbi - A flexible, embeddable expression language
//!
//! # Overview
//!
//! Melbi is an expression-focused scripting language designed for safe evaluation
//! of user-defined logic in host applications. Common use cases include:
//!
//! - Email filters and routing rules
//! - Feature flags and conditional logic
//! - Data transformations and mappings
//! - Business rules engines
//!
//! # Quick Start
//!
//! ```
//! use melbi::{CompileOptions, Engine, EngineOptions};
//! use melbi::values::dynamic::Value;
//! use bumpalo::Bump;
//!
//! // Create an arena for type and environment data
//! let arena = Bump::new();
//! let options = EngineOptions::default();
//!
//! // Create an engine with a global environment
//! let engine = Engine::new(options, &arena, |_arena, type_mgr, env| {
//!     // Register a constant
//!     env.register("pi", Value::float(type_mgr, std::f64::consts::PI))
//!         .expect("registration should succeed");
//! });
//!
//! // Compile an expression
//! let compile_opts = CompileOptions::default();
//! let expr = engine.compile(compile_opts, "pi * 2.0", &[]).unwrap();
//!
//! // Execute in a separate arena
//! let val_arena = Bump::new();
//! let result = expr.run(None, &val_arena, &[]).unwrap();
//! let result_float = result.as_float().unwrap();
//! assert!((result_float - (std::f64::consts::PI * 2.0)).abs() < 0.0001);
//! ```
//!
//! # API Tiers
//!
//! Melbi provides two API tiers:
//!
//! 1. **Dynamic API** (`run`): Runtime validation, works from any language
//! 2. **Unchecked API** (`run_unchecked`): No validation, maximum performance
//!
//! # FFI Support
//!
//! Register native Rust functions using the `NativeFunction` wrapper:
//!
//! ```
//! use melbi::{Engine, EngineOptions, CompileOptions, EvalError};
//! use melbi::values::{NativeFunction, dynamic::Value};
//! use melbi::types::manager::TypeManager;
//! use bumpalo::Bump;
//!
//! fn add<'types, 'arena>(
//!     _arena: &'arena Bump,
//!     type_mgr: &'types TypeManager<'types>,
//!     args: &[Value<'types, 'arena>],
//! ) -> Result<Value<'types, 'arena>, EvalError> {
//!     debug_assert!(args.len() == 2);
//!     let a = args[0].as_int().expect("arg should be int");
//!     let b = args[1].as_int().expect("arg should be int");
//!     Ok(Value::int(type_mgr, a + b))
//! }
//!
//! let arena = Bump::new();
//! let options = EngineOptions::default();
//! let engine = Engine::new(options, &arena, |arena, type_mgr, env| {
//!     let add_ty = type_mgr.function(&[type_mgr.int(), type_mgr.int()], type_mgr.int());
//!     env.register("add", Value::function(arena, NativeFunction::new(add_ty, add)).unwrap())
//!         .expect("registration should succeed");
//! });
//!
//! // Use the function
//! let expr = engine.compile(CompileOptions::default(), "add(40, 2)", &[]).unwrap();
//! let val_arena = Bump::new();
//! let result = expr.run(None, &val_arena, &[]).unwrap();
//! assert_eq!(result.as_int().unwrap(), 42);
//! ```

// Re-export public API from melbi_core
pub use melbi_core::api::{
    CompileOptions, CompiledExpression, Diagnostic, Engine, EngineOptions, EnvironmentBuilder,
    Error, RelatedInfo, RunOptions, Severity,
};

// Re-export commonly used types and values
pub use melbi_core::types::{
    self, Type,
    manager::TypeManager,
    traits::{TypeBuilder, TypeView},
};
pub use melbi_core::values::{self, Function, NativeFn, NativeFunction, dynamic::Value};

// Re-export errors
pub use melbi_core::evaluator::EvalError; // XXX: This should not be user-facing.
