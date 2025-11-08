//! Unit tests for function values (FFI function pointers).

use bumpalo::Bump;

use crate::{
    evaluator::EvalError,
    types::manager::TypeManager,
    values::{
        dynamic::Value,
        from_raw::TypeError,
        function::{FunctionData, NativeFn},
    },
};

// ============================================================================
// Test FFI Functions
// ============================================================================

/// Simple test function: add two integers
fn test_add<'types, 'arena>(
    _arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, EvalError> {
    assert_eq!(args.len(), 2);
    let a = args[0].as_int().unwrap();
    let b = args[1].as_int().unwrap();
    Ok(Value::int(type_mgr, a + b))
}

/// Simple test function: negate a boolean
fn test_not<'types, 'arena>(
    _arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, EvalError> {
    assert_eq!(args.len(), 1);
    let b = args[0].as_bool().unwrap();
    Ok(Value::bool(type_mgr, !b))
}

// ============================================================================
// FunctionData Tests
// ============================================================================

#[test]
fn test_function_data_native_construction() {
    let func_data = FunctionData::native(test_add);

    match func_data {
        FunctionData::Native(_) => {
            // Success - it's a Native variant
        }
    }
}

#[test]
fn test_function_data_as_native() {
    let func_data = FunctionData::native(test_add);
    let func = func_data.as_native();

    assert!(func.is_some(), "Should extract native function");

    // Verify we can call it
    let bump = Bump::new();
    let type_mgr = TypeManager::new(&bump);
    let args = [Value::int(type_mgr, 10), Value::int(type_mgr, 32)];

    let result = func.unwrap()(&bump, type_mgr, &args);
    assert!(result.is_ok());
    let result_value = result.unwrap().as_int().unwrap();
    assert_eq!(result_value, 42);
}

// ============================================================================
// Value::native_function() Tests
// ============================================================================

#[test]
fn test_value_native_function_construction() {
    let bump = Bump::new();
    let type_mgr = TypeManager::new(&bump);

    // Create function type: (Int, Int) -> Int
    let func_ty = type_mgr.function(&[type_mgr.int(), type_mgr.int()], type_mgr.int());

    // Create function value
    let func_value = Value::native_function(&bump, func_ty, test_add);

    assert!(
        func_value.is_ok(),
        "Should create function value successfully"
    );

    let func_value = func_value.unwrap();
    assert!(core::ptr::eq(func_value.ty, func_ty), "Type should match");
}

#[test]
fn test_value_native_function_wrong_type() {
    let bump = Bump::new();
    let type_mgr = TypeManager::new(&bump);

    // Try to create function with non-function type (Int)
    let int_ty = type_mgr.int();
    let func_value = Value::native_function(&bump, int_ty, test_add);

    assert!(func_value.is_err(), "Should reject non-function type");
    assert!(matches!(func_value, Err(TypeError::Mismatch)));
}

#[test]
fn test_value_native_function_different_signatures() {
    let bump = Bump::new();
    let type_mgr = TypeManager::new(&bump);

    // (Int, Int) -> Int
    let add_ty = type_mgr.function(&[type_mgr.int(), type_mgr.int()], type_mgr.int());
    let add_value = Value::native_function(&bump, add_ty, test_add).unwrap();

    // (Bool) -> Bool
    let not_ty = type_mgr.function(&[type_mgr.bool()], type_mgr.bool());
    let not_value = Value::native_function(&bump, not_ty, test_not).unwrap();

    // Both should succeed - no runtime signature validation
    assert!(core::ptr::eq(add_value.ty, add_ty));
    assert!(core::ptr::eq(not_value.ty, not_ty));
}

// ============================================================================
// Value::as_function() Tests
// ============================================================================

#[test]
fn test_value_as_function_extraction() {
    let bump = Bump::new();
    let type_mgr = TypeManager::new(&bump);

    let func_ty = type_mgr.function(&[type_mgr.int(), type_mgr.int()], type_mgr.int());
    let func_value = Value::native_function(&bump, func_ty, test_add).unwrap();

    // Extract function data
    let func_data = func_value.as_function();
    assert!(func_data.is_ok(), "Should extract function data");

    // Verify it's the right function
    let func_data = func_data.unwrap();
    let native_fn = func_data.as_native();
    assert!(native_fn.is_some(), "Should be a native function");
}

#[test]
fn test_value_as_function_wrong_type() {
    let bump = Bump::new();
    let type_mgr = TypeManager::new(&bump);

    // Create an Int value, try to extract as function
    let int_value = Value::int(type_mgr, 42);
    let func_data = int_value.as_function();

    assert!(func_data.is_err(), "Should reject non-function value");
    assert!(matches!(func_data, Err(TypeError::Mismatch)));
}

#[test]
fn test_value_as_function_call_through() {
    let bump = Bump::new();
    let type_mgr = TypeManager::new(&bump);

    // Create function value
    let func_ty = type_mgr.function(&[type_mgr.int(), type_mgr.int()], type_mgr.int());
    let func_value = Value::native_function(&bump, func_ty, test_add).unwrap();

    // Extract and call
    let func_data = func_value.as_function().unwrap();
    let native_fn = func_data.as_native().unwrap();

    let args = [Value::int(type_mgr, 100), Value::int(type_mgr, 23)];

    let result = native_fn(&bump, type_mgr, &args);
    assert!(result.is_ok());

    let result_value = result.unwrap().as_int().unwrap();
    assert_eq!(result_value, 123);
}

// ============================================================================
// Memory and Lifetime Tests
// ============================================================================

#[test]
fn test_multiple_functions_same_arena() {
    let bump = Bump::new();
    let type_mgr = TypeManager::new(&bump);

    // Create multiple functions in same arena
    let add_ty = type_mgr.function(&[type_mgr.int(), type_mgr.int()], type_mgr.int());
    let not_ty = type_mgr.function(&[type_mgr.bool()], type_mgr.bool());

    let add_func = Value::native_function(&bump, add_ty, test_add).unwrap();
    let not_func = Value::native_function(&bump, not_ty, test_not).unwrap();

    // Both should be extractable
    assert!(add_func.as_function().is_ok());
    assert!(not_func.as_function().is_ok());

    // Verify they're different functions
    let add_fn = add_func.as_function().unwrap().as_native().unwrap();
    let not_fn = not_func.as_function().unwrap().as_native().unwrap();

    // Function pointers should be different
    assert_ne!(
        add_fn as *const (), not_fn as *const (),
        "Different functions should have different pointers"
    );
}

#[test]
fn test_function_pointer_size() {
    // Verify FunctionData is small (just a function pointer)
    use core::mem::size_of;

    // FunctionData is an enum with one variant containing a function pointer
    // Should be same size as a function pointer (one word on most platforms)
    assert_eq!(
        size_of::<FunctionData>(),
        size_of::<NativeFn>(),
        "FunctionData should be same size as function pointer"
    );
}
