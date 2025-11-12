//! Compiled Melbi expressions.

use super::Error;
use crate::analyzer::typed_expr::TypedExpr;
use crate::evaluator::{EvaluatorOptions, Evaluator};
use crate::types::{Type, manager::TypeManager, traits::TypeView};
use crate::values::dynamic::Value;
use crate::{Vec, format};
use bumpalo::Bump;

/// A compiled Melbi expression ready for execution.
///
/// Compiled expressions borrow from the Engine's arena and can be executed
/// multiple times with different arguments and value arenas.
///
/// # Execution Tiers
///
/// - **`run()`**: Safe, validates arguments at runtime
/// - **`run_unchecked()`**: Unsafe, skips validation for maximum performance
///
/// # Lifetimes
///
/// - `'arena`: Lifetime of the Engine's arena (holds types, AST, and metadata)
///
/// # Future Work
///
/// For multi-threading support, we'll need to either:
/// - Clone/copy expressions to independent arenas
/// - Use bytecode instead of AST (which can own its data)
/// - Modify Evaluator to accept expressions with different lifetimes
///
/// # Example
///
/// ```ignore
/// let expr = engine.compile("x + y", &[("x", int_ty), ("y", int_ty)])?;
///
/// // Execute with validation
/// let result = expr.run(&arena, &[Value::int(ty_mgr, 10), Value::int(ty_mgr, 32)])?;
///
/// // Execute without validation (unsafe, but faster)
/// let result = unsafe {
///     expr.run_unchecked(&arena, &[Value::int(ty_mgr, 10), Value::int(ty_mgr, 32)])
/// };
/// ```
pub struct CompiledExpression<'arena> {
    /// The type-checked AST
    typed_expr: &'arena TypedExpr<'arena, 'arena>,

    /// Type manager for creating values
    type_manager: &'arena TypeManager<'arena>,

    /// Parameters for validation
    params: &'arena [(&'arena str, &'arena Type<'arena>)],
}

impl<'arena> CompiledExpression<'arena> {
    /// Create a new compiled expression.
    ///
    /// This is called internally by Engine::compile().
    pub(crate) fn new(
        typed_expr: &'arena TypedExpr<'arena, 'arena>,
        type_manager: &'arena TypeManager<'arena>,
        params: &'arena [(&'arena str, &'arena Type<'arena>)],
    ) -> Self {
        Self {
            typed_expr,
            type_manager,
            params,
        }
    }

    /// Execute the expression with runtime validation.
    ///
    /// This is the **safe dynamic API** - it validates:
    /// - Argument count matches parameters
    /// - Argument types match parameter types
    ///
    /// # Parameters
    ///
    /// - `arena`: Arena for allocating the result value
    /// - `args`: Argument values (must match parameter types)
    ///
    /// # Returns
    ///
    /// The result value, or a runtime/validation error.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let result = expr.run(&arena, &[
    ///     Value::int(type_mgr, 10),
    ///     Value::int(type_mgr, 32),
    /// ])?;
    /// ```
    pub fn run<'val>(
        &self,
        arena: &'val Bump,
        args: &[Value<'arena, 'val>],
    ) -> Result<Value<'arena, 'val>, Error> {
        // Validate argument count
        if args.len() != self.params.len() {
            return Err(Error::Api(format!(
                "Argument count mismatch: expected {}, got {}",
                self.params.len(),
                args.len()
            )));
        }

        // Validate argument types using pointer equality (types are interned)
        for (i, (arg, (_param_name, expected_ty))) in args.iter().zip(self.params.iter()).enumerate()
        {
            if !core::ptr::eq(arg.ty, *expected_ty) {
                return Err(Error::Api(format!(
                    "Type mismatch for parameter {}: types don't match",
                    i
                )));
            }
        }

        // Execute
        let result = unsafe { self.run_unchecked(arena, args) };
        Ok(result)
    }

    /// Execute the expression without validation.
    ///
    /// This is the **unsafe unchecked API** - maximum performance, no validation.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// - Argument count matches `self.params().len()`
    /// - Each argument's type matches the corresponding parameter type
    /// - Arguments were created with the same TypeManager as the expression
    ///
    /// Violating these invariants may cause panics or incorrect results.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // SAFETY: We know the expression expects (Int, Int) and we're passing (Int, Int)
    /// let result = unsafe {
    ///     expr.run_unchecked(&arena, &[
    ///         Value::int(type_mgr, 10),
    ///         Value::int(type_mgr, 32),
    ///     ])
    /// };
    /// ```
    pub unsafe fn run_unchecked<'val>(
        &self,
        arena: &'val Bump,
        args: &[Value<'arena, 'val>],
    ) -> Value<'arena, 'val> {
        // Create evaluator options from engine options
        // Note: We'll need to thread EngineOptions through, or use defaults for now
        let options = EvaluatorOptions { max_depth: 1000 };

        // Prepare variables for evaluation (params = args)
        // Copy parameter names into the value arena so lifetimes match
        let mut variables = Vec::new();
        for ((name, _ty), value) in self.params.iter().zip(args.iter()) {
            let name_in_val_arena: &'val str = arena.alloc_str(name);
            variables.push((name_in_val_arena, *value));
        }
        let variables_slice = arena.alloc_slice_copy(&variables);

        // Prepare globals for evaluation
        // TODO: Thread through the actual environment from Engine
        let globals: &[(&str, Value<'arena, 'val>)] = &[];

        // Create evaluator and execute
        let mut evaluator = Evaluator::new(
            options,
            arena,
            self.type_manager,
            globals,
            variables_slice,
        );

        // Evaluate the expression
        // SAFETY: We transmute the expression lifetime to match the evaluator's arena lifetime.
        // This is safe because:
        // 1. The expression is only borrowed for the duration of eval()
        // 2. The actual data lives in 'arena which outlives 'val in practice
        // 3. The evaluator doesn't store the expression reference
        let expr_for_eval: &'val TypedExpr<'arena, 'val> =
            unsafe { core::mem::transmute(self.typed_expr) };

        // In the unchecked path, we unwrap errors since type checking guarantees correctness
        match evaluator.eval(expr_for_eval) {
            Ok(value) => value,
            Err(err) => {
                // In unchecked mode, evaluation errors are still possible (e.g., div by zero)
                // We can't return Result, so we'll need to handle this differently
                // For now, panic (this will be refined)
                panic!("Evaluation error in unchecked mode: {:?}", err);
            }
        }
    }

    /// Get the expression's parameters.
    ///
    /// Returns a slice of (name, type) pairs.
    pub fn params(&self) -> &[(&'arena str, &'arena Type<'arena>)] {
        self.params
    }

    /// Get the expression's return type.
    pub fn return_type(&self) -> &'arena Type<'arena> {
        self.typed_expr.expr.0
    }
}
