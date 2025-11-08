//! Function value representation for FFI and future closures.
//!
//! This module defines the `FunctionData` type which represents callable values in Melbi.
//! Currently supports native Rust functions via function pointers. Future phases will add
//! closure support.

use super::dynamic::Value;
use crate::evaluator::EvalError;
use crate::types::manager::TypeManager;
use bumpalo::Bump;

/// Type alias for native FFI function pointers.
///
/// FFI functions receive:
/// - `arena`: Arena for allocating return values
/// - `type_mgr`: Type manager for constructing typed values
/// - `args`: Slice of evaluated arguments (types guaranteed by analyzer)
///
/// Returns a `Result` to integrate with Melbi's error handling (`otherwise` operator).
///
/// # Example FFI Function
///
/// ```ignore
/// fn array_len<'types, 'arena>(
///     arena: &'arena Bump,
///     type_mgr: &'types TypeManager<'types>,
///     args: &[Value<'types, 'arena>],
/// ) -> Result<Value<'types, 'arena>, EvalError> {
///     // Type checker guarantees arity and types - use assertions
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

/// Function data stored in the arena.
///
/// Currently only supports native function pointers. Future phases will add:
/// - `Closure`: Melbi closures with captured environment
/// - `NativeTrait`: Trait object for stateful functions (if needed)
pub enum FunctionData {
    /// Native Rust function (FFI).
    ///
    /// Function pointer stored directly - zero allocation overhead.
    Native(NativeFn),
}

impl FunctionData {
    /// Create a new native function.
    pub fn native(func: NativeFn) -> Self {
        FunctionData::Native(func)
    }

    /// Get the function pointer if this is a native function.
    pub fn as_native(&self) -> Option<NativeFn> {
        match self {
            FunctionData::Native(func) => Some(*func),
        }
    }

    /// Convert to RawValue for storage in Value.
    ///
    /// Stores pointer to arena-allocated FunctionData.
    pub(crate) fn as_raw_value(&self) -> super::raw::RawValue {
        super::raw::RawValue {
            function: self as *const FunctionData,
        }
    }

    /// Convert from RawValue when extracting from Value.
    ///
    /// Returns reference to arena-allocated FunctionData with proper lifetime.
    pub(crate) fn from_raw_value<'a>(raw: super::raw::RawValue) -> &'a FunctionData {
        unsafe { &*raw.function }
    }
}
