//! Core evaluation logic.

use crate::{
    Type, analyzer::typed_expr::TypedExpr, evaluator::EvalError, parser::BoolOp,
    scope_stack::ScopeStack, types::manager::TypeManager, values::dynamic::Value,
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

            ExprInner::Boolean { op, left, right } => {
                // Evaluate left operand
                let left_val = self.eval_expr(left)?;
                let left_bool = left_val.as_bool().expect("Type-checked as Bool");

                // Short-circuit evaluation
                match op {
                    BoolOp::And => {
                        // If left is false, return false without evaluating right
                        if !left_bool {
                            return Ok(Value::bool(self.type_manager, false));
                        }
                        // Left is true, return right's value
                        let right_val = self.eval_expr(right)?;
                        let right_bool = right_val.as_bool().expect("Type-checked as Bool");
                        Ok(Value::bool(self.type_manager, right_bool))
                    }
                    BoolOp::Or => {
                        // If left is true, return true without evaluating right
                        if left_bool {
                            return Ok(Value::bool(self.type_manager, true));
                        }
                        // Left is false, return right's value
                        let right_val = self.eval_expr(right)?;
                        let right_bool = right_val.as_bool().expect("Type-checked as Bool");
                        Ok(Value::bool(self.type_manager, right_bool))
                    }
                }
            }

            ExprInner::Where { expr, bindings } => {
                // Extract binding names
                let names: crate::Vec<&'arena str> =
                    bindings.iter().map(|(name, _)| *name).collect();

                // Push incomplete scope with all binding names
                // This allows sequential binding (later bindings can reference earlier ones)
                self.scope_stack
                    .push_incomplete(self.arena, &names)
                    .expect("Duplicate binding in where - analyzer should have caught this");

                // Evaluate and bind each expression sequentially
                for (name, value_expr) in bindings.iter() {
                    let value = self.eval_expr(value_expr)?;
                    self.scope_stack
                        .bind_in_current(name, value)
                        .expect("Failed to bind in where - analyzer should have caught this");
                }

                // Evaluate the body expression (has access to all bindings)
                let result = self.eval_expr(expr)?;

                // Pop the scope
                self.scope_stack
                    .pop_incomplete()
                    .expect("Failed to pop where scope - internal error");

                Ok(result)
            }

            ExprInner::Record { fields } => {
                // Get field names from the type (which has the right lifetime 'types)
                let Type::Record(field_types) = expr.0 else {
                    unreachable!("Record expression must have Record type")
                };

                // Evaluate fields in type order (sorted), not AST order
                // Build a map from field name to field expression for quick lookup
                let mut field_map = hashbrown::HashMap::new();
                for (name, expr) in fields.iter() {
                    field_map.insert(*name, expr);
                }

                // Evaluate in type order and collect values
                let mut field_values_temp: crate::Vec<(&'types str, Value<'types, 'arena>)> =
                    crate::Vec::new();

                for (field_name, _field_ty) in field_types.iter() {
                    // Look up the expression for this field
                    let field_expr = field_map
                        .get(field_name)
                        .expect("Field in type but not in AST - analyzer should have caught this");

                    // Evaluate the field expression
                    let field_value = self.eval_expr(field_expr)?;

                    // Use field name from type (has 'types lifetime)
                    field_values_temp.push((*field_name, field_value));
                }

                // Allocate in arena to get proper lifetime
                let field_values = self.arena.alloc_slice_copy(&field_values_temp);

                // Construct record value (fields are now in sorted order)
                Ok(Value::record(self.arena, expr.0, field_values)
                    .expect("Record construction failed - analyzer should have validated types"))
            }

            ExprInner::Field { value, field } => {
                // Evaluate the record expression
                let record_value = self.eval_expr(value)?;

                // Extract as record
                let record = record_value
                    .as_record()
                    .expect("Field access on non-record - analyzer should have caught this");

                // Look up field by name
                Ok(record
                    .get(field)
                    .expect("Field not found in record - analyzer should have caught this"))
            }

            ExprInner::Unary { op, expr: operand } => {
                use crate::Type;

                // Evaluate the operand
                let operand_val = self.eval_expr(operand)?;

                // Dispatch based on type
                match operand_val.ty {
                    Type::Int => {
                        let val = operand_val.as_int().expect("Type-checked as Int");
                        let result = super::operators::eval_unary_int(*op, val);
                        Ok(Value::int(self.type_manager, result))
                    }
                    Type::Float => {
                        let val = operand_val.as_float().expect("Type-checked as Float");
                        let result = super::operators::eval_unary_float(*op, val);
                        Ok(Value::float(self.type_manager, result))
                    }
                    Type::Bool => {
                        let val = operand_val.as_bool().expect("Type-checked as Bool");
                        let result = super::operators::eval_unary_bool(*op, val);
                        Ok(Value::bool(self.type_manager, result))
                    }
                    _ => {
                        // Type checker should have caught this
                        debug_assert!(false, "Unary operator on invalid type");
                        unreachable!("Unary operator on invalid type in type-checked expression")
                    }
                }
            }

            ExprInner::If {
                cond,
                then_branch,
                else_branch,
            } => {
                // Evaluate the condition
                let cond_val = self.eval_expr(cond)?;
                let cond_bool = cond_val.as_bool().expect("Type-checked as Bool");

                // Evaluate the appropriate branch (lazy evaluation)
                if cond_bool {
                    self.eval_expr(then_branch)
                } else {
                    self.eval_expr(else_branch)
                }
            }

            _ => {
                // TODO: Implement remaining expression types in future milestones
                todo!("Expression type not yet implemented: {:?}", expr.1)
            }
        }
    }
}
