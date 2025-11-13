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
//! ```ignore
//! use melbi::{Engine, EngineOptions};
//! use melbi::values::{NativeFunction, dynamic::Value};
//! use bumpalo::Bump;
//!
//! // Create an arena for type and environment data
//! let arena = Bump::new();
//! let options = EngineOptions::default();
//!
//! // Create an engine with a global environment
//! let engine = Engine::new(&arena, options, |arena, type_mgr, env| {
//!     // Register a constant
//!     env.register("pi", Value::float(type_mgr, 3.14159));
//! });
//!
//! // Compile an expression
//! let expr = engine.compile("pi * 2.0", &[]).unwrap();
//!
//! // Execute in a separate arena
//! let val_arena = Bump::new();
//! let result = expr.run(&val_arena, &[]).unwrap();
//! assert_eq!(result.as_float().unwrap(), std::f64::consts::PI * 2.0);
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
//! ```ignore
//! use melbi::values::{NativeFunction, dynamic::Value};
//! use melbi::evaluator::EvalError;
//!
//! fn add<'types, 'arena>(
//!     _arena: &'arena Bump,
//!     type_mgr: &'types TypeManager<'types>,
//!     args: &[Value<'types, 'arena>],
//! ) -> Result<Value<'types, 'arena>, EvalError> {
//!     let a = args[0].as_int()?;
//!     let b = args[1].as_int()?;
//!     Ok(Value::int(type_mgr, a + b))
//! }
//!
//! let engine = Engine::new(&arena, options, |arena, type_mgr, env| {
//!     let add_ty = type_mgr.function(&[type_mgr.int(), type_mgr.int()], type_mgr.int());
//!     env.register("add", Value::function(arena, NativeFunction::new(add_ty, add)).unwrap());
//! });
//! ```

// Re-export public API from melbi_core
pub use melbi_core::api::{
    CompilationOptions, CompiledExpression, Diagnostic, Engine, EngineOptions, EnvironmentBuilder,
    Error, RelatedInfo, Severity,
};

// Re-export commonly used types and values
pub use melbi_core::types::{self, Type, manager::TypeManager, traits::{TypeBuilder, TypeView}};
pub use melbi_core::values::{self, dynamic::Value, NativeFunction, NativeFn, Function};

// Re-export errors
pub use melbi_core::evaluator::EvalError;
