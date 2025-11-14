use alloc::collections::BTreeSet;
use alloc::rc::Rc;
use alloc::string::ToString;
use bumpalo::Bump;
use core::cell::RefCell;

use crate::{
    String, Vec,
    analyzer::typed_expr::{Expr, ExprInner, TypedExpr},
    analyzer::error::{TypeError, TypeErrorKind},
    casting,
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
/// Analyzes a parsed expression into a typed expression within the provided arena and type context.
///
/// This is the entry point for type-checking and converting a `parser::ParsedExpr` into a
/// `TypedExpr` using the given `TypeManager` and allocation `Bump`. Provided `globals` and
/// `variables` are installed as monomorphic bindings before analysis. All type-class constraints
/// are resolved before returning.
///
/// # Parameters
///
/// - `type_manager`: the type system context used for creating and comparing types.
/// - `arena`: bump allocator used to allocate the resulting `TypedExpr` and related annotations.
/// - `expr`: the parsed expression to analyze.
/// - `globals`: sorted list of global bindings (name and concrete type) to make available during analysis.
/// - `variables`: sorted list of runtime variable bindings (name and concrete type) to make available during analysis.
///
/// # Returns
///
/// On success, a reference to the analyzed `TypedExpr` allocated in `arena`; on failure, a `TypeError`
/// describing the first encountered type-checking or constraint resolution error.
///
/// # Examples
///
/// ```no_run
/// use core::analyzer::analyzer::analyze;
/// // Assume `type_manager`, `arena`, and `parsed_expr` have been created appropriately.
/// // let typed = analyze(&type_manager, &arena, &parsed_expr, &globals, &variables)?;
/// ```
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
    /// Analyze a parsed expression and allocate the resulting typed expression in the arena.
    ///
    /// This method analyzes `expr` to produce a `TypedExpr` and returns a reference to the
    /// arena-allocated `TypedExpr` containing the analyzed expression and the analyzer's
    /// current typed annotation map.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Given an `analyzer` and a parsed expression `parsed_expr`:
    /// // let mut analyzer: Analyzer<'types, 'arena> = ...;
    /// // let parsed_expr: parser::ParsedExpr<'arena> = ...;
    /// let typed_ref = analyzer.analyze_expr(&parsed_expr)?;
    /// // `typed_ref` is a reference to the arena-allocated TypedExpr.
    /// ```
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

    /// Wraps a unification `Error` into a `TypeError` that carries the analyzer's current source `Span`.
    ///
    /// The provided `message` is accepted for future contextualization but is currently unused; only the
    /// unification error and the analyzer's current span are recorded in the resulting `TypeError`.
    ///
    /// # Examples
    ///
    /// ```
    /// // Simulated usage (types elided): `analyzer.with_context(unify_result, "unify failed")`
    /// let span = analyzer.current_span.clone().unwrap_or(Span(0..0));
    /// let _ = analyzer.with_context(Err(unify_err), "context").unwrap_err();
    /// assert_eq!(_ .span(), span);
    /// ```
    fn with_context<T>(
        &self,
        result: Result<T, crate::types::unification::Error>,
        message: impl Into<String>,
    ) -> Result<T, TypeError> {
        result.map_err(|err| {
            let span = self.current_span.clone().unwrap_or(Span(0..0));
            // Note: message parameter could be used to add context in the future
            let _ = message.into();
            TypeError::from_unification_error(err, span)
        })
    }

    // Get current span or default
    /// Return the analyzer's current source span, or a default zero-length span when none is set.
    ///
    /// # Returns
    ///
    /// `Span(0..0)` if no current span is recorded, otherwise the recorded `Span`.
    ///
    /// # Examples
    ///
    /// ```
    /// // The default zero-length span used when no span is set.
    /// let default = Span(0..0);
    /// assert_eq!(default, Span(0..0));
    /// ```
    fn get_span(&self) -> Span {
        self.current_span.clone().unwrap_or(Span(0..0))
    }

    // Helper for internal/unexpected errors (invariant violations)
    /// Create a TypeError for an internal invariant violation using the analyzer's current span.
    ///
    /// The provided `message` is attached to a `TypeErrorKind::Other` and the error's span is set
    /// to the analyzer's current span as returned by `get_span()`.
    ///
    /// # Examples
    ///
    /// ```
    /// // Given an `analyzer` value:
    /// let err = analyzer.internal_error("unexpected internal state");
    /// // `err` is a `TypeError` describing an internal error with the current span.
    /// ```
    fn internal_error(&self, message: impl Into<String>) -> TypeError {
        TypeError::new(TypeErrorKind::Other {
            message: message.into(),
            span: self.get_span(),
        })
    }

    // Helper to expect a specific type
    /// Ensure `got` can be unified with `expected`, producing the resolved type or a contextual type error.
    ///
    /// Unifies `got` with `expected` via the analyzer's unifier and, on failure, wraps the unification error
    /// with the current span and the provided context message.
    ///
    /// # Parameters
    ///
    /// - `context`: a short human-readable message describing the expectation used when reporting errors.
    ///
    /// # Returns
    ///
    /// - `Ok`: the resolved `got` type with any unifier substitutions applied.
    /// - `Err`: a `TypeError` describing the unification failure with span and the given context.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Given an Analyzer `an` and two types `got` and `expected`, assert they unify:
    /// let _ = an.expect_type(got, expected, "expected same type for operands");
    /// ```
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
    /// Ensures a type satisfies the Numeric type class (Int or Float).
    ///
    /// Returns `Err(TypeError::ConstraintViolation)` with the current span when `ty` is not `Int` or `Float`.
    ///
    /// # Examples
    ///
    /// ```
    /// // assuming `analyzer` is an initialized Analyzer and `ty` is a `Type`
    /// // analyzer.expect_numeric(&Type::Int, "context").unwrap();
    /// // assert!(analyzer.expect_numeric(&Type::Str, "context").is_err());
    /// ```
    fn expect_numeric(&self, ty: &'types Type<'types>, _context: &str) -> Result<(), TypeError> {
        match ty {
            Type::Int | Type::Float => Ok(()),
            _ => Err(TypeError::new(TypeErrorKind::ConstraintViolation {
                ty: format!("{}", ty),
                type_class: "Numeric".to_string(),
                span: self.get_span(),
            })),
        }
    }

    // Helper to expect Ord type (supports ordering comparisons)
    /// Ensures a type supports ordering (the `Ord` type class).
    ///
    /// Returns `Ok(())` if `ty` is one of `Int`, `Float`, `Str`, or `Bytes`; otherwise returns a
    /// `TypeError::ConstraintViolation` describing that `ty` does not satisfy the `Ord` constraint.
    /// The `_context` parameter is accepted for potential caller context but is not used by this check.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # // Example usage (ignored for doctest compilation)
    /// # use crate::types::Type;
    /// # use crate::analyzer::Analyzer;
    /// # fn example(analyzer: &Analyzer, ty: &Type) {
    /// analyzer.expect_ord(ty, "comparison");
    /// # }
    /// ```
    fn expect_ord(&self, ty: &'types Type<'types>, _context: &str) -> Result<(), TypeError> {
        match ty {
            Type::Int | Type::Float | Type::Str | Type::Bytes => Ok(()),
            _ => Err(TypeError::new(TypeErrorKind::ConstraintViolation {
                ty: format!("{}", ty),
                type_class: "Ord".to_string(),
                span: self.get_span(),
            })),
        }
    }

    // Add a Numeric constraint to a type (if it's a type variable)
    /// Adds a `Numeric` type-class constraint to `ty` when `ty` is a type variable.
    ///
    /// This records that the type variable must satisfy numeric operations (e.g., integer or float)
    /// by registering the constraint with the analyzer's TypeClassResolver using the current span.
    ///
    /// # Examples
    ///
    /// ```
    /// // Given an Analyzer `analyzer` and a type `ty` that may be a type variable:
    /// // analyzer.add_numeric_constraint(ty);
    /// ```
    fn add_numeric_constraint(&mut self, ty: &'types Type<'types>) {
        if let TypeKind::TypeVar(id) = ty.view() {
            let span = self.get_span();
            self.type_class_resolver
                .add_constraint(id, TypeClassId::Numeric, span);
        }
    }

    // Add an Indexable constraint to a type (if it's a type variable)
    fn add_indexable_constraint(&mut self, ty: &'types Type<'types>) {
        if let TypeKind::TypeVar(id) = ty.view() {
            let span = self.get_span();
            self.type_class_resolver
                .add_constraint(id, TypeClassId::Indexable, span);
        }
    }

    // Add a Hashable constraint to a type (if it's a type variable)
    #[allow(dead_code)]
    fn add_hashable_constraint(&mut self, ty: &'types Type<'types>) {
        if let TypeKind::TypeVar(id) = ty.view() {
            let span = self.get_span();
            self.type_class_resolver
                .add_constraint(id, TypeClassId::Hashable, span);
        }
    }

    // Add an Ord constraint to a type (if it's a type variable)
    /// Adds an `Ord` type-class constraint for a type variable.
    ///
    /// If `ty` is a type variable, records an `Ord` constraint for that variable
    /// using the current source span; otherwise this is a no-op.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Given an Analyzer `an` and a type variable `ty`, this records that `ty` must
    /// // implement the `Ord` type class:
    /// an.add_ord_constraint(ty);
    /// ```
    fn add_ord_constraint(&mut self, ty: &'types Type<'types>) {
        if let TypeKind::TypeVar(id) = ty.view() {
            let span = self.get_span();
            self.type_class_resolver
                .add_constraint(id, TypeClassId::Ord, span);
        }
    }

    // Finalize type checking by resolving all type class constraints
    /// Resolves all pending type-class constraints recorded during analysis.
    ///
    /// Converts constraint resolution failures into a `TypeError`. On error, the first
    /// constraint error is turned into a `TypeError` (with the analyzer's current span);
    /// if no concrete constraint error is available an internal `TypeError` is produced.
    ///
    /// # Returns
    ///
    /// `Ok(())` if all type-class constraints were resolved successfully, `Err(TypeError)` if any constraint cannot be satisfied.
    ///
    /// # Examples
    ///
    /// ```
    /// # // Example is illustrative; constructing a real `Analyzer` is omitted.
    /// # #[allow(unused_variables, dead_code)]
    /// # fn example(analyzer: &mut Analyzer<'static, 'static>) {
    /// let result = analyzer.finalize_constraints();
    /// match result {
    ///     Ok(()) => { /* constraints satisfied */ }
    ///     Err(err) => eprintln!("constraint error: {:?}", err),
    /// }
    /// # }
    /// ```
    fn finalize_constraints(&self) -> Result<(), TypeError> {
        let unification = &self.unification;

        let resolve_fn = |var: u16| -> &'types Type<'types> { unification.resolve_var(var) };

        self.type_class_resolver
            .resolve_all(resolve_fn)
            .map_err(|errors| {
                // For now, just report the first error
                // In the future, we can report all errors
                if let Some(first_error) = errors.first() {
                    TypeError::from_constraint_error(first_error.clone())
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

    /// Analyzes a parsed expression, producing an arena-allocated typed expression or a `TypeError`.
    ///
    /// This method sets the analyzer's current source span from the parsed annotations, dispatches
    /// the expression to the appropriate `analyze_*` handler based on its variant, restores the
    /// previous span, and returns the resulting `Expr` annotated with resolved types and constraints.
    ///
    /// # Examples
    ///
    /// ```
    /// // Setup omitted: create a TypeManager, arena, parsed annotations, and an Analyzer instance.
    /// // let mut analyzer = Analyzer::new(...);
    /// // let parsed_expr = parser::Expr::Literal(parser::Literal::Bool(true));
    /// // let typed = analyzer.analyze(&parsed_expr).unwrap();
    /// // assert!(matches!(typed.inner, Expr::Literal(_)));
    /// ```
    fn analyze(
        &mut self,
        expr: &parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        // Set current span for this expression from parsed annotations
        let old_span = self.current_span.clone();
        self.current_span = self.parsed_ann.span_of(expr);

        let result = match expr {
            parser::Expr::Binary { op, left, right } => {
                self.analyze_binary(*op, left, right)
            }
            parser::Expr::Boolean { op, left, right } => {
                self.analyze_boolean(*op, left, right)
            }
            parser::Expr::Comparison { op, left, right } => {
                self.analyze_comparison(*op, left, right)
            }
            parser::Expr::Unary { op, expr } => self.analyze_unary(*op, expr),
            parser::Expr::Call { callable, args } => {
                self.analyze_call(callable, args)
            }
            parser::Expr::Index { value, index } => {
                self.analyze_index(value, index)
            }
            parser::Expr::Field { value, field } => {
                self.analyze_field(value, *field)
            }
            parser::Expr::Cast { ty, expr } => self.analyze_cast(ty, expr),
            parser::Expr::Lambda { params, body } => {
                self.analyze_lambda(params, body)
            }
            parser::Expr::If {
                cond,
                then_branch,
                else_branch,
            } => self
                .analyze_if(cond, then_branch, else_branch),
            parser::Expr::Where { expr, bindings } => {
                self.analyze_where(expr, bindings)
            }
            parser::Expr::Otherwise { primary, fallback } => {
                self.analyze_otherwise(primary, fallback)
            }
            parser::Expr::Record(items) => self.analyze_record(items),
            parser::Expr::Map(items) => self.analyze_map(items),
            parser::Expr::Array(exprs) => self.analyze_array(exprs),
            parser::Expr::FormatStr { strs, exprs } => {
                self.analyze_format_str(strs, exprs)
            }
            parser::Expr::Literal(literal) => self.analyze_literal(literal),
            parser::Expr::Ident(ident) => self.analyze_ident(*ident),
        };

        // Restore previous span
        self.current_span = old_span;

        result
    }

    /// Analyze a binary operation, enforce operand type equality and numeric constraints, and allocate a typed `Binary` expression.
    ///
    /// The analyzer:
    /// - analyzes both operands,
    /// - adds numeric constraints to each operand,
    /// - unifies their types (erroring if they cannot be unified),
    /// - if an operand's type is already concrete, validates it is numeric immediately; otherwise defers numeric checks until constraint finalization,
    /// and then returns an allocated typed binary expression whose type is the unified operand type.
    ///
    /// # Examples
    ///
    /// ```
    /// // Pseudocode; types and values depend on surrounding Analyzer setup:
    /// let bin = analyzer.analyze_binary(BinaryOp::Add, &left_parsed_expr, &right_parsed_expr)?;
    /// ```
    fn analyze_binary(
        &mut self,
        op: BinaryOp,
        left: &parser::Expr<'arena>,
        right: &parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
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

    /// Type-checks and analyzes a boolean binary expression.
    ///
    /// Ensures both operands are `bool` and returns a typed boolean expression node.
    ///
    /// # Errors
    ///
    /// Returns a `TypeError` if either operand does not have boolean type; the error
    /// carries source-span context identifying the offending operand.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Pseudocode usage sketch (requires Analyzer setup from the crate):
    /// // let mut analyzer = Analyzer::new(...);
    /// // let left = parser::Expr::LiteralBool(true);
    /// // let right = parser::Expr::LiteralBool(false);
    /// // let typed = analyzer.analyze_boolean(parser::BoolOp::And, &left, &right)?;
    /// ```
    fn analyze_boolean(
        &mut self,
        op: parser::BoolOp,
        left: &parser::Expr<'arena>,
        right: &parser::Expr<'arena>,
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

    /// Analyzes a comparison expression and returns a typed comparison node.
    ///
    /// For equality operators (`==`, `!=`) this enforces that both operands have the same type.
    /// For ordering operators (`<`, `>`, `<=`, `>=`) this adds `Ord` constraints to both operands,
    /// enforces they have the same type, and immediately validates `Ord` for any concrete types
    /// (deferred for type variables until constraint finalization).
    ///
    /// # Returns
    ///
    /// A typed `Expr` representing the comparison with resulting type `Bool`.
    ///
    /// # Examples
    ///
    /// ```
    /// // Example usage (contextual; requires an initialized Analyzer, TypeManager, and parser Expr):
    /// // let mut analyzer = Analyzer::new(...);
    /// // let typed = analyzer.analyze_comparison(ComparisonOp::Lt, &left_parsed, &right_parsed)?;
    /// // assert_eq!(typed.annotation.ty, analyzer.type_manager.bool());
    /// ```
    fn analyze_comparison(
        &mut self,
        op: ComparisonOp,
        left: &parser::Expr<'arena>,
        right: &parser::Expr<'arena>,
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

    /// Analyzes a unary operation applied to a parsed expression and returns a typed unary expression node.
    ///
    /// Performs operator-specific checks and constraints:
    /// - For `Neg`, adds a numeric constraint to the operand and verifies concrete numeric types.
    /// - For `Not`, ensures the operand is boolean.
    ///
    /// # Parameters
    ///
    /// - `op`: the unary operator to apply.
    /// - `expr`: the parsed expression to analyze.
    ///
    /// # Returns
    ///
    /// A reference to the arena-allocated typed `Expr` representing the unary operation.
    ///
    /// # Examples
    ///
    /// ```
    /// // Illustrative usage:
    /// // let mut analyzer = Analyzer::new(...);
    /// // let parsed_expr = parser::Expr::Literal(...);
    /// // let typed_unary = analyzer.analyze_unary(UnaryOp::Neg, &parsed_expr).unwrap();
    /// ```
    fn analyze_unary(
        &mut self,
        op: UnaryOp,
        expr: &parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
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

    /// Analyze a function call expression, type-check its callable against the provided arguments, and produce a typed `Call` expression.
    ///
    /// This analyzes the callable and each argument, creates a fresh type variable for the call's return type, constructs an expected function type from the argument types to that return type, unifies the callable's inferred type with the expected function type (producing any necessary substitutions and constraints), resolves the concrete return type through the unifier, and allocates a typed `Call` expression with that return type and the typed argument list.
    ///
    /// # Returns
    ///
    /// A reference to the arena-allocated typed `Call` expression whose annotation is the resolved return type.
    ///
    /// # Errors
    ///
    /// Returns a `TypeError` when unification fails (argument types do not match the callable's function signature) or when an internal invariant is violated (the unified type is not a `Function`).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Given a prepared `Analyzer` with parsed `callable` and `args`,
    /// // this produces a typed call expression or a TypeError:
    /// // let typed_call = analyzer.analyze_call(&callable_expr, &args_exprs)?;
    /// ```
    fn analyze_call(
        &mut self,
        callable: &parser::Expr<'arena>,
        args: &'arena [&'arena parser::Expr<'arena>],
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
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
            return Err(self.internal_error(format!(
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

    /// Analyze an index expression, resolve the container and index types, and produce a typed `Index` expression.
    ///
    /// This checks and enforces indexability rules:
    /// - Arrays: index must be `Int`; result is the array element type.
    /// - Maps: index must match the map key type; result is the map value type.
    /// - Bytes: index must be `Int`; result is `Int`.
    /// - Type variables: an `Indexable` constraint is added and the analyzer attempts to unify the variable with `Array<element>`; the index must be `Int` and the element type is returned.
    /// If the container is not indexable, a `TypeError::NotIndexable` is returned with the current span.
    ///
    /// # Returns
    ///
    /// The allocated typed `Index` expression whose type corresponds to the element/value produced by indexing the container.
    ///
    /// # Examples
    ///
    /// ```
    /// // Assume `analyzer` is an initialized Analyzer and `value_expr`/`index_expr` are parser expressions.
    /// // The example demonstrates the intended call pattern; construction of analyzer and parser expressions
    /// // is omitted for brevity.
    /// let typed_index = analyzer.analyze_index(&value_expr, &index_expr).unwrap();
    /// // `typed_index` is a typed Expr representing `value[index]`.
    /// ```
    fn analyze_index(
        &mut self,
        value: &parser::Expr<'arena>,
        index: &parser::Expr<'arena>,
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
                return Err(TypeError::new(TypeErrorKind::NotIndexable {
                    ty: format!("{}", value.0),
                    span: self.get_span(),
                }));
            }
        };

        Ok(self.alloc(result_ty, ExprInner::Index { value, index }))
    }

    /// Analyze a field access expression and produce a typed `Field` expression.
    ///
    /// Looks up `field` on the analyzed `value`. If `value` is a record type, returns the field's
    /// type and allocates a `Field` typed expression. Errors if the value is not a record, if the
    /// field is unknown, or if the record type cannot be inferred from a type variable.
    ///
    /// # Parameters
    ///
    /// - `value`: the parsed expression whose field is being accessed.
    /// - `field`: the name of the field to access.
    ///
    /// # Returns
    ///
    /// A reference to the allocated typed `Expr::Field` node containing the resolved field type.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use core::analyzer::Analyzer;
    /// # use core::parser;
    /// # // Setup of `analyzer` and `parsed_expr` omitted for brevity.
    /// let mut analyzer: Analyzer<'_, '_> = /* ... */;
    /// let parsed_value: parser::Expr = /* parsed expression representing a record value */;
    /// let field_name: &str = "name";
    ///
    /// // Analyze a field access like `value.name`
    /// let typed_field = analyzer.analyze_field(&parsed_value, field_name)?;
    /// ```
    fn analyze_field(
        &mut self,
        value: &parser::Expr<'arena>,
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
                        TypeError::new(TypeErrorKind::UnknownField {
                            field: field.to_string(),
                            available_fields: fields_vec
                                .iter()
                                .map(|(n, _)| n.to_string())
                                .collect(),
                            span: self.get_span(),
                        })
                    })?
            }
            TypeKind::TypeVar(_) => {
                // Cannot infer record type from field access alone
                // TODO(row-polymorphism): With row polymorphism, we could infer
                // "any record with at least field 'x' of some type"
                return Err(TypeError::new(TypeErrorKind::CannotInferRecordType {
                    field: field.to_string(),
                    span: self.get_span(),
                }));
            }
            _ => {
                return Err(TypeError::new(TypeErrorKind::NotARecord {
                    ty: format!("{}", value.0),
                    field: field.to_string(),
                    span: self.get_span(),
                }));
            }
        };

        Ok(self.alloc(result_ty, ExprInner::Field { value, field }))
    }

    /// Analyze a cast expression and produce a typed `Cast` node.
    ///
    /// Converts the parsed target `TypeExpr` into an internal `Type`, analyzes the inner expression,
    /// and validates that a cast from the inner expression's type to the target type is permitted.
    /// On success, returns a typed `Expr::Cast` allocated in the arena with the target type as its
    /// annotation.
    ///
    /// # Errors
    ///
    /// Returns `TypeError::InvalidTypeExpression` if the target type expression cannot be converted
    /// to an internal `Type`. Returns `TypeError::InvalidCast` if the casting library reports that
    /// a cast from the source type to the target type is invalid. Both errors are annotated with
    /// the analyzer's current span.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Pseudo-usage:
    /// // let mut analyzer = Analyzer::new(...);
    /// // let typed_cast = analyzer.analyze_cast(&parsed_target_type_expr, &parsed_expr)?;
    /// ```
    fn analyze_cast(
        &mut self,
        ty: &parser::TypeExpr<'arena>,
        expr: &parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        let analyzed_expr = self.analyze(expr)?;
        let source_type = analyzed_expr.0;
        let target_type = match type_expr_to_type(self.type_manager, ty) {
            Ok(ty) => ty,
            Err(e) => {
                return Err(TypeError::new(TypeErrorKind::InvalidTypeExpression {
                    message: e.to_string(),
                    span: self.get_span(),
                }))
            }
        };

        // Validate the cast using casting library
        casting::validate_cast(source_type, target_type).map_err(|err| {
            TypeError::new(TypeErrorKind::InvalidCast {
                from: format!("{}", source_type),
                to: format!("{}", target_type),
                reason: err.to_string(),
                span: self.get_span(),
            })
        })?;

        // If cast is valid, create the cast expression
        // TODO(effects): Track whether cast is fallible
        Ok(self.alloc(
            target_type,
            ExprInner::Cast {
                expr: analyzed_expr,
            },
        ))
    }

    /// Analyze a lambda expression: bind parameters to fresh monomorphic types, analyze the body,
    /// record and capture free variables, and return a typed lambda expression node allocated in the arena.
    ///
    /// This method pushes a recording scope and an incomplete parameter scope, prevents generalization
    /// of parameter type variables in nested scopes, analyzes the body, then constructs and returns
    /// a `Lambda` expression whose type is a function from the parameter types to the body type.
    ///
    /// # Examples
    ///
    /// ```
    /// // Pseudocode example; actual construction of `Analyzer` and `parser::Expr` depends on the crate.
    ///
    /// // let mut analyzer = Analyzer::new(...);
    /// // let params = &["x", "y"];
    /// // let parsed_body = parser::Expr::...;
    /// // let typed_lambda = analyzer.analyze_lambda(params, &parsed_body).unwrap();
    /// // assert_eq!(typed_lambda.ty(), /* function type from param types to body type */);
    /// ```
    /* no outer attributes */
    fn analyze_lambda(
        &mut self,
        params: &'arena [&'arena str],
        body: &parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        let ty = self.type_manager;

        // Create shared recording vector and push recording scope
        let recorded = Rc::new(RefCell::new(BTreeSet::new()));
        let recording_scope = scope_stack::RecordingScope::new(recorded.clone());
        self.scope_stack.push(recording_scope);

        // Push incomplete scope with parameter names
        self.scope_stack.push(
            scope_stack::IncompleteScope::new(self.arena, params).map_err(|e| {
                TypeError::new(TypeErrorKind::DuplicateParameter {
                    name: e.0.to_string(),
                    span: self.get_span(),
                })
            })?,
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
                .map_err(|e| self.internal_error(format!("Failed to bind parameter: {:?}", e)))?;
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
            .map_err(|e| self.internal_error(format!("Failed to pop scope: {:?}", e)))?;

        // Pop recording scope (we don't need the returned value)
        self.scope_stack
            .pop()
            .map_err(|e| self.internal_error(format!("Failed to pop recording scope: {:?}", e)))?;

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

    /// Analyzes an `if` expression: type-checks the condition and unifies the then/else branch types.
    ///
    /// The condition is required to have boolean type. The types of the then and else branches are
    /// unified; the resulting unified type becomes the type of the `If` expression. Type errors
    /// (e.g., non-boolean condition or incompatible branch types) are returned as `TypeError`.
    ///
    /// # Returns
    ///
    /// A reference to the allocated typed `Expr::If` node.
    ///
    /// # Examples
    ///
    /// ```
    /// // Given an existing `analyzer: Analyzer<_, _>` and parsed expressions `cond`, `then_branch`, `else_branch`:
    /// let typed_if = analyzer.analyze_if(cond, then_branch, else_branch).unwrap();
    /// match &typed_if.inner {
    ///     core::ExprInner::If { .. } => { /* typed if-expression produced */ }
    ///     _ => panic!("expected If expression"),
    /// }
    /// ```
    fn analyze_if(
        &mut self,
        cond: &parser::Expr<'arena>,
        then_branch: &parser::Expr<'arena>,
        else_branch: &parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
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

    /// Analyzes a `where` clause: type-checks and binds its local declarations, then analyzes the main expression in the extended scope.
    ///
    /// Binds each provided name in a fresh incomplete scope, analyzes its expression, generalizes the resulting type into a `TypeScheme` while preventing generalization of type variables present in the current environment, and adds the binding to the scope. After all bindings are added, the main expression is analyzed in the extended scope. The scope is popped before returning the resulting typed `Where` expression.
    ///
    /// Errors:
    /// - Returns a `TypeError::DuplicateBinding` if the binding list contains duplicate names.
    /// - Propagates other `TypeError` values produced while analyzing bindings or the main expression.
    ///
    /// # Examples
    ///
    /// ```
    /// // Pseudocode example (setup omitted):
    /// // let mut analyzer = Analyzer::new(...);
    /// // let where_expr = parser::Expr::Where { /* ... */ };
    /// // let bindings = &[("x", &expr_x), ("y", &expr_y)];
    /// // let typed = analyzer.analyze_where(&where_expr, bindings)?;
    /// // assert!(matches!(typed.inner, ExprInner::Where { .. }));
    /// ```
    fn analyze_where(
        &mut self,
        expr: &parser::Expr<'arena>,
        bindings: &'arena [(&'arena str, &'arena parser::Expr<'arena>)],
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        // Extract binding names
        let names: Vec<&'arena str> = bindings.iter().map(|(name, _)| *name).collect();

        // Push incomplete scope with all binding names
        self.scope_stack.push(
            scope_stack::IncompleteScope::new(self.arena, &names).map_err(|e| {
                TypeError::new(TypeErrorKind::DuplicateBinding {
                    name: e.0.to_string(),
                    span: self.get_span(),
                })
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

    /// Produces a typed `Otherwise` expression by ensuring the primary and fallback branches have compatible types.
    ///
    /// Analyzes both branches and unifies their result types; if unification succeeds, allocates and returns an `Otherwise` expression annotated with the unified type. If the branches' types are incompatible, returns a `TypeError`.
    ///
    /// Note: effect-system checks for fallible primary branches are not implemented here (see TODO in implementation).
    ///
    /// # Returns
    ///
    /// `Ok` with a typed `Otherwise` expression whose type is the unified type of the primary and fallback branches, `Err` with a `TypeError` if analysis or type unification fails.
    fn analyze_otherwise(
        &mut self,
        primary: &parser::Expr<'arena>,
        fallback: &parser::Expr<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
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

    /// Analyzes a record literal and returns a typed record expression allocated in the arena.
    ///
    /// The function analyzes each field expression, collects their resolved types to construct
    /// a record type, and allocates a `Record` `Expr` node whose annotation is that record type.
    ///
    /// # Returns
    ///
    /// `&'arena mut Expr<'types, 'arena>` — a reference to the allocated `Record` expression with its resolved record type, or a `TypeError` if analysis of any field fails.
    fn analyze_record(
        &mut self,
        items: &'arena [(&'arena str, &'arena parser::Expr<'arena>)],
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        let fields: Vec<_> = items
            .iter()
            .map(|(key, value)| {
                let value = self.analyze(value)?;
                Ok::<_, TypeError>((*key, value))
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

    /// Analyzes a map literal, inferring and unifying a common key type and a common value type and producing a typed Map expression.
    ///
    /// This checks that all keys unify to a single key type and all values unify to a single value type, constructs a `Map<key, value>` type, and allocates a `Map` expression in the arena. Returns a `TypeError` if any unification or constraint fails (with the current span recorded).
    ///
    /// # Returns
    ///
    /// `Ok(&'arena mut Expr<...>)` with a `Map`-typed expression on success, `Err(TypeError)` on failure.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// // Given an analyzer `an` and a slice `items` of parsed key/value expressions:
    /// // let typed_map = an.analyze_map(items)?;
    /// // assert!(matches!(typed_map.inner, ExprInner::Map { .. }));
    /// ```
    fn analyze_map(
        &mut self,
        items: &'arena [(&'arena parser::Expr<'arena>, &'arena parser::Expr<'arena>)],
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        let elements: Vec<_> = items
            .iter()
            .map(|(key, value)| {
                let key = self.analyze(key)?;
                let value = self.analyze(value)?;
                Ok::<_, TypeError>((key, value))
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

    /// Analyze a sequence of parsed expressions and produce a typed array expression.
    ///
    /// This analyzes each element expression, enforces all elements share the same type
    /// (unifying their types as needed), and returns an arena-allocated `Expr` whose
    /// type is `Array<element_type>`. If element types cannot be unified, returns a
    /// `TypeError` with the context "Array elements must have the same type".
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Given an `Analyzer` instance `analyzer` and a slice of parsed expressions `exprs`,
    /// // call `analyze_array` to obtain a typed array expression:
    /// // let typed_array = analyzer.analyze_array(exprs)?;
    /// ```
    fn analyze_array(
        &mut self,
        exprs: &'arena [&'arena parser::Expr<'arena>],
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
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

    /// Analyzes a format string with embedded expressions and produces a typed string expression.
    ///
    /// Each expression is analyzed and must be formattable (not a function). On success returns a
    /// typed `FormatStr` expression whose type is `Str`. Propagates any type errors from analyzing
    /// the embedded expressions or from the formattability check.
    ///
    /// # Examples
    ///
    /// ```
    /// // Given an `analyzer` with an appropriate arena and parsed pieces:
    /// // let strs: &'arena [&'arena str] = ...;
    /// // let exprs: &'arena [&'arena parser::Expr<'arena>] = ...;
    /// // let typed = analyzer.analyze_format_str(strs, exprs).unwrap();
    /// // `typed` is a typed expression representing the formatted string.
    /// ```
    fn analyze_format_str(
        &mut self,
        strs: &'arena [&'arena str],
        exprs: &'arena [&'arena parser::Expr<'arena>],
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        let exprs_typed: Vec<_> = exprs
            .iter()
            .map(|expr| self.analyze(expr))
            .collect::<Result<_, _>>()?;

        // Check that all expressions are formattable (not functions)
        for expr in &exprs_typed {
            if matches!(expr.type_view(), TypeKind::Function { .. }) {
                return Err(TypeError::new(TypeErrorKind::NotFormattable {
                    ty: format!("{}", expr.0),
                    span: self.get_span(),
                }));
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

    /// Analyze a parser literal and produce a typed constant expression in the arena.
    ///
    /// This converts parser literal variants (int, float, bool, string, bytes) into a typed
    /// `Expr::Constant` allocated in the analyzer's arena. Integer and float literals with
    /// suffixes are rejected with a `TypeErrorKind::UnsupportedFeature`.
    ///
    /// # Returns
    ///
    /// An `Expr` node representing the literal with its concrete type.
    ///
    /// # Examples
    ///
    /// ```
    /// // Construct a parser literal and pass it to the analyzer to obtain a typed expression.
    /// // (Illustrative; actual construction of `Analyzer` and arena setup is omitted.)
    /// let lit = parser::Literal::Int { value: 42, suffix: None };
    /// // let typed_expr = analyzer.analyze_literal(&lit).unwrap();
    /// ```
    fn analyze_literal(
        &mut self,
        literal: &parser::Literal<'arena>,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        match literal {
            parser::Literal::Int { value, suffix } => {
                if let Some(_suffix) = suffix {
                    return Err(TypeError::new(TypeErrorKind::UnsupportedFeature {
                        feature: "Integer suffixes are not yet supported".to_string(),
                        suggestion: "In the future, suffixes will support units of measurement (e.g., 10`MB`, 5`seconds`)".to_string(),
                        span: self.get_span(),
                    }));
                }
                let ty = self.type_manager.int();
                let value = Value::int(self.type_manager, *value);
                Ok(self.alloc(ty, ExprInner::Constant(value)))
            }
            parser::Literal::Float { value, suffix } => {
                if let Some(_suffix) = suffix {
                    return Err(TypeError::new(TypeErrorKind::UnsupportedFeature {
                        feature: "Float suffixes are not yet supported".to_string(),
                        suggestion: "In the future, suffixes will support units of measurement (e.g., 3.14`meters`, 2.5`kg`)".to_string(),
                        span: self.get_span(),
                    }));
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

    /// Resolves an identifier in the current scope and returns a typed `Ident` expression.
    ///
    /// Instantiates the identifier's `TypeScheme` to produce a fresh type (copying any constraints)
    /// and allocates a `TypedExpr::Ident` node annotated with that instantiated type.
    ///
    /// Returns a reference to the allocated typed `Ident` expression on success.
    /// Returns a `TypeError::UnboundVariable` if the identifier is not found in scope.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Given an Analyzer `analyzer` with "x" bound in its scope:
    /// let typed = analyzer.analyze_ident("x").unwrap();
    /// match &typed.inner {
    ///     ExprInner::Ident(name) => assert_eq!(*name, "x"),
    ///     _ => panic!(),
    /// }
    /// ```
    fn analyze_ident(
        &mut self,
        ident: &'arena str,
    ) -> Result<&'arena mut Expr<'types, 'arena>, TypeError> {
        if let Some(scheme) = self.scope_stack.lookup(ident) {
            // Instantiate the type scheme to get a fresh type
            // Constraints are automatically copied during instantiation
            let ty = self
                .unification
                .instantiate(scheme, &mut self.type_class_resolver);
            return Ok(self.alloc(ty, ExprInner::Ident(ident)));
        }

        Err(TypeError::new(TypeErrorKind::UnboundVariable {
            name: ident.to_string(),
            span: self.get_span(),
        }))
    }
}