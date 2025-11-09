//! Lambda function implementation for closures.
//!
//! This module defines `LambdaFunction` which represents Melbi lambdas as callable values.
//! Currently only supports non-capturing lambdas (closures will be added in a future phase).

use super::dynamic::Value;
use super::function::Function;
use crate::analyzer::typed_expr::Expr;
use crate::evaluator::{EvalError, Evaluator, EvaluatorOptions};
use crate::{
    Vec,
    types::{Type, manager::TypeManager},
};
use bumpalo::Bump;

/// A lambda function value.
///
/// Stores the lambda's type signature, parameters, and body expression.
/// When called, it evaluates the body in a new scope with the parameters bound.
///
/// # Current Limitations
///
/// - Only non-capturing lambdas are supported (closures rejected by analyzer)
/// - No recursive lambdas (would require Y-combinator or named functions)
///
/// # Future Extensions
///
/// - Closure support: Add `upvalues: &'arena [(&'arena str, Value<'types, 'arena>)]`
/// - Multi-value return (for pattern matching)
pub struct LambdaFunction<'types, 'arena> {
    /// The function's type signature (Function type)
    ty: &'types Type<'types>,

    /// Parameter names
    params: &'arena [&'arena str],

    /// The lambda body expression (will be evaluated when called)
    body: &'arena Expr<'types, 'arena>,
    // Future: upvalues for closure support
    // upvalues: &'arena [(&'arena str, Value<'arena, 'arena>)],
}

impl<'types, 'arena> LambdaFunction<'types, 'arena> {
    /// Create a new lambda function.
    ///
    /// # Parameters
    ///
    /// - `ty`: The function's type (must be a Function type)
    /// - `params`: Parameter names
    /// - `body`: The body expression to evaluate when called
    pub fn new(
        ty: &'types Type<'types>,
        params: &'arena [&'arena str],
        body: &'arena Expr<'types, 'arena>,
    ) -> Self {
        debug_assert!(
            matches!(ty, Type::Function { .. }),
            "LambdaFunction type must be Function"
        );

        Self { ty, params, body }
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
    ) -> Result<Value<'types, 'arena>, EvalError> {
        // Build parameter bindings for the lambda call
        let param_bindings: Vec<_> = self
            .params
            .iter()
            .zip(args.iter())
            .map(|(name, value)| (*name, *value))
            .collect();

        // Create an evaluator with the parameter bindings in scope
        let mut evaluator = Evaluator::new(
            EvaluatorOptions::default(),
            arena,
            type_mgr,
            &[],
            param_bindings.as_slice(),
        );

        // Evaluate the body expression
        evaluator.eval_expr(self.body)
    }
}
