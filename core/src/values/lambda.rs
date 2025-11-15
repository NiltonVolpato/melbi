//! Lambda function implementation for closures.
//!
//! This module defines `LambdaFunction` which represents Melbi lambdas as callable values.
//! Currently only supports non-capturing lambdas (closures will be added in a future phase).

use super::dynamic::Value;
use super::function::Function;
use crate::analyzer::typed_expr::TypedExpr;
use crate::evaluator::{Evaluator, EvaluatorOptions, ExecutionError};
use crate::scope_stack::CompleteScope;
use crate::types::{Type, manager::TypeManager};
use alloc::vec::Vec;
use bumpalo::Bump;

/// A lambda function value.
///
/// Stores the lambda's type signature, parameters, body expression, and captured variables.
/// When called, it evaluates the body in a new scope with captures and parameters bound.
///
/// # Closure Support
///
/// Lambdas can capture variables from their enclosing scope. Captured variables are stored
/// as a slice of (name, value) pairs and pushed onto the scope stack when the lambda is called.
///
/// # Current Limitations
///
/// - No recursive lambdas (would require Y-combinator or named functions)
///
/// # Future Extensions
///
/// - Multi-value return (for pattern matching)
pub struct LambdaFunction<'types, 'arena> {
    /// The function's type signature (Function type)
    ty: &'types Type<'types>,

    /// Parameter names
    params: &'arena [&'arena str],

    /// The lambda body expression with annotations (for error reporting)
    body: &'arena TypedExpr<'types, 'arena>,

    /// Captured variables from the enclosing scope
    captures: &'arena [(&'arena str, Value<'types, 'arena>)],
}

impl<'types, 'arena> LambdaFunction<'types, 'arena> {
    /// Create a new lambda function.
    ///
    /// # Parameters
    ///
    /// - `ty`: The function's type (must be a Function type)
    /// - `params`: Parameter names
    /// - `body`: The typed body expression with source annotations
    /// - `captures`: Captured variables from the enclosing scope
    pub fn new(
        ty: &'types Type<'types>,
        params: &'arena [&'arena str],
        body: &'arena TypedExpr<'types, 'arena>,
        captures: &'arena [(&'arena str, Value<'types, 'arena>)],
    ) -> Self {
        debug_assert!(
            matches!(ty, Type::Function { .. }),
            "LambdaFunction type must be Function"
        );

        Self {
            ty,
            params,
            body,
            captures,
        }
    }
}

impl<'types, 'arena> Function<'types, 'arena> for LambdaFunction<'types, 'arena> {
    fn ty(&self) -> &'types Type<'types> {
        self.ty
    }

    unsafe fn call_unchecked(
        &self,
        arena: &'arena Bump,
        type_mgr: &'types TypeManager<'types>,
        args: &[Value<'types, 'arena>],
    ) -> Result<Value<'types, 'arena>, ExecutionError> {
        // Build parameter bindings for the lambda call
        let mut param_bindings: Vec<_> = self
            .params
            .iter()
            .zip(args.iter())
            .map(|(name, value)| (*name, *value))
            .collect();

        // Sort parameter bindings by name for binary search in CompleteScope
        param_bindings.sort_by_key(|(name, _)| *name);

        // Create an evaluator with the lambda body's TypedExpr
        // Scope order: globals (empty) → captures → parameters
        let mut evaluator = Evaluator::new(
            EvaluatorOptions::default(),
            arena,
            type_mgr,
            self.body, // Pass the full TypedExpr for error context
            &[], // No globals passed - they'll be accessed through normal scoping
            &[], // We'll push captures and parameters manually
        );

        // Push captures scope
        if !self.captures.is_empty() {
            evaluator.push_scope(CompleteScope::from_sorted(self.captures));
        }

        // Push parameters scope
        let param_slice = arena.alloc_slice_copy(&param_bindings);
        evaluator.push_scope(CompleteScope::from_sorted(param_slice));

        // Evaluate the body expression (now with full error context)
        evaluator.eval()
    }
}
