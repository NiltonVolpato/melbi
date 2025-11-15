use alloc::collections::BTreeSet;
use alloc::rc::Rc;
use alloc::string::ToString;
use bumpalo::Bump;
use core::cell::RefCell;

use crate::{
    String, Vec,
    analyzer::error::{TypeError, TypeErrorKind},
    analyzer::typed_expr::{Expr, ExprInner, TypedExpr},
    format,
    parser::{self, BinaryOp, ComparisonOp, Span, UnaryOp},
    scope_stack::{self, ScopeStack},
    types::{
        Type, TypeClassResolver, TypeScheme,
        manager::TypeManager,
        traits::{TypeKind, TypeView},
        type_expr_to_type,
        unification::Unification,
    },
};

// TODO: Create a temporary TypeManager for analysis only.
pub fn analyze<'types, 'arena>(
    type_manager: &'types TypeManager<'types>,
    arena: &'arena Bump,
    expr: &'arena parser::ParsedExpr<'arena>,
    globals: &[(&'arena str, &'types Type<'types>)],
    variables: &[(&'arena str, &'types Type<'types>)],
) -> Result<&'arena TypedExpr<'types, 'arena>, TypeError> {
    // Create annotation map for typed expressions
    // We reuse the same source string since both ParsedExpr and TypedExpr are in the same arena
    let typed_ann = arena.alloc(parser::AnnotatedSource::new(arena, expr.ann.source));

    let mut analyzer = Analyzer {
        type_manager,
        arena,
        scope_stack: ScopeStack::new(),
        unification: Unification::new(type_manager),
        type_class_resolver: TypeClassResolver::new(),
        parsed_ann: expr.ann,
        typed_ann,
        current_span: None, // Initialize to None
        env_vars_stack: Vec::new(),
    };

    // Push globals scope (constants, packages, functions)
    if !globals.is_empty() {
        // Wrap each type in a monomorphic TypeScheme
        // TODO: Accept TypeScheme as an argument.
        let bindings: Vec<(&'arena str, TypeScheme<'types>)> = globals
            .iter()
            .map(|(name, ty)| {
                let empty_quantified = type_manager.alloc_u16_slice(&[]);
                (*name, TypeScheme::new(empty_quantified, ty))
            })
            .collect();
        let bindings_slice = arena.alloc_slice_fill_iter(bindings.into_iter());
        analyzer
            .scope_stack
            .push(scope_stack::CompleteScope::from_sorted(bindings_slice));
    }

    // Push variables scope (client-provided runtime variables)
    if !variables.is_empty() {
        // Wrap each type in a monomorphic TypeScheme
        // TODO: Accept TypeScheme as an argument.
        let bindings: Vec<(&'arena str, TypeScheme<'types>)> = variables
            .iter()
            .map(|(name, ty)| {
                let empty_quantified = type_manager.alloc_u16_slice(&[]);
                (*name, TypeScheme::new(empty_quantified, ty))
            })
            .collect();
        let bindings_slice = arena.alloc_slice_fill_iter(bindings.into_iter());
        analyzer
            .scope_stack
            .push(scope_stack::CompleteScope::from_sorted(bindings_slice));
    }
    let result = analyzer.analyze_expr(expr)?;

    // Check all type class constraints after unification
    analyzer.finalize_constraints()?;

    Ok(&*result)
}

struct Analyzer<'types, 'arena> {
    type_manager: &'types TypeManager<'types>,
    arena: &'arena Bump,
    scope_stack: ScopeStack<'arena, TypeScheme<'types>>,
    unification: Unification<'types, &'types TypeManager<'types>>,
    type_class_resolver: TypeClassResolver<'types>,
    parsed_ann: &'arena parser::AnnotatedSource<'arena, parser::Expr<'arena>>,
    typed_ann: &'arena parser::AnnotatedSource<'arena, Expr<'types, 'arena>>,
    current_span: Option<Span>, // Track current expression span
    /// Stack of environment type variables from outer scopes.
    /// Each element is a set of type variables that should not be generalized.
    /// When entering a lambda or let-binding, we push the free vars from parameters.
    /// When exiting, we pop them.
    env_vars_stack: Vec<hashbrown::HashSet<u16>>,
}

impl<'types, 'arena> Analyzer<'types, 'arena> {
    fn analyze_expr(
        &mut self,
        expr: &parser::ParsedExpr<'arena>,
    ) -> Result<&'arena mut TypedExpr<'types, 'arena>, TypeError> {
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
    ) -> &'arena mut Expr<'types, 'arena> {
        let typed_expr = self
            .arena
            .alloc(Expr(self.unification.fully_resolve(ty), inner));
        // Copy span from current_span to typed annotation
        if let Some(ref span) = self.current_span {
            self.typed_ann.add_span(typed_expr, span.clone());
        }
        typed_expr
    }

    /// Helper to wrap unification errors with current span
    fn with_context<T>(
        &self,
        result: Result<T, crate::types::unification::Error>,
        message: impl Into<String>,
    ) -> Result<T, TypeError> {
        result.map_err(|err| {
            let span = self.current_span.clone().unwrap_or(Span(0..0));
            // Note: message parameter could be used to add context in the future
            let _ = message.into();
            TypeError::from_unification_error(err, span, self.get_source())
        })
    }

    // Get current span or default
    fn get_span(&self) -> Span {
        self.current_span.clone().unwrap_or(Span(0..0))
    }

    // Get source code as String
    fn get_source(&self) -> String {
        self.parsed_ann.source.to_string()
    }

    // Helper for internal/unexpected errors (invariant violations)
    fn internal_error(&self, message: impl Into<String>) -> TypeError {
        TypeError::new(
            TypeErrorKind::Other {
                message: message.into(),
                span: self.get_span(),
            },
            self.get_source(),
        )
    }

    // Helper to expect a specific type
    fn expect_type(
        &mut self,
        got: &'types Type<'types>,
        expected: &'types Type<'types>,
        context: &str,
    ) -> Result<&'types Type<'types>, TypeError> {
        let unification_result = self.unification.unifies_to(got, expected);
        self.with_context(
            unification_result,
            format!("{}: expected {:?} = {:?}", context, expected, got),
        )
    }

    // Helper to expect numeric type
    fn expect_numeric(&self, ty: &'types Type<'types>, _context: &str) -> Result<(), TypeError> {
        match ty {
            Type::Int | Type::Float => Ok(()),
            _ => Err(TypeError::new(
                TypeErrorKind::ConstraintViolation {
                    ty: format!("{}", ty),
                    type_class: "Numeric".to_string(),
                    span: self.get_span(),
                },
                self.get_source(),
            )),
        }
    }

    // Helper to expect Ord type (supports ordering comparisons)
    fn expect_ord(&self, ty: &'types Type<'types>, _context: &str) -> Result<(), TypeError> {
        match ty {
            Type::Int | Type::Float | Type::Str | Type::Bytes => Ok(()),
            _ => Err(TypeError::new(
                TypeErrorKind::ConstraintViolation {
                    ty: format!("{}", ty),
                    type_class: "Ord".to_string(),
                    span: self.get_span(),
                },
                self.get_source(),
            )),
        }
    }
    // Finalize type checking by resolving all type class constraints
    fn finalize_constraints(&mut self) -> Result<(), TypeError> {
        self.type_class_resolver
            .resolve_all(&mut self.unification)
            .map_err(|errors| {
                // For now, just report the first error
                // In the future, we can report all errors
                if let Some(first_error) = errors.first() {
                    TypeError::new(
                        TypeErrorKind::Other {
                            message: first_error.message.clone(),
                            span: first_error.span.clone(),
                        },
                        self.get_source(),
                    )
                } else {
                    self.internal_error("Type class constraint error".to_string())
                }
            })
    }

    /// Get the current environment type variables (union of all sets in the stack).
    /// These are type variables that should NOT be generalized in let-polymorphism.
    fn get_env_vars(&self) -> hashbrown::HashSet<u16> {
        let mut result = hashbrown::HashSet::new();
        for set in &self.env_vars_stack {
            result.extend(set);
        }
        result
    }

    fn analyze(
        &mut self,
        expr: &'arena parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        // Set current span for this expression from parsed annotations
        let old_span = self.current_span.clone();
        self.current_span = self.parsed_ann.span_of(expr);

        let result = match expr {
            parser::Expr::Binary { op, left, right } => self.analyze_binary(*op, left, right),
            parser::Expr::Boolean { op, left, right } => self.analyze_boolean(*op, left, right),
            parser::Expr::Comparison { op, left, right } => {
                self.analyze_comparison(*op, left, right)
            }
            parser::Expr::Unary { op, expr } => self.analyze_unary(*op, expr),
            parser::Expr::Call { callable, args } => self.analyze_call(callable, args),
            parser::Expr::Index { value, index } => self.analyze_index(value, index),
            parser::Expr::Field { value, field } => self.analyze_field(value, *field),
            parser::Expr::Cast { ty, expr } => self.analyze_cast(ty, expr),
            parser::Expr::Lambda { params, body } => self.analyze_lambda(params, body),
            parser::Expr::If {
                cond,
                then_branch,
                else_branch,
            } => self.analyze_if(cond, then_branch, else_branch),
            parser::Expr::Where { expr, bindings } => self.analyze_where(expr, bindings),
            parser::Expr::Otherwise { primary, fallback } => {
                self.analyze_otherwise(primary, fallback)
            }
            parser::Expr::Record(items) => self.analyze_record(items),
            parser::Expr::Map(items) => self.analyze_map(items),
            parser::Expr::Array(exprs) => self.analyze_array(exprs),
            parser::Expr::FormatStr { strs, exprs } => self.analyze_format_str(strs, exprs),
            parser::Expr::Literal(literal) => self.analyze_literal(literal),
            parser::Expr::Ident(ident) => self.analyze_ident(*ident),
        };

        // Restore previous span
        self.current_span = old_span;

        result
    }

    fn analyze_binary(
        &mut self,
        op: BinaryOp,
        left: &'arena parser::Expr<'arena>,
        right: &'arena parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        let left = self.analyze(left)?;
        let right = self.analyze(right)?;

        // Unify left and right to determine result type
        let result_ty = self.expect_type(left.0, right.0, "operands must have same type")?;

        // Add relational Numeric constraint: Numeric(left, right, result)
        // The constraint resolver will verify and unify based on the numeric instance:
        //   - (Int, Int) => Int
        //   - (Float, Float) => Float
        self.type_class_resolver.add_numeric_constraint(
            left.0,
            right.0,
            result_ty,
            self.get_span(),
        );

        Ok(self.alloc(result_ty, ExprInner::Binary { op, left, right }))
    }

    fn analyze_boolean(
        &mut self,
        op: parser::BoolOp,
        left: &'arena parser::Expr<'arena>,
        right: &'arena parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        let left = self.analyze(left)?;
        let right = self.analyze(right)?;

        self.expect_type(left.0, self.type_manager.bool(), "left operand")?;
        self.expect_type(right.0, self.type_manager.bool(), "right operand")?;

        Ok(self.alloc(
            self.type_manager.bool(),
            ExprInner::Boolean { op, left, right },
        ))
    }

    fn analyze_comparison(
        &mut self,
        op: ComparisonOp,
        left: &'arena parser::Expr<'arena>,
        right: &'arena parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        let left = self.analyze(left)?;
        let right = self.analyze(right)?;

        // For equality operators (== and !=), any types can be compared
        // For ordering operators (<, >, <=, >=), operands must support Ord (Int, Float, Str, Bytes)
        match op {
            ComparisonOp::Eq | ComparisonOp::Neq => {
                // Equality: just ensure both operands have the same type
                self.expect_type(left.0, right.0, "operands must have same type")?;
            }
            ComparisonOp::Lt | ComparisonOp::Gt | ComparisonOp::Le | ComparisonOp::Ge => {
                // Ordering: operands must support Ord and have the same type

                // Unify left and right
                self.expect_type(left.0, right.0, "operands must have same type")?;

                // Add Ord constraints - Ord is a simple predicate, not relational
                let span = self.get_span();
                self.type_class_resolver.add_ord_constraint(left.0, span.clone());
                self.type_class_resolver.add_ord_constraint(right.0, span);

                // Note: No need to check immediately - finalize_constraints will check
            }
        }

        Ok(self.alloc(
            self.type_manager.bool(),
            ExprInner::Comparison { op, left, right },
        ))
    }

    fn analyze_unary(
        &mut self,
        op: UnaryOp,
        expr: &'arena parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        let expr = self.analyze(expr)?;

        match op {
            UnaryOp::Neg => {
                // Negation: operand must be numeric
                // Add relational Numeric constraint: Numeric(expr, expr, expr)
                // (negation preserves type: -Int => Int, -Float => Float)
                let span = self.get_span();
                self.type_class_resolver.add_numeric_constraint(
                    expr.0,
                    expr.0,
                    expr.0,
                    span,
                );

                Ok(self.alloc(expr.0, ExprInner::Unary { op, expr }))
            }
            UnaryOp::Not => {
                // Logical not: operand must be Bool
                self.expect_type(expr.0, self.type_manager.bool(), "operand must be Bool")?;

                Ok(self.alloc(
                    self.type_manager.bool(),
                    ExprInner::Unary { op, expr },
                ))
            }
        }
    }

    fn analyze_call(
        &mut self,
        callable: &'arena parser::Expr<'arena>,
        args: &'arena [&'arena parser::Expr<'arena>],
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        let callable = self.analyze(callable)?;

        // Analyze all arguments
        let args_typed: Vec<&'arena mut Expr<'types, 'arena>> = args
            .iter()
            .map(|arg| self.analyze(arg))
            .collect::<Result<_, _>>()?;

        // Create fresh type variable for the return type
        let ret_ty = self.type_manager.fresh_type_var();

        // Construct the expected function type based on argument types
        let arg_types: Vec<&'types Type<'types>> =
            args_typed.iter().map(|arg| arg.0).collect();
        let expected_fn_ty = self.type_manager.function(&arg_types, ret_ty);

        // Unify callable with the expected function type
        self.expect_type(
            callable.0,
            expected_fn_ty,
            "function call type mismatch",
        )?;

        Ok(self.alloc(
            ret_ty,
            ExprInner::Call {
                callable,
                args: self.arena.alloc_slice_fill_iter(args_typed.into_iter().map(|e| &*e)),
            },
        ))
    }

    fn analyze_index(
        &mut self,
        value: &'arena parser::Expr<'arena>,
        index: &'arena parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        let value = self.analyze(value)?;
        let index = self.analyze(index)?;

        // Determine the result type based on the value type
        let result_ty = match value.0.view() {
            TypeKind::Array(element_ty) => {
                // Arrays are indexed by integers
                self.expect_type(index.0, self.type_manager.int(), "array index must be Int")?;
                element_ty
            }
            TypeKind::Map(key_ty, value_ty) => {
                // Maps are indexed by their key type
                self.expect_type(index.0, key_ty, "map index must match key type")?;
                value_ty
            }
            TypeKind::Bytes => {
                // Bytes are indexed by integers, return Int
                self.expect_type(index.0, self.type_manager.int(), "bytes index must be Int")?;
                self.type_manager.int()
            }
            TypeKind::TypeVar(_) => {
                // Type variable not yet resolved - add relational Indexable constraint
                // The constraint tracks: Indexable(container, index, result)
                // When resolved, it will unify based on the container's concrete type:
                //   - Array[E]: index=Int, result=E
                //   - Map[K,V]: index=K, result=V
                //   - Bytes: index=Int, result=Int

                let result_ty = self.type_manager.fresh_type_var();

                // Add the relational constraint
                self.type_class_resolver.add_indexable_constraint(
                    value.0,
                    index.0,
                    result_ty,
                    self.get_span(),
                );

                result_ty
            }
            _ => {
                return Err(TypeError::new(
                    TypeErrorKind::NotIndexable {
                        ty: format!("{}", value.0),
                        span: self.get_span(),
                    },
                    self.get_source(),
                ));
            }
        };

        Ok(self.alloc(result_ty, ExprInner::Index { value, index }))
    }

    fn analyze_field(
        &mut self,
        value: &'arena parser::Expr<'arena>,
        field: &'arena str,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        let value = self.analyze(value)?;

        // Check that value is a record and get the field type
        let result_ty = match value.0.view() {
            TypeKind::Record(fields) => {
                // Clone the iterator to use it twice (once for search, once for error message)
                let fields_vec: Vec<_> = fields.collect();

                // Look for the field in the record
                fields_vec
                    .iter()
                    .find(|(name, _)| *name == field)
                    .map(|(_, ty)| *ty)
                    .ok_or_else(|| {
                        TypeError::new(
                            TypeErrorKind::UnknownField {
                                field: field.to_string(),
                                available_fields: fields_vec
                                    .iter()
                                    .map(|(n, _)| n.to_string())
                                    .collect(),
                                span: self.get_span(),
                            },
                            self.get_source(),
                        )
                    })?
            }
            TypeKind::TypeVar(_) => {
                // Cannot infer record type from field access alone
                // This would require row polymorphism
                return Err(TypeError::new(
                    TypeErrorKind::CannotInferRecordType {
                        field: field.to_string(),
                        span: self.get_span(),
                    },
                    self.get_source(),
                ));
            }
            _ => {
                return Err(TypeError::new(
                    TypeErrorKind::NotARecord {
                        ty: format!("{}", value.0),
                        field: field.to_string(),
                        span: self.get_span(),
                    },
                    self.get_source(),
                ));
            }
        };

        Ok(self.alloc(result_ty, ExprInner::Field { value, field }))
    }

    fn analyze_cast(
        &mut self,
        ty_expr: &'arena parser::TypeExpr<'arena>,
        expr: &'arena parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        let expr = self.analyze(expr)?;

        // Convert parser type to internal type
        let target_ty = type_expr_to_type(self.type_manager, ty_expr).map_err(|err| {
            TypeError::new(
                TypeErrorKind::InvalidTypeExpression {
                    message: format!("{:?}", err),
                    span: self.get_span(),
                },
                self.get_source(),
            )
        })?;

        Ok(self.alloc(target_ty, ExprInner::Cast { expr }))
    }

    fn analyze_lambda(
        &mut self,
        params: &'arena [&'arena str],
        body: &'arena parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        let ty = self.type_manager;

        // Create shared recording vector and push recording scope
        let recorded = Rc::new(RefCell::new(BTreeSet::new()));
        let recording_scope = scope_stack::RecordingScope::new(recorded.clone());
        self.scope_stack.push(recording_scope);

        // Push incomplete scope with parameter names
        self.scope_stack.push(
            scope_stack::IncompleteScope::new(self.arena, params).map_err(|e| {
                TypeError::new(
                    TypeErrorKind::DuplicateParameter {
                        name: e.0.to_string(),
                        span: self.get_span(),
                    },
                    self.get_source(),
                )
            })?,
        );

        // Create fresh type variables for parameters
        let param_types: Vec<&'types Type<'types>> =
            (0..params.len()).map(|_| ty.fresh_type_var()).collect();

        // Bind parameters to their types in scope
        for (param_name, param_ty) in params.iter().zip(param_types.iter()) {
            // Parameters get simple non-polymorphic types (no generalization)
            let scheme = self.unification.generalize(*param_ty, &hashbrown::HashSet::new());

            self.scope_stack
                .bind_in_current(*param_name, scheme)
                .map_err(|e| self.internal_error(format!("Failed to bind parameter: {:?}", e)))?;
        }

        // Push parameter type vars to env_vars_stack so they won't be generalized
        // These should NOT be generalized in nested where clauses
        let mut param_env_vars = hashbrown::HashSet::new();
        for param_ty in &param_types {
            param_env_vars.extend(self.unification.free_type_vars(*param_ty));
        }
        self.env_vars_stack.push(param_env_vars);

        let body = self.analyze(body)?;

        // Pop environment variables
        self.env_vars_stack.pop();

        // Pop parameter scope
        self.scope_stack
            .pop()
            .map_err(|e| self.internal_error(format!("Failed to pop scope: {:?}", e)))?;

        // Pop recording scope (we don't need the returned value)
        self.scope_stack
            .pop()
            .map_err(|e| self.internal_error(format!("Failed to pop recording scope: {:?}", e)))?;

        // Get captured variables from the recording scope
        let captured_vars = Rc::try_unwrap(recorded)
            .unwrap_or_else(|_| panic!("Recording scope still referenced"))
            .into_inner();

        // Build the function type
        let fn_ty = ty.function(&param_types, body.0);

        Ok(self.alloc(
            fn_ty,
            ExprInner::Lambda {
                params,
                body,
                captures: self.arena.alloc_slice_copy(&captured_vars.into_iter().collect::<Vec<_>>()),
            },
        ))
    }

    fn analyze_if(
        &mut self,
        cond: &'arena parser::Expr<'arena>,
        then_branch: &'arena parser::Expr<'arena>,
        else_branch: &'arena parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        let cond = self.analyze(cond)?;
        let then_branch = self.analyze(then_branch)?;
        let else_branch = self.analyze(else_branch)?;

        // Condition must be a boolean
        self.expect_type(cond.0, self.type_manager.bool(), "condition")?;

        // Both branches must have the same type
        let result_ty = self.expect_type(
            then_branch.0,
            else_branch.0,
            "if branches must have same type",
        )?;

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
        expr: &'arena parser::Expr<'arena>,
        bindings: &'arena [(&'arena str, &'arena parser::Expr<'arena>)],
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        // Extract binding names
        let names: Vec<&'arena str> = bindings.iter().map(|(name, _)| *name).collect();

        // Push incomplete scope with all binding names
        self.scope_stack.push(
            scope_stack::IncompleteScope::new(self.arena, &names).map_err(|e| {
                TypeError::new(
                    TypeErrorKind::DuplicateBinding {
                        name: e.0.to_string(),
                        span: self.get_span(),
                    },
                    self.get_source(),
                )
            })?,
        );

        // Analyze and bind each expression sequentially
        let mut analyzed_bindings: Vec<(&'arena str, &'arena mut Expr<'types, 'arena>)> =
            Vec::new();
        for (name, value_expr) in bindings.iter() {
            let analyzed = self.analyze(value_expr)?;

            // Generalize the type to a type scheme
            // Use current environment variables to prevent generalizing over lambda parameters
            let env_vars = self.get_env_vars();
            let scheme = self.unification.generalize(analyzed.0, &env_vars);

            self.scope_stack
                .bind_in_current(*name, scheme)
                .map_err(|e| self.internal_error(format!("Failed to bind in where: {:?}", e)))?;
            analyzed_bindings.push((*name, analyzed));
        }

        let expr_typed = self.analyze(expr)?;

        self.scope_stack
            .pop()
            .map_err(|e| self.internal_error(format!("Failed to pop scope: {:?}", e)))?;

        Ok(self.alloc(
            expr_typed.0,
            ExprInner::Where {
                expr: expr_typed,
                bindings: self
                    .arena
                    .alloc_slice_fill_iter(analyzed_bindings.into_iter().map(|(k, v)| (k, &*v))),
            },
        ))
    }

    fn analyze_otherwise(
        &mut self,
        primary: &'arena parser::Expr<'arena>,
        fallback: &'arena parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        let primary = self.analyze(primary)?;
        let fallback = self.analyze(fallback)?;

        // Both expressions must have the same type
        let result_ty = self.expect_type(
            primary.0,
            fallback.0,
            "otherwise branches must have same type",
        )?;

        Ok(self.alloc(
            result_ty,
            ExprInner::Otherwise { primary, fallback },
        ))
    }

    fn analyze_record(
        &mut self,
        items: &'arena [(&'arena str, &'arena parser::Expr<'arena>)],
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        let mut fields: Vec<(&'arena str, &'arena mut Expr<'types, 'arena>)> = Vec::new();
        let mut field_types: Vec<(&'arena str, &'types Type<'types>)> = Vec::new();

        for (name, value_expr) in items {
            let value = self.analyze(value_expr)?;
            field_types.push((*name, value.0));
            fields.push((*name, value));
        }

        let record_ty = self.type_manager.record(field_types);

        Ok(self.alloc(
            record_ty,
            ExprInner::Record {
                fields: self.arena.alloc_slice_fill_iter(fields.into_iter().map(|(k, v)| (k, &*v))),
            },
        ))
    }

    fn analyze_map(
        &mut self,
        items: &'arena [(&'arena parser::Expr<'arena>, &'arena parser::Expr<'arena>)],
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        if items.is_empty() {
            // Empty map - use fresh type variables for key and value
            let key_ty = self.type_manager.fresh_type_var();
            let value_ty = self.type_manager.fresh_type_var();
            let map_ty = self.type_manager.map(key_ty, value_ty);

            return Ok(self.alloc(map_ty, ExprInner::Map { elements: &[] }));
        }

        // Analyze all keys and values
        let mut entries: Vec<(
            &'arena mut Expr<'types, 'arena>,
            &'arena mut Expr<'types, 'arena>,
        )> = Vec::new();

        for (key_expr, value_expr) in items {
            let key = self.analyze(key_expr)?;
            let value = self.analyze(value_expr)?;
            entries.push((key, value));
        }

        // All keys must have the same type, all values must have the same type
        let key_ty = entries[0].0 .0;
        let value_ty = entries[0].1 .0;

        for (i, (key, value)) in entries.iter().enumerate().skip(1) {
            self.expect_type(
                key.0,
                key_ty,
                &format!("map key {} must match first key type", i),
            )?;
            self.expect_type(
                value.0,
                value_ty,
                &format!("map value {} must match first value type", i),
            )?;
        }

        // Map keys must be hashable
        let span = self.get_span();
        self.type_class_resolver.add_hashable_constraint(key_ty, span);

        let map_ty = self.type_manager.map(key_ty, value_ty);

        Ok(self.alloc(
            map_ty,
            ExprInner::Map {
                elements: self.arena.alloc_slice_fill_iter(entries.into_iter().map(|(k, v)| (&*k, &*v))),
            },
        ))
    }

    fn analyze_array(
        &mut self,
        exprs: &'arena [&'arena parser::Expr<'arena>],
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        if exprs.is_empty() {
            // Empty array - use a fresh type variable for the element type
            let element_ty = self.type_manager.fresh_type_var();
            let array_ty = self.type_manager.array(element_ty);

            return Ok(self.alloc(array_ty, ExprInner::Array { elements: &[] }));
        }

        // Analyze all elements
        let elements: Vec<&'arena mut Expr<'types, 'arena>> = exprs
            .iter()
            .map(|e| self.analyze(e))
            .collect::<Result<_, _>>()?;

        // All elements must have the same type
        let element_ty = elements[0].0;
        for (i, elem) in elements.iter().enumerate().skip(1) {
            self.expect_type(
                elem.0,
                element_ty,
                &format!("array element {} must match first element type", i),
            )?;
        }

        let array_ty = self.type_manager.array(element_ty);

        Ok(self.alloc(
            array_ty,
            ExprInner::Array {
                elements: self.arena.alloc_slice_fill_iter(elements.into_iter().map(|e| &*e)),
            },
        ))
    }

    fn analyze_format_str(
        &mut self,
        _strs: &'arena [&'arena str],
        exprs: &'arena [&'arena parser::Expr<'arena>],
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        // Analyze all interpolated expressions
        let exprs_typed: Vec<&'arena mut Expr<'types, 'arena>> = exprs
            .iter()
            .map(|e| self.analyze(e))
            .collect::<Result<_, _>>()?;

        // Format strings always produce Str type
        Ok(self.alloc(
            self.type_manager.str(),
            ExprInner::FormatStr {
                strs: _strs,
                exprs: self.arena.alloc_slice_fill_iter(exprs_typed.into_iter().map(|e| &*e)),
            },
        ))
    }

    fn analyze_literal(
        &mut self,
        literal: &parser::Literal<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        use parser::Literal;
        use crate::values::dynamic::Value;

        let (ty, constant) = match literal {
            Literal::Int { value, .. } => (
                self.type_manager.int(),
                Value::int(self.type_manager, *value),
            ),
            Literal::Float { value, .. } => (
                self.type_manager.float(),
                Value::float(self.type_manager, *value),
            ),
            Literal::Bool(value) => (
                self.type_manager.bool(),
                Value::bool(self.type_manager, *value),
            ),
            Literal::Str(value) => {
                let ty = self.type_manager.str();
                (ty, Value::str(self.arena, ty, value))
            }
            Literal::Bytes(value) => {
                let ty = self.type_manager.bytes();
                (ty, Value::bytes(self.arena, ty, value))
            }
        };

        Ok(self.alloc(ty, ExprInner::Constant(constant)))
    }

    fn analyze_ident(
        &mut self,
        ident: &'arena str,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        // Look up the identifier in the scope stack
        if let Some(scheme) = self.scope_stack.lookup(ident) {
            // Instantiate the type scheme with fresh type variables
            let ty = self
                .unification
                .instantiate(scheme, &mut self.type_class_resolver);
            return Ok(self.alloc(ty, ExprInner::Ident(ident)));
        }

        Err(TypeError::new(
            TypeErrorKind::UnboundVariable {
                name: ident.to_string(),
                span: self.get_span(),
            },
            self.get_source(),
        ))
    }
}
