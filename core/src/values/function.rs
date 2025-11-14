//! Function value representation for FFI and future closures.
//!
//! This module defines the `Function` trait which represents callable values in Melbi.
//! Supports native Rust functions, and will support closures, foreign language functions, etc.

use super::dynamic::Value;
use crate::evaluator::ExecutionError;
use crate::types::{Type, manager::TypeManager};
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
pub trait Function<'types, 'arena> {
    /// Returns the function's type signature.
    ///
    /// This type is owned by the implementor and used for runtime validation
    /// in the safe `call()` wrapper (future feature).
    fn ty(&self) -> &'types Type<'types>;

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
    unsafe fn call_unchecked(
        &self,
        arena: &'arena Bump,
        type_mgr: &'types TypeManager<'types>,
        args: &[Value<'types, 'arena>],
    ) -> Result<Value<'types, 'arena>, ExecutionError>;
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
) -> Result<Value<'types, 'arena>, ExecutionError>;

/// Wrapper for native Rust function pointers.
///
/// Implements the `Function` trait by delegating to the wrapped function pointer.
/// This allows regular Rust functions to be used as Melbi functions.
///
/// # Example
///
/// ```ignore
/// let add_ty = type_mgr.function(&[type_mgr.int(), type_mgr.int()], type_mgr.int());
/// let func = NativeFunction::new(add_ty, array_add);
/// let value = Value::function(&arena, func)?;
/// ```
pub struct NativeFunction<'ty> {
    ty: &'ty Type<'ty>,
    func: NativeFn,
}

impl<'ty> NativeFunction<'ty> {
    /// Create a new native function with its type signature.
    pub fn new(ty: &'ty Type<'ty>, func: NativeFn) -> Self {
        Self { ty, func }
    }
}

impl<'types> Function<'types, '_> for NativeFunction<'types> {
    fn ty(&self) -> &'types Type<'types> {
        self.ty
    }

    unsafe fn call_unchecked<'arena>(
        &self,
        arena: &'arena Bump,
        type_mgr: &'types TypeManager<'types>,
        args: &[Value<'types, 'arena>],
    ) -> Result<Value<'types, 'arena>, ExecutionError> {
        // Delegate to the wrapped function pointer
        (self.func)(arena, type_mgr, args)
    }
}
