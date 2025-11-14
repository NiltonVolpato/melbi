//! Tree-walking AST evaluator for Melbi expressions.
//!
//! The evaluator interprets typed AST expressions (`TypedExpr`) and produces runtime values (`Value`).
//!
//! ## Design Principles
//!
//! - **Never panic**: All adversarial inputs must be handled gracefully
//! - **Stack-safe**: Depth tracking prevents stack overflow from deeply nested expressions
//! - **Type-safe**: Evaluates type-checked expressions, so many error conditions are impossible
//!
//! ## Example
//!
//! ```ignore
//! use melbi_core::{parser, analyzer, evaluator::{eval::Evaluator, EvaluatorOptions}};
//! use bumpalo::Bump;
//!
//! let arena = Bump::new();
//! let type_manager = TypeManager::new(&arena);
//!
//! // Parse and type-check
//! let parsed = parser::parse(&arena, "1 + 2").unwrap();
//! let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
//!
//! // Evaluate
//! let result = Evaluator::new(
//!     EvaluatorOptions::default(),
//!     &arena,
//!     type_manager,
//!     &[],
//!     &[],
//! )
//! .eval(&typed)
//! .unwrap();
//! assert_eq!(result.as_int(), Some(3));
//! ```

mod error;
mod eval;
mod operators;

#[cfg(test)]
mod eval_test;

pub use error::{ExecutionError, ResourceExceededError, RuntimeError};

/// Options for configuring the evaluator.
pub struct EvaluatorOptions {
    /// Maximum evaluation stack depth (for recursion protection).
    pub max_depth: usize,
}

impl Default for EvaluatorOptions {
    fn default() -> Self {
        Self { max_depth: 1000 }
    }
}

pub use eval::Evaluator;
