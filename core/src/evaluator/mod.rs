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

pub use error::{EvalError, ResourceExceeded, RuntimeError};

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
/// - `globals`: Global constants and functions (e.g., PI, Math package)
/// - `variables`: Client-provided runtime variables
///
/// ## Returns
///
/// The resulting value, or an evaluation error.
///
/// ## Example
///
/// ```ignore
/// let globals = [("PI", Value::float(type_manager, 3.14159))];
/// let variables = [("x", Value::int(type_manager, 42))];
/// let result = eval(type_manager, &arena, &typed_expr, &globals, &variables)?;
/// ```
pub fn eval<'types, 'arena>(
    arena: &'arena Bump,
    type_manager: &'types TypeManager<'types>,
    expr: &'arena TypedExpr<'types, 'arena>,
    globals: &[(&'arena str, Value<'types, 'arena>)],
    variables: &[(&'arena str, Value<'types, 'arena>)],
) -> Result<Value<'types, 'arena>, EvalError>
where
    'types: 'arena,
{
    eval_with_limits(arena, type_manager, expr, globals, variables, 1000)
}

/// Evaluate a type-checked expression with custom depth limit.
///
/// ## Arguments
///
/// - `type_manager`: Type manager used during type-checking
/// - `arena`: Bump allocator for allocating result values
/// - `expr`: Type-checked expression to evaluate
/// - `globals`: Global constants and functions (e.g., PI, Math package)
/// - `variables`: Client-provided runtime variables
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
/// let result = eval_with_limits(type_manager, &arena, &typed_expr, &[], &[], 5000)?;
/// ```
pub fn eval_with_limits<'types, 'arena>(
    arena: &'arena Bump,
    type_manager: &'types TypeManager<'types>,
    expr: &'arena TypedExpr<'types, 'arena>,
    globals: &[(&'arena str, Value<'types, 'arena>)],
    variables: &[(&'arena str, Value<'types, 'arena>)],
    max_depth: usize,
) -> Result<Value<'types, 'arena>, EvalError>
where
    'types: 'arena,
{
    eval::Evaluator::new(arena, type_manager, globals, variables, max_depth).eval(expr)
}
