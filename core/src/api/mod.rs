//! Public API for the Melbi expression language.
//!
//! This module provides the stable public API for compiling and executing
//! Melbi expressions. It follows the three-tier design:
//!
//! 1. **Unchecked API**: Maximum performance, no validation (`run_unchecked`)
//! 2. **Dynamic API**: Runtime validation, C FFI compatible (`run`)
//! 3. **Static API**: (Future) Compile-time type checking
//!
//! # Example
//!
//! ```ignore
//! use melbi_core::api::{CompilationOptions, Engine, EngineOptions};
//! use melbi_core::values::{NativeFunction, dynamic::Value};
//! use bumpalo::Bump;
//!
//! let arena = Bump::new();
//! let options = EngineOptions::default();
//!
//! let engine = Engine::new(&arena, options, |arena, type_mgr, env| {
//!     // Register constants
//!     env.register("pi", Value::float(type_mgr, 3.14159));
//!
//!     // Register functions
//!     let add_ty = type_mgr.function(&[type_mgr.int(), type_mgr.int()], type_mgr.int());
//!     env.register("add", Value::function(arena, NativeFunction::new(add_ty, add_impl)).unwrap());
//! });
//!
//! // Compile expression
//! let compile_opts = CompilationOptions::default();
//! let expr = engine.compile(compile_opts, "pi * 2", &[]).unwrap();
//!
//! // Execute
//! let val_arena = Bump::new();
//! let result = expr.run(&val_arena, &[]).unwrap();
//! ```

pub mod engine;
pub mod environment;
pub mod error;
pub mod expression;
pub mod options;

pub use engine::Engine;
pub use environment::EnvironmentBuilder;
pub use error::{Diagnostic, Error, RelatedInfo, Severity};
pub use expression::CompiledExpression;
pub use options::{CompilationOptions, EngineOptions, ExecutionOptions};
