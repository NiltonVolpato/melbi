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
//! use melbi_core::{parser, analyzer, evaluator};
//! use bumpalo::Bump;
//!
//! let arena = Bump::new();
//! let type_manager = TypeManager::new(&arena);
//!
//! // Parse and type-check
//! let parsed = parser::parse(&arena, "1 + 2").unwrap();
//! let typed = analyzer::analyze(type_manager, &arena, &parsed).unwrap();
//!
//! // Evaluate
//! let result = evaluator::eval(type_manager, &arena, &typed).unwrap();
//! assert_eq!(result.as_int(), Some(3));
//! ```

mod error;
mod eval;
mod operators;

#[cfg(test)]
mod eval_test;

pub use error::EvalError;

use crate::{analyzer::typed_expr::TypedExpr, types::manager::TypeManager, values::dynamic::Value};
use bumpalo::Bump;

/// Evaluate a type-checked expression with default limits.
///
/// Uses default stack depth limit of 1000.
///
/// ## Arguments
///
/// - `type_manager`: Type manager used during type-checking
/// - `arena`: Bump allocator for allocating result values
/// - `expr`: Type-checked expression to evaluate
///
/// ## Returns
///
/// The resulting value, or an evaluation error.
///
/// ## Example
///
/// ```ignore
/// let result = eval(type_manager, &arena, &typed_expr)?;
/// ```
pub fn eval<'types, 'arena>(
    type_manager: &'types TypeManager<'types>,
    arena: &'arena Bump,
    expr: &'arena TypedExpr<'types, 'arena>,
) -> Result<Value<'types, 'arena>, EvalError>
where
    'types: 'arena,
{
    eval_with_limits(type_manager, arena, expr, 1000)
}

/// Evaluate a type-checked expression with custom depth limit.
///
/// ## Arguments
///
/// - `type_manager`: Type manager used during type-checking
/// - `arena`: Bump allocator for allocating result values
/// - `expr`: Type-checked expression to evaluate
/// - `max_depth`: Maximum evaluation stack depth (for recursion protection)
///
/// ## Returns
///
/// The resulting value, or an evaluation error.
///
/// ## Example
///
/// ```ignore
/// // Allow deeper recursion for specific use case
/// let result = eval_with_limits(type_manager, &arena, &typed_expr, 5000)?;
/// ```
pub fn eval_with_limits<'types, 'arena>(
    type_manager: &'types TypeManager<'types>,
    arena: &'arena Bump,
    expr: &'arena TypedExpr<'types, 'arena>,
    max_depth: usize,
) -> Result<Value<'types, 'arena>, EvalError>
where
    'types: 'arena,
{
    eval::Evaluator::new(type_manager, arena, max_depth).eval(expr)
}
