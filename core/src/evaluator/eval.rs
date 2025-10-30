//! Core evaluation logic.

use crate::{
    analyzer::typed_expr::TypedExpr, evaluator::EvalError, scope_stack::ScopeStack,
    types::manager::TypeManager, values::dynamic::Value,
};
use bumpalo::Bump;

/// Evaluator for type-checked expressions.
pub(super) struct Evaluator<'types, 'arena> {
    type_manager: &'types TypeManager<'types>,
    arena: &'arena Bump,
    scope_stack: ScopeStack<'arena, Value<'types, 'arena>>,
    depth: usize,
    max_depth: usize,
}

impl<'types, 'arena> Evaluator<'types, 'arena>
where
    'types: 'arena,
{
    /// Create a new evaluator with the given depth limit.
    pub(super) fn new(
        type_manager: &'types TypeManager<'types>,
        arena: &'arena Bump,
        globals: &[(&'arena str, Value<'types, 'arena>)],
        variables: &[(&'arena str, Value<'types, 'arena>)],
        max_depth: usize,
    ) -> Self {
        let mut scope_stack = ScopeStack::new();

        // Push globals scope (constants, packages, functions)
        if !globals.is_empty() {
            scope_stack.push_complete(arena.alloc_slice_copy(globals));
        }

        // Push variables scope (client-provided runtime variables)
        if !variables.is_empty() {
            scope_stack.push_complete(arena.alloc_slice_copy(variables));
        }

        Self {
            type_manager,
            arena,
            scope_stack,
            depth: 0,
            max_depth,
        }
    }

    /// Evaluate a type-checked expression.
    pub(super) fn eval(
        &mut self,
        expr: &'arena TypedExpr<'types, 'arena>,
    ) -> Result<Value<'types, 'arena>, EvalError> {
        // Check depth before recursing
        if self.depth >= self.max_depth {
            return Err(EvalError::StackOverflow {
                depth: self.depth,
                max_depth: self.max_depth,
            });
        }

        self.depth += 1;
        let result = self.eval_expr(expr.expr);
        self.depth -= 1;

        result
    }

    /// Evaluate an expression node.
    fn eval_expr(
        &mut self,
        expr: &'arena crate::analyzer::typed_expr::Expr<'types, 'arena>,
    ) -> Result<Value<'types, 'arena>, EvalError> {
        // Check depth before recursing
        if self.depth >= self.max_depth {
            return Err(EvalError::StackOverflow {
                depth: self.depth,
                max_depth: self.max_depth,
            });
        }

        self.depth += 1;
        let result = self.eval_expr_inner(expr);
        self.depth -= 1;

        result
    }

    /// Inner evaluation logic (no depth tracking).
    fn eval_expr_inner(
        &mut self,
        expr: &'arena crate::analyzer::typed_expr::Expr<'types, 'arena>,
    ) -> Result<Value<'types, 'arena>, EvalError> {
        use crate::analyzer::typed_expr::ExprInner;

        match &expr.1 {
            ExprInner::Constant(value) => {
                // Constants are already values, just return them
                Ok(*value)
            }

            ExprInner::Ident(name) => {
                // Look up variable in scope stack
                match self.scope_stack.lookup(name) {
                    Some(value) => Ok(*value),
                    None => {
                        // This should never happen if the expression was type-checked
                        debug_assert!(
                            false,
                            "Undefined variable '{}' - analyzer should have caught this",
                            name
                        );
                        // In release mode, we can't panic, so return a dummy value
                        // This is safe because the expression is guaranteed to be well-typed
                        unreachable!("Undefined variable '{}' in type-checked expression", name)
                    }
                }
            }

            ExprInner::Binary { op, left, right } => {
                use crate::Type;

                // Recursively evaluate operands (direct call to eval_expr, not eval)
                let left_val = self.eval_expr(left)?;
                let right_val = self.eval_expr(right)?;

                // Dispatch based on type (we know both operands have the same type after type-checking)
                match left_val.ty {
                    Type::Int => {
                        let l = left_val.as_int().expect("Type-checked as Int");
                        let r = right_val.as_int().expect("Type-checked as Int");
                        let result = super::operators::eval_binary_int(*op, l, r, None)?;
                        Ok(Value::int(self.type_manager, result))
                    }
                    Type::Float => {
                        let l = left_val.as_float().expect("Type-checked as Float");
                        let r = right_val.as_float().expect("Type-checked as Float");
                        let result = super::operators::eval_binary_float(*op, l, r);
                        Ok(Value::float(self.type_manager, result))
                    }
                    _ => {
                        // Type checker should have caught this
                        debug_assert!(false, "Binary operator on non-numeric type");
                        unreachable!("Binary operator on invalid type in type-checked expression")
                    }
                }
            }

            _ => {
                // TODO: Implement remaining expression types in future milestones
                todo!("Expression type not yet implemented: {:?}", expr.1)
            }
        }
    }
}
