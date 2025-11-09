//! Function value representation for FFI and future closures.
//!
//! This module defines the `Function` trait which represents callable values in Melbi.
//! Supports native Rust functions, and will support closures, foreign language functions, etc.

use super::dynamic::Value;
use crate::evaluator::EvalError;
use crate::types::manager::TypeManager;
use bumpalo::Bump;

/// Trait for callable functions in Melbi.
///
/// All callable values (native FFI functions, closures, bytecode lambdas, etc.)
/// implement this trait.
///
/// The `call_unchecked` method uses generic lifetimes to allow functions to work with
/// different arena and type manager lifetimes.
///
/// # Safety
///
/// This trait is intended for internal use by the evaluator. User code should not
/// call `call_unchecked` directly - use the evaluator's function call mechanism instead,
/// which performs proper type checking and argument validation.
pub trait Function {
    /// Call the function with the given arguments, without runtime type checking.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// - The number of arguments matches the function's arity
    /// - Each argument's type matches the function's parameter types
    /// - The function type was validated by the type checker
    ///
    /// The evaluator guarantees these invariants, so this is safe when called
    /// from within the evaluator. Direct calls from user code may violate these
    /// invariants and cause panics or incorrect behavior.
    ///
    /// # Parameters
    /// - `arena`: Arena for allocating return values
    /// - `type_mgr`: Type manager for constructing typed values
    /// - `args`: Slice of evaluated arguments (types guaranteed by type checker)
    ///
    /// # Returns
    /// Result containing the return value, or an error that can be caught with `otherwise`.
    unsafe fn call_unchecked<'types, 'arena>(
        &self,
        arena: &'arena Bump,
        type_mgr: &'types TypeManager<'types>,
        args: &[Value<'types, 'arena>],
    ) -> Result<Value<'types, 'arena>, EvalError>;
}

/// Type alias for native FFI function pointers.
///
/// This is the signature expected for Rust functions that will be called from Melbi.
///
/// # Example
///
/// ```ignore
/// fn array_len<'types, 'arena>(
///     arena: &'arena Bump,
///     type_mgr: &'types TypeManager<'types>,
///     args: &[Value<'types, 'arena>],
/// ) -> Result<Value<'types, 'arena>, EvalError> {
///     assert_eq!(args.len(), 1);
///     assert!(args[0].is_array());
///
///     let array = args[0].as_array().unwrap();
///     Ok(Value::int(type_mgr, array.len() as i64))
/// }
/// ```
pub type NativeFn = for<'types, 'arena> fn(
    arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, EvalError>;

/// Wrapper for native Rust function pointers.
///
/// Implements the `Function` trait by delegating to the wrapped function pointer.
/// This allows regular Rust functions to be used as Melbi functions.
///
/// # Example
///
/// ```ignore
/// let func = NativeFunction(array_len);
/// let value = Value::function(&arena, func_ty, &func)?;
/// ```
pub struct NativeFunction(pub NativeFn);

impl Function for NativeFunction {
    unsafe fn call_unchecked<'types, 'arena>(
        &self,
        arena: &'arena Bump,
        type_mgr: &'types TypeManager<'types>,
        args: &[Value<'types, 'arena>],
    ) -> Result<Value<'types, 'arena>, EvalError> {
        // Delegate to the wrapped function pointer
        (self.0)(arena, type_mgr, args)
    }
}
