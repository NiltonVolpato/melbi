use crate::{String, Vec, errors::MelbiError, format, types::unification::UnificationError};
use alloc::string::ToString;
use bumpalo::Bump;
use core::cell::RefCell;
use hashbrown::{DefaultHashBuilder, HashMap};
//use miette::{Context, Report, SourceSpan};
use thiserror_context::Context;

use crate::{
    analyzer::typed_expr::{Expr, ExprInner, TypedExpr},
    parser::{self, BinaryOp, Span, UnaryOp},
    types::{Type, manager::TypeManager, type_expr_to_type, unification::UnificationContext},
    values::dynamic::Value,
};

pub fn analyze<'types, 'arena>(
    type_manager: &'types TypeManager<'types>,
    arena: &'arena Bump,
    expr: &'arena parser::ParsedExpr<'arena>,
) -> Result<&'arena TypedExpr<'types, 'arena>, MelbiError>
where
    'types: 'arena,
{
    // TODO: Create a temporary TypeManager for analysis only.
    let mut analyzer = Analyzer {
        type_manager,
        arena,
        scopes: Vec::new(),
        context: UnificationContext::new(),
        source: expr.source,
        spans: HashMap::new_in(arena),
        current_span: None, // Initialize to None
    };
    analyzer.spans.clone_from(&expr.spans);
    analyzer.analyze_expr(expr)
}

struct Analyzer<'types, 'arena> {
    type_manager: &'types TypeManager<'types>,
    arena: &'arena Bump,
    scopes: Vec<&'arena Scope<'types, 'arena>>,
    context: UnificationContext<'types>,
    source: &'arena str,
    spans: HashMap<*const parser::Expr<'arena>, Span, DefaultHashBuilder, &'arena Bump>,
    current_span: Option<Span>, // Track current expression span
}

type Scope<'types, 'arena> =
    RefCell<HashMap<&'arena str, &'types Type<'types>, DefaultHashBuilder, &'arena Bump>>;

impl<'types, 'arena> Analyzer<'types, 'arena>
where
    'types: 'arena, // types must live at least as long as arena allocations that point to them
{
    fn analyze_expr(
        &mut self,
        expr: &parser::ParsedExpr<'arena>,
    ) -> Result<&'arena TypedExpr<'types, 'arena>, Report> {
        Ok(self.arena.alloc(TypedExpr {
            source: expr.source,
            expr: self.analyze(&expr.expr)?,
        }))
    }

    fn alloc(
        &mut self,
        ty: &'types Type<'types>,
        inner: ExprInner<'types, 'arena>,
    ) -> &'arena Expr<'types, 'arena> {
        self.arena.alloc(Expr(ty, inner))
    }

    /// Helper to wrap unification errors with current span
    fn with_context<T>(
        &self,
        result: Result<T, UnificationError>,
        message: impl Into<String>,
    ) -> Result<T, Error> {
        if let Some(span) = self.current_span {
            if let Err(err) = &result {
                // Create primary error with span, then attach unification error as context
                Err(Error::TypeChecking {
                    src: self.source.to_string(),
                    span: Some(span),
                    help: Some(message.into()),
                }
                .into())
                //.context(err)
            }
        } else {
            // No span available, just add message context
            let message: String = message.into();
            result.context(message)
        }
    }

    // Helper to create type errors with current span
    fn type_error(&self, message: impl Into<String>) -> Report {
        Report::new(TypeChecking {
            src: self.source.to_string(),
            span: self.current_span,
            help: Some(message.into()),
        })
    }

    // Helper to create type conversion errors with current span
    fn type_conversion_error(&self, message: impl Into<String>) -> Report {
        Report::new(TypeConversion {
            src: self.source.to_string(),
            span: self.current_span.unwrap_or(SourceSpan::from((0, 0))),
            help: message.into(),
        })
    }

    // Helper to expect a specific type
    fn expect_type(
        &self,
        got: &'types Type<'types>,
        expected: &'types Type<'types>,
        context: &str,
    ) -> Result<(), Report> {
        if got != expected {
            return Err(self.type_error(format!(
                "{}: expected {:?}, got {:?}",
                context, expected, got
            )));
        }
        Ok(())
    }

    // Helper to expect numeric type
    fn expect_numeric(&self, ty: &'types Type<'types>, context: &str) -> Result<(), Report> {
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
    ) -> Result<&'arena Expr<'types, 'arena>, Report> {
        // Set current span for this expression
        let old_span = self.current_span;
        self.current_span = self
            .spans
            .get(&(expr as *const parser::Expr<'arena>))
            .map(|s| {
                let offset = s.start;
                let length = s.end - s.start;
                SourceSpan::from((offset, length))
            });

        let result = match expr {
            parser::Expr::Binary { op, left, right } => self
                .analyze_binary(*op, left, right)
                .wrap_err("While analyzing binary expression"),
            parser::Expr::Unary { op, expr } => self
                .analyze_unary(*op, expr)
                .wrap_err("While analyzing unary expression"),
            parser::Expr::Call { callable, args } => self
                .analyze_call(callable, args)
                .wrap_err("While analyzing function call"),
            parser::Expr::Index { value, index } => self
                .analyze_index(value, index)
                .wrap_err("While analyzing index expression"),
            parser::Expr::Field { value, field } => self
                .analyze_field(value, *field)
                .wrap_err("While analyzing field access"),
            parser::Expr::Cast { ty, expr } => self
                .analyze_cast(ty, expr)
                .wrap_err("While analyzing cast expression"),
            parser::Expr::Lambda { params, body } => self
                .analyze_lambda(params, body)
                .wrap_err("While analyzing lambda expression"),
            parser::Expr::If {
                cond,
                then_branch,
                else_branch,
            } => self
                .analyze_if(cond, then_branch, else_branch)
                .wrap_err("While analyzing if expression"),
            parser::Expr::Where { expr, bindings } => self
                .analyze_where(expr, bindings)
                .wrap_err("While analyzing where expression"),
            parser::Expr::Otherwise { primary, fallback } => self
                .analyze_otherwise(primary, fallback)
                .wrap_err("While analyzing 'otherwise' expression"),
            parser::Expr::Record(items) => self
                .analyze_record(items)
                .wrap_err("While analyzing record expression"),
            parser::Expr::Map(items) => self
                .analyze_map(items)
                .wrap_err("While analyzing map expression"),
            parser::Expr::Array(exprs) => self
                .analyze_array(exprs)
                .wrap_err("While analyzing array expression"),
            parser::Expr::FormatStr { strs, exprs } => self
                .analyze_format_str(strs, exprs)
                .wrap_err("While analyzing format string"),
            parser::Expr::Literal(literal) => self
                .analyze_literal(literal)
                .wrap_err("While analyzing literal"),
            parser::Expr::Ident(ident) => self
                .analyze_ident(*ident)
                .wrap_err("While analyzing identifier"),
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
    ) -> Result<&'arena Expr<'types, 'arena>, Report> {
        let left = self.analyze(left)?;
        let right = self.analyze(right)?;

        let result_ty = match op {
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Pow => {
                self.expect_numeric(left.0, "left operand")?;
                self.expect_numeric(right.0, "right operand")?;
                self.expect_type(left.0, right.0, "operands must have same type")?;
                left.0
            }
            BinaryOp::And | BinaryOp::Or => {
                self.expect_type(left.0, self.type_manager.bool(), "left operand")?;
                self.expect_type(right.0, self.type_manager.bool(), "right operand")?;
                self.type_manager.bool()
            }
        };

        Ok(self.alloc(result_ty, ExprInner::Binary { op, left, right }))
    }

    fn analyze_unary(
        &mut self,
        op: UnaryOp,
        expr: &parser::Expr<'arena>,
    ) -> Result<&'arena Expr<'types, 'arena>, Report> {
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
    ) -> Result<&'arena Expr<'types, 'arena>, Report> {
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
    ) -> Result<&'arena Expr<'types, 'arena>, Report> {
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
    ) -> Result<&'arena Expr<'types, 'arena>, Report> {
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

    // TODO: Add cast validation once we have a casting module
    fn analyze_cast(
        &mut self,
        ty: &parser::TypeExpr<'arena>,
        expr: &parser::Expr<'arena>,
    ) -> Result<&'arena Expr<'types, 'arena>, Report> {
        let expr = self.analyze(expr)?;
        let result_ty = match type_expr_to_type(self.type_manager, ty) {
            Ok(ty) => ty,
            Err(e) => return Err(self.type_conversion_error(e.to_string())),
        };
        Ok(self.alloc(result_ty, ExprInner::Cast { expr }))
    }

    fn analyze_lambda(
        &mut self,
        params: &'arena [&'arena str],
        body: &parser::Expr<'arena>,
    ) -> Result<&'arena Expr<'types, 'arena>, Report> {
        let ty = self.type_manager;
        let new_scope = self.arena.alloc(RefCell::new(HashMap::new_in(self.arena)));
        let mut param_types: Vec<&'types Type<'types>> = Vec::new();

        for param in params.iter() {
            let param_ty = ty.fresh_type_var();
            if new_scope.borrow_mut().insert(*param, param_ty).is_some() {
                return Err(self.type_error(format!("Duplicate parameter name '{}'", param)));
            }
            param_types.push(param_ty);
        }

        self.scopes.push(new_scope);
        let body = self.analyze(body)?;
        self.scopes.pop();

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
    ) -> Result<&'arena Expr<'types, 'arena>, Report> {
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
    ) -> Result<&'arena Expr<'types, 'arena>, Report> {
        let new_scope = self.arena.alloc(RefCell::new(HashMap::new_in(self.arena)));
        self.scopes.push(new_scope);

        let mut analyzed_bindings: Vec<(&'arena str, &'arena Expr<'types, 'arena>)> = Vec::new();
        for (k, v) in bindings.iter() {
            let analyzed = self.analyze(v)?;
            if new_scope.borrow_mut().insert(*k, analyzed.0).is_some() {
                return Err(self.type_error(format!("Duplicate binding name '{}'", k)));
            }
            analyzed_bindings.push((*k, analyzed));
        }

        let expr_typed = self.analyze(expr)?;
        self.scopes.pop();

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
    ) -> Result<&'arena Expr<'types, 'arena>, Report> {
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
    ) -> Result<&'arena Expr<'types, 'arena>, Report> {
        let fields: Vec<_> = items
            .iter()
            .map(|(key, value)| {
                let value = self.analyze(value)?;
                Ok::<_, Report>((*key, value))
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
    ) -> Result<&'arena Expr<'types, 'arena>, Report> {
        let elements: Vec<_> = items
            .iter()
            .map(|(key, value)| {
                let key = self.analyze(key)?;
                let value = self.analyze(value)?;
                Ok::<_, Report>((key, value))
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
    ) -> Result<&'arena Expr<'types, 'arena>, Report> {
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
    ) -> Result<&'arena Expr<'types, 'arena>, Report> {
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
    ) -> Result<&'arena Expr<'types, 'arena>, Report> {
        match literal {
            parser::Literal::Int { value, suffix } => {
                if let Some(_) = suffix {
                    todo!();
                }
                let ty = self.type_manager.int();
                let value = Value::int(self.type_manager, *value);
                Ok(self.alloc(ty, ExprInner::Constant(value)))
            }
            parser::Literal::Float { value, suffix } => {
                if let Some(_) = suffix {
                    todo!();
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

    fn analyze_ident(
        &mut self,
        ident: &'arena str,
    ) -> Result<&'arena Expr<'types, 'arena>, Report> {
        for scope in self.scopes.iter().rev() {
            let map = scope.borrow();
            if let Some(ty) = map.get(ident) {
                return Ok(self.alloc(*ty, ExprInner::Ident(ident)));
            }
        }

        Err(self.type_error(format!("Undefined variable: '{}'", ident)))
    }
}
