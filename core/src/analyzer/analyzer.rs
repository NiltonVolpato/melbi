use alloc::string::ToString;
use alloc::sync::Arc;
use bumpalo::Bump;

use crate::{
    String, Vec,
    analyzer::typed_expr::{Expr, ExprInner, TypedExpr},
    casting,
    errors::{Error, ErrorKind},
    format,
    parser::{self, BinaryOp, Span, UnaryOp},
    scope_stack::ScopeStack,
    types::unification,
    types::{Type, manager::TypeManager, type_expr_to_type, unification::UnificationContext},
    values::dynamic::Value,
};

// TODO: Create a temporary TypeManager for analysis only.
pub fn analyze<'types, 'arena>(
    type_manager: &'types TypeManager<'types>,
    arena: &'arena Bump,
    expr: &'arena parser::ParsedExpr<'arena>,
    globals: &[(&'arena str, &'types Type<'types>)],
    variables: &[(&'arena str, &'types Type<'types>)],
) -> Result<&'arena TypedExpr<'types, 'arena>, Error>
where
    'types: 'arena,
{
    // Create annotation map for typed expressions
    // We reuse the same source string since both ParsedExpr and TypedExpr are in the same arena
    let typed_ann = arena.alloc(parser::AnnotatedSource::new(arena, expr.ann.source));

    let mut analyzer = Analyzer {
        type_manager,
        arena,
        scope_stack: ScopeStack::new(),
        context: UnificationContext::new(),
        parsed_ann: expr.ann,
        typed_ann,
        current_span: None, // Initialize to None
    };

    // Push globals scope (constants, packages, functions)
    if !globals.is_empty() {
        analyzer
            .scope_stack
            .push_complete(arena.alloc_slice_copy(globals));
    }

    // Push variables scope (client-provided runtime variables)
    if !variables.is_empty() {
        analyzer
            .scope_stack
            .push_complete(arena.alloc_slice_copy(variables));
    }
    analyzer.analyze_expr(expr)
}

struct Analyzer<'types, 'arena> {
    type_manager: &'types TypeManager<'types>,
    arena: &'arena Bump,
    scope_stack: ScopeStack<'arena, &'types Type<'types>>,
    context: UnificationContext<'types>,
    parsed_ann: &'arena parser::AnnotatedSource<'arena, parser::Expr<'arena>>,
    typed_ann: &'arena parser::AnnotatedSource<'arena, Expr<'types, 'arena>>,
    current_span: Option<Span>, // Track current expression span
}

impl<'types, 'arena> Analyzer<'types, 'arena>
where
    'types: 'arena, // types must live at least as long as arena allocations that point to them
{
    fn analyze_expr(
        &mut self,
        expr: &parser::ParsedExpr<'arena>,
    ) -> Result<&'arena TypedExpr<'types, 'arena>, Error> {
        let typed_expr = self.analyze(&expr.expr)?;
        Ok(self.arena.alloc(TypedExpr {
            expr: typed_expr,
            ann: self.typed_ann,
        }))
    }

    fn alloc(
        &mut self,
        ty: &'types Type<'types>,
        inner: ExprInner<'types, 'arena>,
    ) -> &'arena Expr<'types, 'arena> {
        let typed_expr = self.arena.alloc(Expr(ty, inner));
        // Copy span from current_span to typed annotation
        if let Some(ref span) = self.current_span {
            self.typed_ann.add_span(typed_expr, span.clone());
        }
        typed_expr
    }

    /// Helper to wrap unification errors with current span
    fn with_context<T>(
        &self,
        result: Result<T, unification::Error>,
        message: impl Into<String>,
    ) -> Result<T, Error> {
        result.map_err(|err| {
            // Create primary error with span, then attach unification error as context
            Error {
                kind: Arc::new(ErrorKind::TypeChecking {
                    src: self.parsed_ann.source.to_string(),
                    span: self.current_span.clone(),
                    help: Some(message.into()),
                    unification_context: Some(err),
                }),
                context: Vec::new(),
            }
        })
    }

    // Helper to create type errors with current span
    fn type_error(&self, message: impl Into<String>) -> Error {
        Error {
            kind: Arc::new(ErrorKind::TypeChecking {
                src: self.parsed_ann.source.to_string(),
                span: self.current_span.clone(),
                help: Some(message.into()),
                unification_context: None,
            }),
            context: Vec::new(),
        }
    }

    // Helper to create type conversion errors with current span
    fn type_conversion_error(&self, message: impl Into<String>) -> Error {
        Error {
            kind: Arc::new(ErrorKind::TypeConversion {
                src: self.parsed_ann.source.to_string(),
                span: self.current_span.clone().unwrap_or(Span(0..0)),
                help: message.into(),
            }),
            context: Vec::new(),
        }
    }

    // Helper to expect a specific type
    fn expect_type(
        &self,
        got: &'types Type<'types>,
        expected: &'types Type<'types>,
        context: &str,
    ) -> Result<(), Error> {
        if got != expected {
            return Err(self.type_error(format!(
                "{}: expected {:?}, got {:?}",
                context, expected, got
            )));
        }
        Ok(())
    }

    // Helper to expect numeric type
    fn expect_numeric(&self, ty: &'types Type<'types>, context: &str) -> Result<(), Error> {
        if ty != self.type_manager.int() && ty != self.type_manager.float() {
            return Err(
                self.type_error(format!("{}: expected Int or Float, got {:?}", context, ty))
            );
        }
        Ok(())
    }

    fn analyze(
        &mut self,
        expr: &parser::Expr<'arena>,
    ) -> Result<&'arena Expr<'types, 'arena>, Error> {
        // Set current span for this expression from parsed annotations
        let old_span = self.current_span.clone();
        self.current_span = self.parsed_ann.span_of(expr);

        let result = match expr {
            parser::Expr::Binary { op, left, right } => {
                self.analyze_binary(*op, left, right).map_err(|mut e| {
                    e.context
                        .push("While analyzing binary expression".to_string());
                    e
                })
            }
            parser::Expr::Boolean { op, left, right } => {
                self.analyze_boolean(*op, left, right).map_err(|mut e| {
                    e.context
                        .push("While analyzing boolean expression".to_string());
                    e
                })
            }
            parser::Expr::Unary { op, expr } => self.analyze_unary(*op, expr).map_err(|mut e| {
                e.context
                    .push("While analyzing unary expression".to_string());
                e
            }),
            parser::Expr::Call { callable, args } => {
                self.analyze_call(callable, args).map_err(|mut e| {
                    e.context.push("While analyzing function call".to_string());
                    e
                })
            }
            parser::Expr::Index { value, index } => {
                self.analyze_index(value, index).map_err(|mut e| {
                    e.context
                        .push("While analyzing index expression".to_string());
                    e
                })
            }
            parser::Expr::Field { value, field } => {
                self.analyze_field(value, *field).map_err(|mut e| {
                    e.context.push("While analyzing field access".to_string());
                    e
                })
            }
            parser::Expr::Cast { ty, expr } => self.analyze_cast(ty, expr).map_err(|mut e| {
                e.context
                    .push("While analyzing cast expression".to_string());
                e
            }),
            parser::Expr::Lambda { params, body } => {
                self.analyze_lambda(params, body).map_err(|mut e| {
                    e.context
                        .push("While analyzing lambda expression".to_string());
                    e
                })
            }
            parser::Expr::If {
                cond,
                then_branch,
                else_branch,
            } => self
                .analyze_if(cond, then_branch, else_branch)
                .map_err(|mut e| {
                    e.context.push("While analyzing if expression".to_string());
                    e
                }),
            parser::Expr::Where { expr, bindings } => {
                self.analyze_where(expr, bindings).map_err(|mut e| {
                    e.context
                        .push("While analyzing where expression".to_string());
                    e
                })
            }
            parser::Expr::Otherwise { primary, fallback } => {
                self.analyze_otherwise(primary, fallback).map_err(|mut e| {
                    e.context
                        .push("While analyzing 'otherwise' expression".to_string());
                    e
                })
            }
            parser::Expr::Record(items) => self.analyze_record(items).map_err(|mut e| {
                e.context
                    .push("While analyzing record expression".to_string());
                e
            }),
            parser::Expr::Map(items) => self.analyze_map(items).map_err(|mut e| {
                e.context.push("While analyzing map expression".to_string());
                e
            }),
            parser::Expr::Array(exprs) => self.analyze_array(exprs).map_err(|mut e| {
                e.context
                    .push("While analyzing array expression".to_string());
                e
            }),
            parser::Expr::FormatStr { strs, exprs } => {
                self.analyze_format_str(strs, exprs).map_err(|mut e| {
                    e.context.push("While analyzing format string".to_string());
                    e
                })
            }
            parser::Expr::Literal(literal) => self.analyze_literal(literal).map_err(|mut e| {
                e.context.push("While analyzing literal".to_string());
                e
            }),
            parser::Expr::Ident(ident) => self.analyze_ident(*ident).map_err(|mut e| {
                e.context.push("While analyzing identifier".to_string());
                e
            }),
        };

        // Restore previous span
        self.current_span = old_span;

        result
    }

    fn analyze_binary(
        &mut self,
        op: BinaryOp,
        left: &parser::Expr<'arena>,
        right: &parser::Expr<'arena>,
    ) -> Result<&'arena Expr<'types, 'arena>, Error> {
        let left = self.analyze(left)?;
        let right = self.analyze(right)?;

        let result_ty = match op {
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Pow => {
                self.expect_numeric(left.0, "left operand")?;
                self.expect_numeric(right.0, "right operand")?;
                self.expect_type(left.0, right.0, "operands must have same type")?;
                left.0
            }
        };

        Ok(self.alloc(result_ty, ExprInner::Binary { op, left, right }))
    }

    fn analyze_boolean(
        &mut self,
        op: parser::BoolOp,
        left: &parser::Expr<'arena>,
        right: &parser::Expr<'arena>,
    ) -> Result<&'arena Expr<'types, 'arena>, Error> {
        let left = self.analyze(left)?;
        let right = self.analyze(right)?;

        self.expect_type(left.0, self.type_manager.bool(), "left operand")?;
        self.expect_type(right.0, self.type_manager.bool(), "right operand")?;

        Ok(self.alloc(
            self.type_manager.bool(),
            ExprInner::Boolean { op, left, right },
        ))
    }

    fn analyze_unary(
        &mut self,
        op: UnaryOp,
        expr: &parser::Expr<'arena>,
    ) -> Result<&'arena Expr<'types, 'arena>, Error> {
        let expr = self.analyze(expr)?;
        let result_ty = match op {
            UnaryOp::Neg => {
                self.expect_numeric(expr.0, "unary negation")?;
                expr.0
            }
            UnaryOp::Not => {
                self.expect_type(expr.0, self.type_manager.bool(), "unary not")?;
                self.type_manager.bool()
            }
        };
        Ok(self.alloc(result_ty, ExprInner::Unary { op, expr }))
    }

    fn analyze_call(
        &mut self,
        callable: &parser::Expr<'arena>,
        args: &'arena [&'arena parser::Expr<'arena>],
    ) -> Result<&'arena Expr<'types, 'arena>, Error> {
        let callable = self.analyze(callable)?;
        let args_typed = self
            .arena
            .alloc_slice_try_fill_iter(args.iter().map(|arg| self.analyze(arg)))?;

        let Type::Function { ret, .. } = callable.0 else {
            return Err(self.type_error("Called expression is not a function"));
        };
        let ty = &self.type_manager;
        let types = args_typed
            .iter()
            .map(|arg| ty.alpha_convert(arg.0))
            .collect::<Vec<_>>();

        let f = ty.alpha_convert(callable.0); // function type being called.
        let g = ty.function(&*types, *ret);
        let result = ty.unifies_to(f, g, &mut self.context);
        let result_function = self.with_context(result, "Function argument types do not match")?;

        let Type::Function { ret: result_ty, .. } = result_function else {
            return Err(self.type_error(format!(
                "Expected Function type after unification, got {}",
                result_function
            )));
        };

        Ok(self.alloc(
            result_ty,
            ExprInner::Call {
                callable,
                args: self.arena.alloc_slice_copy(&args_typed),
            },
        ))
    }

    fn analyze_index(
        &mut self,
        value: &parser::Expr<'arena>,
        index: &parser::Expr<'arena>,
    ) -> Result<&'arena Expr<'types, 'arena>, Error> {
        let value = self.analyze(value)?;
        let index = self.analyze(index)?;

        // Determine the result type based on the value type
        let result_ty = match value.0 {
            Type::Array(element_ty) => {
                // Arrays are indexed by integers
                self.expect_type(index.0, self.type_manager.int(), "array index must be Int")?;
                *element_ty
            }
            Type::Map(key_ty, value_ty) => {
                // Maps are indexed by their key type
                self.expect_type(index.0, *key_ty, "map index must match key type")?;
                *value_ty
            }
            _ => {
                return Err(self.type_error(format!(
                    "Cannot index into non-indexable type: {:?}",
                    value.0
                )));
            }
        };

        Ok(self.alloc(result_ty, ExprInner::Index { value, index }))
    }

    fn analyze_field(
        &mut self,
        value: &parser::Expr<'arena>,
        field: &'arena str,
    ) -> Result<&'arena Expr<'types, 'arena>, Error> {
        let value = self.analyze(value)?;

        // Check that value is a record and get the field type
        let result_ty = match value.0 {
            Type::Record(fields) => {
                // Look for the field in the record
                fields
                    .iter()
                    .find(|(name, _)| *name == field)
                    .map(|(_, ty)| *ty)
                    .ok_or_else(|| {
                        self.type_error(format!(
                            "Record does not have field '{}'. Available fields: {}",
                            field,
                            fields
                                .iter()
                                .map(|(n, _)| *n)
                                .collect::<Vec<_>>()
                                .join(", ")
                        ))
                    })?
            }
            _ => {
                return Err(self.type_error(format!(
                    "Cannot access field on non-record type: {:?}",
                    value.0
                )));
            }
        };

        Ok(self.alloc(result_ty, ExprInner::Field { value, field }))
    }

    fn analyze_cast(
        &mut self,
        ty: &parser::TypeExpr<'arena>,
        expr: &parser::Expr<'arena>,
    ) -> Result<&'arena Expr<'types, 'arena>, Error> {
        let analyzed_expr = self.analyze(expr)?;
        let source_type = analyzed_expr.0;
        let target_type = match type_expr_to_type(self.type_manager, ty) {
            Ok(ty) => ty,
            Err(e) => return Err(self.type_conversion_error(e.to_string())),
        };

        // Validate the cast using casting library
        casting::validate_cast(source_type, target_type)
            .map_err(|err| self.type_error(err.to_string()))?;

        // If cast is valid, create the cast expression
        // TODO(effects): Track whether cast is fallible
        Ok(self.alloc(
            target_type,
            ExprInner::Cast {
                expr: analyzed_expr,
            },
        ))
    }

    fn analyze_lambda(
        &mut self,
        params: &'arena [&'arena str],
        body: &parser::Expr<'arena>,
    ) -> Result<&'arena Expr<'types, 'arena>, Error> {
        let ty = self.type_manager;

        // Push incomplete scope with parameter names
        self.scope_stack
            .push_incomplete(self.arena, params)
            .map_err(|e| self.type_error(format!("Duplicate parameter name: {}", e.0)))?;

        // Bind each parameter to a fresh type variable
        let mut param_types: Vec<&'types Type<'types>> = Vec::new();
        for param in params.iter() {
            let param_ty = ty.fresh_type_var();
            self.scope_stack
                .bind_in_current(*param, param_ty)
                .map_err(|e| self.type_error(format!("Failed to bind parameter: {:?}", e)))?;
            param_types.push(param_ty);
        }

        let body = self.analyze(body)?;

        self.scope_stack
            .pop_incomplete()
            .map_err(|e| self.type_error(format!("Failed to pop scope: {:?}", e)))?;

        let result_ty = ty.function(self.arena.alloc_slice_copy(param_types.as_slice()), body.0);
        Ok(self.alloc(
            result_ty,
            ExprInner::Lambda {
                params: self.arena.alloc_slice_copy(params),
                body,
            },
        ))
    }

    fn analyze_if(
        &mut self,
        cond: &parser::Expr<'arena>,
        then_branch: &parser::Expr<'arena>,
        else_branch: &parser::Expr<'arena>,
    ) -> Result<&'arena Expr<'types, 'arena>, Error> {
        let cond = self.analyze(cond)?;
        let then_branch = self.analyze(then_branch)?;
        let else_branch = self.analyze(else_branch)?;

        self.expect_type(
            cond.0,
            self.type_manager.bool(),
            "If condition must be boolean",
        )?;

        // Separate the unification call to avoid borrowing issues
        let unify_result =
            self.type_manager
                .unifies_to(then_branch.0, else_branch.0, &mut self.context);
        let result_ty = self.with_context(unify_result, "Branches have incompatible types")?;

        Ok(self.alloc(
            result_ty,
            ExprInner::If {
                cond,
                then_branch,
                else_branch,
            },
        ))
    }

    fn analyze_where(
        &mut self,
        expr: &parser::Expr<'arena>,
        bindings: &'arena [(&'arena str, &'arena parser::Expr<'arena>)],
    ) -> Result<&'arena Expr<'types, 'arena>, Error> {
        // Extract binding names
        let names: Vec<&'arena str> = bindings.iter().map(|(name, _)| *name).collect();

        // Push incomplete scope with all binding names
        self.scope_stack
            .push_incomplete(self.arena, &names)
            .map_err(|e| self.type_error(format!("Duplicate binding name: {}", e.0)))?;

        // Analyze and bind each expression sequentially
        let mut analyzed_bindings: Vec<(&'arena str, &'arena Expr<'types, 'arena>)> = Vec::new();
        for (name, value_expr) in bindings.iter() {
            let analyzed = self.analyze(value_expr)?;
            self.scope_stack
                .bind_in_current(*name, analyzed.0)
                .map_err(|e| self.type_error(format!("Failed to bind in where: {:?}", e)))?;
            analyzed_bindings.push((*name, analyzed));
        }

        let expr_typed = self.analyze(expr)?;

        self.scope_stack
            .pop_incomplete()
            .map_err(|e| self.type_error(format!("Failed to pop scope: {:?}", e)))?;

        Ok(self.alloc(
            expr_typed.0,
            ExprInner::Where {
                expr: expr_typed,
                bindings: self.arena.alloc_slice_copy(&analyzed_bindings),
            },
        ))
    }

    fn analyze_otherwise(
        &mut self,
        primary: &parser::Expr<'arena>,
        fallback: &parser::Expr<'arena>,
    ) -> Result<&'arena Expr<'types, 'arena>, Error> {
        let primary = self.analyze(primary)?;
        let fallback = self.analyze(fallback)?;

        // TODO(effects): When effect system is implemented:
        // - Check that primary has an error effect (e.g., Type!)
        // - Strip the error effect from the result type
        // - Reject cases like `1 otherwise 0` where primary cannot fail
        // For now, we only verify that both branches have compatible types.

        // Separate the unification call to avoid borrowing issues
        let unify_result = self
            .type_manager
            .unifies_to(primary.0, fallback.0, &mut self.context);
        let result_ty = self.with_context(
            unify_result,
            "Primary and fallback branches must have compatible types",
        )?;

        Ok(self.alloc(result_ty, ExprInner::Otherwise { primary, fallback }))
    }

    fn analyze_record(
        &mut self,
        items: &'arena [(&'arena str, &'arena parser::Expr<'arena>)],
    ) -> Result<&'arena Expr<'types, 'arena>, Error> {
        let fields: Vec<_> = items
            .iter()
            .map(|(key, value)| {
                let value = self.analyze(value)?;
                Ok::<_, Error>((*key, value))
            })
            .collect::<Result<_, _>>()?;

        // Create the record type from the analyzed fields
        let field_types: Vec<(&str, &'types Type<'types>)> =
            fields.iter().map(|(name, expr)| (*name, expr.0)).collect();
        let result_ty = self.type_manager.record(&field_types);

        Ok(self.alloc(
            result_ty,
            ExprInner::Record {
                fields: self.arena.alloc_slice_copy(&fields),
            },
        ))
    }

    fn analyze_map(
        &mut self,
        items: &'arena [(&'arena parser::Expr<'arena>, &'arena parser::Expr<'arena>)],
    ) -> Result<&'arena Expr<'types, 'arena>, Error> {
        let elements: Vec<_> = items
            .iter()
            .map(|(key, value)| {
                let key = self.analyze(key)?;
                let value = self.analyze(value)?;
                Ok::<_, Error>((key, value))
            })
            .collect::<Result<_, _>>()?;

        // Unify all keys to ensure they have the same type
        let mut key_ty = self.type_manager.fresh_type_var();
        for (key, _) in &elements {
            let unification_result = self
                .type_manager
                .unifies_to(key.0, key_ty, &mut self.context);
            key_ty = self.with_context(unification_result, "Map keys must have the same type")?;
        }

        // Unify all values to ensure they have the same type
        let mut value_ty = self.type_manager.fresh_type_var();
        for (_, value) in &elements {
            let unification_result =
                self.type_manager
                    .unifies_to(value.0, value_ty, &mut self.context);
            value_ty =
                self.with_context(unification_result, "Map values must have the same type")?;
        }

        let result_ty = self.type_manager.map(key_ty, value_ty);
        Ok(self.alloc(
            result_ty,
            ExprInner::Map {
                elements: self.arena.alloc_slice_copy(&elements),
            },
        ))
    }

    fn analyze_array(
        &mut self,
        exprs: &'arena [&'arena parser::Expr<'arena>],
    ) -> Result<&'arena Expr<'types, 'arena>, Error> {
        let elements: Vec<_> = exprs
            .iter()
            .map(|expr| self.analyze(expr))
            .collect::<Result<_, _>>()?;
        let mut element_ty = self.type_manager.fresh_type_var();
        for element in &elements {
            let unification_result =
                self.type_manager
                    .unifies_to(element.0, element_ty, &mut self.context);
            element_ty =
                self.with_context(unification_result, "Array elements must have the same type")?;
        }
        let result_ty = self.type_manager.array(element_ty);
        Ok(self.alloc(
            result_ty,
            ExprInner::Array {
                elements: self.arena.alloc_slice_copy(&elements),
            },
        ))
    }

    fn analyze_format_str(
        &mut self,
        strs: &'arena [&'arena str],
        exprs: &'arena [&'arena parser::Expr<'arena>],
    ) -> Result<&'arena Expr<'types, 'arena>, Error> {
        let exprs_typed: Vec<_> = exprs
            .iter()
            .map(|expr| self.analyze(expr))
            .collect::<Result<_, _>>()?;

        // Check that all expressions are formattable (not functions)
        for expr in &exprs_typed {
            if matches!(expr.0, Type::Function { .. }) {
                return Err(self.type_error(format!(
                    "Cannot format function type in format string: {:?}",
                    expr.0
                )));
            }
        }

        Ok(self.alloc(
            self.type_manager.str(),
            ExprInner::FormatStr {
                strs: self.arena.alloc_slice_copy(strs),
                exprs: self.arena.alloc_slice_copy(&exprs_typed),
            },
        ))
    }

    fn analyze_literal(
        &mut self,
        literal: &parser::Literal<'arena>,
    ) -> Result<&'arena Expr<'types, 'arena>, Error> {
        match literal {
            parser::Literal::Int { value, suffix } => {
                if let Some(_suffix) = suffix {
                    return Err(self.type_error(
                        "Integer suffixes are not yet supported. \
                         In the future, suffixes will support units of measurement (e.g., 10`MB`, 5`seconds`)".to_string()
                    ));
                }
                let ty = self.type_manager.int();
                let value = Value::int(self.type_manager, *value);
                Ok(self.alloc(ty, ExprInner::Constant(value)))
            }
            parser::Literal::Float { value, suffix } => {
                if let Some(_suffix) = suffix {
                    return Err(self.type_error(
                        "Float suffixes are not yet supported. \
                         In the future, suffixes will support units of measurement (e.g., 3.14`meters`, 2.5`kg`)".to_string()
                    ));
                }
                let ty = self.type_manager.float();
                let value = Value::float(self.type_manager, *value);
                Ok(self.alloc(ty, ExprInner::Constant(value)))
            }
            parser::Literal::Bool(value) => {
                let ty = self.type_manager.bool();
                let value = Value::bool(self.type_manager, *value);
                Ok(self.alloc(ty, ExprInner::Constant(value)))
            }
            parser::Literal::Str(value) => {
                let ty = self.type_manager.str();
                let value = Value::str(self.arena, ty, value);
                Ok(self.alloc(ty, ExprInner::Constant(value)))
            }
            parser::Literal::Bytes(value) => {
                let ty = self.type_manager.bytes();
                let value = Value::bytes(self.arena, ty, value);
                Ok(self.alloc(ty, ExprInner::Constant(value)))
            }
        }
    }

    fn analyze_ident(&mut self, ident: &'arena str) -> Result<&'arena Expr<'types, 'arena>, Error> {
        if let Some(ty) = self.scope_stack.lookup(ident) {
            return Ok(self.alloc(*ty, ExprInner::Ident(ident)));
        }

        Err(self.type_error(format!("Undefined variable: '{}'", ident)))
    }
}
