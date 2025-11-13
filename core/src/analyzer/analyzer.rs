use alloc::collections::BTreeSet;
use alloc::rc::Rc;
use alloc::string::ToString;
use alloc::sync::Arc;
use bumpalo::Bump;
use core::cell::RefCell;

use crate::{
    String, Vec,
    analyzer::typed_expr::{Expr, ExprInner, TypedExpr},
    casting,
    errors::{Error, ErrorKind},
    format,
    parser::{self, BinaryOp, ComparisonOp, Span, UnaryOp},
    scope_stack::{self, ScopeStack},
    types::{
        Type, TypeClassId, TypeClassResolver, TypeScheme,
        manager::TypeManager,
        traits::{TypeKind, TypeView},
        type_expr_to_type,
        unification::Unification,
    },
    values::dynamic::Value,
};

// TODO: Create a temporary TypeManager for analysis only.
pub fn analyze<'types, 'arena>(
    type_manager: &'types TypeManager<'types>,
    arena: &'arena Bump,
    expr: &'arena parser::ParsedExpr<'arena>,
    globals: &[(&'arena str, &'types Type<'types>)],
    variables: &[(&'arena str, &'types Type<'types>)],
) -> Result<&'arena TypedExpr<'types, 'arena>, Error> {
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
    type_class_resolver: TypeClassResolver,
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
    ) -> Result<&'arena mut TypedExpr<'types, 'arena>, Error> {
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
        &mut self,
        got: &'types Type<'types>,
        expected: &'types Type<'types>,
        context: &str,
    ) -> Result<&'types Type<'types>, Error> {
        let unification_result = self.unification.unifies_to(got, expected);
        self.with_context(
            unification_result,
            format!("{}: expected {:?} = {:?}", context, expected, got),
        )
    }

    // Helper to expect numeric type
    fn expect_numeric(&self, ty: &'types Type<'types>, context: &str) -> Result<(), Error> {
        match ty {
            Type::Int | Type::Float => Ok(()),
            _ => Err(self.type_error(format!("{}: expected Int or Float, got {:?}", context, ty))),
        }
    }

    // Helper to expect Ord type (supports ordering comparisons)
    fn expect_ord(&self, ty: &'types Type<'types>, context: &str) -> Result<(), Error> {
        match ty {
            Type::Int | Type::Float | Type::Str | Type::Bytes => Ok(()),
            _ => Err(self.type_error(format!(
                "{}: expected Int, Float, Str, or Bytes (types that implement Ord), got {:?}",
                context, ty
            ))),
        }
    }

    // Convert current span to tuple for constraint tracking
    fn span_to_tuple(&self) -> (usize, usize) {
        self.current_span
            .as_ref()
            .map(|span| (span.0.start, span.0.end))
            .unwrap_or((0, 0))
    }

    // Add a Numeric constraint to a type (if it's a type variable)
    fn add_numeric_constraint(&mut self, ty: &'types Type<'types>) {
        if let TypeKind::TypeVar(id) = ty.view() {
            let span = self.span_to_tuple();
            self.type_class_resolver
                .add_constraint(id, TypeClassId::Numeric, span);
        }
    }

    // Add an Indexable constraint to a type (if it's a type variable)
    fn add_indexable_constraint(&mut self, ty: &'types Type<'types>) {
        if let TypeKind::TypeVar(id) = ty.view() {
            let span = self.span_to_tuple();
            self.type_class_resolver
                .add_constraint(id, TypeClassId::Indexable, span);
        }
    }

    // Add a Hashable constraint to a type (if it's a type variable)
    #[allow(dead_code)]
    fn add_hashable_constraint(&mut self, ty: &'types Type<'types>) {
        if let TypeKind::TypeVar(id) = ty.view() {
            let span = self.span_to_tuple();
            self.type_class_resolver
                .add_constraint(id, TypeClassId::Hashable, span);
        }
    }

    // Add an Ord constraint to a type (if it's a type variable)
    fn add_ord_constraint(&mut self, ty: &'types Type<'types>) {
        if let TypeKind::TypeVar(id) = ty.view() {
            let span = self.span_to_tuple();
            self.type_class_resolver
                .add_constraint(id, TypeClassId::Ord, span);
        }
    }

    // Finalize type checking by resolving all type class constraints
    fn finalize_constraints(&self) -> Result<(), Error> {
        let unification = &self.unification;

        let resolve_fn = |var: u16| -> &'types Type<'types> { unification.resolve_var(var) };

        self.type_class_resolver
            .resolve_all(resolve_fn)
            .map_err(|errors| {
                // For now, just report the first error
                // In the future, we can report all errors
                if let Some(first_error) = errors.first() {
                    Error {
                        kind: Arc::new(ErrorKind::TypeChecking {
                            src: self.parsed_ann.source.to_string(),
                            span: Some(Span(first_error.span.0..first_error.span.1)),
                            help: Some(first_error.message()),
                            unification_context: None,
                        }),
                        context: Vec::new(),
                    }
                } else {
                    self.type_error("Type class constraint error".to_string())
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
        expr: &parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, Error> {
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
            parser::Expr::Comparison { op, left, right } => {
                self.analyze_comparison(*op, left, right).map_err(|mut e| {
                    e.context
                        .push("While analyzing comparison expression".to_string());
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
    ) -> Result<&'arena mut Expr<'types, 'arena>, Error> {
        let left = self.analyze(left)?;
        let right = self.analyze(right)?;

        // Add Numeric constraints to both operands
        self.add_numeric_constraint(left.0);
        self.add_numeric_constraint(right.0);

        let result_ty = self.expect_type(left.0, right.0, "operands must have same type")?;

        // Check resolved types (if concrete, check immediately; if still type var, defer to finalize)
        let resolved_left = self.unification.resolve(left.0);
        let resolved_right = self.unification.resolve(right.0);

        // Only check if not a type variable (type variables will be checked at finalize)
        if !matches!(resolved_left.view(), TypeKind::TypeVar(_)) {
            self.expect_numeric(resolved_left, "left operand")?;
        }
        if !matches!(resolved_right.view(), TypeKind::TypeVar(_)) {
            self.expect_numeric(resolved_right, "right operand")?;
        }

        Ok(self.alloc(result_ty, ExprInner::Binary { op, left, right }))
    }

    fn analyze_boolean(
        &mut self,
        op: parser::BoolOp,
        left: &parser::Expr<'arena>,
        right: &parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, Error> {
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
        left: &parser::Expr<'arena>,
        right: &parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, Error> {
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

                // Add Ord constraints to both operands
                self.add_ord_constraint(left.0);
                self.add_ord_constraint(right.0);

                self.expect_type(left.0, right.0, "operands must have same type")?;

                // Check resolved types (if concrete, check immediately; if still type var, defer to finalize)
                let resolved_left = self.unification.resolve(left.0);
                let resolved_right = self.unification.resolve(right.0);

                // Only check if not a type variable (type variables will be checked at finalize)
                if !matches!(resolved_left.view(), TypeKind::TypeVar(_)) {
                    self.expect_ord(resolved_left, "left operand")?;
                }
                if !matches!(resolved_right.view(), TypeKind::TypeVar(_)) {
                    self.expect_ord(resolved_right, "right operand")?;
                }
            }
        }

        // All comparison operators return Bool
        Ok(self.alloc(
            self.type_manager.bool(),
            ExprInner::Comparison { op, left, right },
        ))
    }

    fn analyze_unary(
        &mut self,
        op: UnaryOp,
        expr: &parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, Error> {
        let expr = self.analyze(expr)?;
        let result_ty = match op {
            UnaryOp::Neg => {
                // Add Numeric constraint
                self.add_numeric_constraint(expr.0);

                // Check if concrete type
                let resolved = self.unification.resolve(expr.0);
                if !matches!(resolved.view(), TypeKind::TypeVar(_)) {
                    self.expect_numeric(resolved, "unary negation")?;
                }
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
    ) -> Result<&'arena mut Expr<'types, 'arena>, Error> {
        // 1. Analyze callable and its arguments.
        let callable = self.analyze(callable)?;
        let args_typed = self
            .arena
            .alloc_slice_try_fill_iter(args.iter().map(|arg| self.analyze(arg)))?;

        // 2. Extract actual argument types.
        let arg_types: Vec<_> = args_typed.iter().map(|arg| arg.0).collect();

        // 3. Create a fresh type variable for the return type.
        let ret_ty = self.type_manager.fresh_type_var();

        // 4. Construct the expected function type.
        let expected_fn_ty = self.type_manager.function(&arg_types, ret_ty);

        // 5. Unify callable's type with the expected function type.
        let unified = self.unification.unifies_to(callable.0, expected_fn_ty);
        let unified_fn_type = self.with_context(
            unified,
            "Function call: argument types do not match function signature",
        )?;

        // 6. Extract return type from unified function type.
        let TypeKind::Function { ret: result_ty, .. } = unified_fn_type.view() else {
            return Err(self.type_error(format!(
                "Internal error: Expected Function type after unification, got {}",
                unified_fn_type
            )));
        };

        // 7. Resolve the return type through substitution
        let resolved_ret_ty = self.unification.resolve(result_ty);

        // 8. Create the typed Call expression
        Ok(self.alloc(
            resolved_ret_ty,
            ExprInner::Call {
                callable,
                args: self
                    .arena
                    .alloc_slice_fill_iter(args_typed.into_iter().map(|arg| &**arg)),
            },
        ))
    }

    fn analyze_index(
        &mut self,
        value: &parser::Expr<'arena>,
        index: &parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, Error> {
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
                // Type variable not yet resolved - add Indexable constraint
                self.add_indexable_constraint(value.0);

                // For now, assume it's an array (most common case)
                // Create a fresh type variable for the element type
                let element_ty = self.type_manager.fresh_type_var();

                // Unify the value with Array<element_ty>
                let array_ty = self.type_manager.array(element_ty);
                let unify_result = self.unification.unifies_to(value.0, array_ty);
                self.with_context(unify_result, "Indexing requires an indexable type")?;

                // Unify index with Int (arrays are indexed by integers)
                self.expect_type(index.0, self.type_manager.int(), "array index must be Int")?;

                // Return the element type
                element_ty
            }
            _ => {
                return Err(
                    self.type_error(format!("Cannot index into non-indexable type: {}", value.0))
                );
            }
        };

        Ok(self.alloc(result_ty, ExprInner::Index { value, index }))
    }

    fn analyze_field(
        &mut self,
        value: &parser::Expr<'arena>,
        field: &'arena str,
    ) -> Result<&'arena mut Expr<'types, 'arena>, Error> {
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
                        self.type_error(format!(
                            "Record does not have field '{}'. Available fields: {}",
                            field,
                            fields_vec
                                .iter()
                                .map(|(n, _)| *n)
                                .collect::<Vec<_>>()
                                .join(", ")
                        ))
                    })?
            }
            TypeKind::TypeVar(_) => {
                // Cannot infer record type from field access alone
                // TODO(row-polymorphism): With row polymorphism, we could infer
                // "any record with at least field 'x' of some type"
                return Err(self.type_error(format!(
                    "Cannot infer record type for field access `.{}`. Row polymorphism not yet supported.",
                    field
                )));
            }
            _ => {
                return Err(self.type_error(format!(
                    "Cannot access field on non-record type: {}",
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
    ) -> Result<&'arena mut Expr<'types, 'arena>, Error> {
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
    ) -> Result<&'arena mut Expr<'types, 'arena>, Error> {
        let ty = self.type_manager;

        // Create shared recording vector and push recording scope
        let recorded = Rc::new(RefCell::new(BTreeSet::new()));
        let recording_scope = scope_stack::RecordingScope::new(recorded.clone());
        self.scope_stack.push(recording_scope);

        // Push incomplete scope with parameter names
        self.scope_stack.push(
            scope_stack::IncompleteScope::new(self.arena, params)
                .map_err(|e| self.type_error(format!("Duplicate parameter name: {}", e.0)))?,
        );

        // Bind each parameter to a fresh type variable (monomorphic)
        let mut param_types: Vec<&'types Type<'types>> = Vec::new();
        for param in params.iter() {
            let param_ty = ty.fresh_type_var();

            // Wrap in monomorphic TypeScheme (lambda parameters are not polymorphic)
            let empty_quantified = self.type_manager.alloc_u16_slice(&[]);
            let scheme = TypeScheme::new(empty_quantified, param_ty);

            self.scope_stack
                .bind_in_current(*param, scheme)
                .map_err(|e| self.type_error(format!("Failed to bind parameter: {:?}", e)))?;
            param_types.push(param_ty);
        }

        // Collect free type variables from parameter types and push to env_vars_stack
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
            .map_err(|e| self.type_error(format!("Failed to pop scope: {:?}", e)))?;

        // Pop recording scope (we don't need the returned value)
        self.scope_stack
            .pop()
            .map_err(|e| self.type_error(format!("Failed to pop recording scope: {:?}", e)))?;

        // Get recorded names from our Rc clone
        let captures = self
            .arena
            .alloc_slice_fill_iter(recorded.borrow().iter().copied());

        let result_ty = ty.function(
            self.arena.alloc_slice_fill_iter(
                param_types.into_iter().map(|t| self.unification.resolve(t)),
            ),
            body.0,
        );
        Ok(self.alloc(
            result_ty,
            ExprInner::Lambda {
                params: self.arena.alloc_slice_copy(params),
                body,
                captures,
            },
        ))
    }

    fn analyze_if(
        &mut self,
        cond: &parser::Expr<'arena>,
        then_branch: &parser::Expr<'arena>,
        else_branch: &parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, Error> {
        let cond = self.analyze(cond)?;
        let then_branch = self.analyze(then_branch)?;
        let else_branch = self.analyze(else_branch)?;

        cond.0 = self.expect_type(
            cond.0,
            self.type_manager.bool(),
            "If condition must be boolean",
        )?;

        // Separate the unification call to avoid borrowing issues
        let unify_result = self.unification.unifies_to(then_branch.0, else_branch.0);
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
    ) -> Result<&'arena mut Expr<'types, 'arena>, Error> {
        // Extract binding names
        let names: Vec<&'arena str> = bindings.iter().map(|(name, _)| *name).collect();

        // Push incomplete scope with all binding names
        self.scope_stack.push(
            scope_stack::IncompleteScope::new(self.arena, &names)
                .map_err(|e| self.type_error(format!("Duplicate binding name: {}", e.0)))?,
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
                .map_err(|e| self.type_error(format!("Failed to bind in where: {:?}", e)))?;
            analyzed_bindings.push((*name, analyzed));
        }

        let expr_typed = self.analyze(expr)?;

        self.scope_stack
            .pop()
            .map_err(|e| self.type_error(format!("Failed to pop scope: {:?}", e)))?;

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
        primary: &parser::Expr<'arena>,
        fallback: &parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, Error> {
        let primary = self.analyze(primary)?;
        let fallback = self.analyze(fallback)?;

        // TODO(effects): When effect system is implemented:
        // - Check that primary has an error effect (e.g., Type!)
        // - Strip the error effect from the result type
        // - Reject cases like `1 otherwise 0` where primary cannot fail
        // For now, we only verify that both branches have compatible types.

        // Separate the unification call to avoid borrowing issues
        let unify_result = self.unification.unifies_to(primary.0, fallback.0);
        let result_ty = self.with_context(
            unify_result,
            "Primary and fallback branches must have compatible types",
        )?;

        Ok(self.alloc(result_ty, ExprInner::Otherwise { primary, fallback }))
    }

    fn analyze_record(
        &mut self,
        items: &'arena [(&'arena str, &'arena parser::Expr<'arena>)],
    ) -> Result<&'arena mut Expr<'types, 'arena>, Error> {
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
        let result_ty = self.type_manager.record(field_types);

        Ok(self.alloc(
            result_ty,
            ExprInner::Record {
                fields: self
                    .arena
                    .alloc_slice_fill_iter(fields.into_iter().map(|(k, v)| (k, &*v))),
            },
        ))
    }

    fn analyze_map(
        &mut self,
        items: &'arena [(&'arena parser::Expr<'arena>, &'arena parser::Expr<'arena>)],
    ) -> Result<&'arena mut Expr<'types, 'arena>, Error> {
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
            let unification_result = self.unification.unifies_to(key.0, key_ty);
            key_ty = self.with_context(unification_result, "Map keys must have the same type")?;
        }

        // Unify all values to ensure they have the same type
        let mut value_ty = self.type_manager.fresh_type_var();
        for (_, value) in &elements {
            let unification_result = self.unification.unifies_to(value.0, value_ty);
            value_ty =
                self.with_context(unification_result, "Map values must have the same type")?;
        }

        let result_ty = self.type_manager.map(key_ty, value_ty);
        Ok(self.alloc(
            result_ty,
            ExprInner::Map {
                elements: self
                    .arena
                    .alloc_slice_fill_iter(elements.into_iter().map(|(k, v)| (&*k, &*v))),
            },
        ))
    }

    fn analyze_array(
        &mut self,
        exprs: &'arena [&'arena parser::Expr<'arena>],
    ) -> Result<&'arena mut Expr<'types, 'arena>, Error> {
        let elements: Vec<_> = exprs
            .iter()
            .map(|expr| self.analyze(expr))
            .collect::<Result<_, _>>()?;
        let mut element_ty = self.type_manager.fresh_type_var();
        for element in &elements {
            let unification_result = self.unification.unifies_to(element.0, element_ty);
            element_ty =
                self.with_context(unification_result, "Array elements must have the same type")?;
        }
        let result_ty = self.type_manager.array(element_ty);
        Ok(self.alloc(
            result_ty,
            ExprInner::Array {
                elements: self
                    .arena
                    .alloc_slice_fill_iter(elements.into_iter().map(|e| &*e)),
            },
        ))
    }

    fn analyze_format_str(
        &mut self,
        strs: &'arena [&'arena str],
        exprs: &'arena [&'arena parser::Expr<'arena>],
    ) -> Result<&'arena mut Expr<'types, 'arena>, Error> {
        let exprs_typed: Vec<_> = exprs
            .iter()
            .map(|expr| self.analyze(expr))
            .collect::<Result<_, _>>()?;

        // Check that all expressions are formattable (not functions)
        for expr in &exprs_typed {
            if matches!(expr.type_view(), TypeKind::Function { .. }) {
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
                exprs: self
                    .arena
                    .alloc_slice_fill_iter(exprs_typed.into_iter().map(|e| &*e)),
            },
        ))
    }

    fn analyze_literal(
        &mut self,
        literal: &parser::Literal<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, Error> {
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

    fn analyze_ident(
        &mut self,
        ident: &'arena str,
    ) -> Result<&'arena mut Expr<'types, 'arena>, Error> {
        if let Some(scheme) = self.scope_stack.lookup(ident) {
            // Instantiate the type scheme to get a fresh type
            // Constraints are automatically copied during instantiation
            let ty = self
                .unification
                .instantiate(scheme, &mut self.type_class_resolver);
            return Ok(self.alloc(ty, ExprInner::Ident(ident)));
        }

        Err(self.type_error(format!("Undefined variable: '{}'", ident)))
    }
}
