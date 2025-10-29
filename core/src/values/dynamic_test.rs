//! Tests for the Dynamic Value API
//!
//! This tests the new dynamic API that doesn't require compile-time type knowledge.

use crate::{types::manager::TypeManager, values::dynamic::Value};
use bumpalo::Bump;

#[test]
fn test_dynamic_int() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    // Use new API - no raw construction needed
    let value = Value::int(type_mgr, 42);

    // Extract dynamically without compile-time type
    let result = value.as_int().unwrap();
    assert_eq!(result, 42);
}

#[test]
fn test_dynamic_float() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let value = Value::float(type_mgr, 3.14);

    let result = value.as_float().unwrap();
    assert_eq!(result, 3.14);
}

#[test]
fn test_dynamic_bool() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let value = Value::bool(type_mgr, true);

    let result = value.as_bool().unwrap();
    assert_eq!(result, true);
}

#[test]
fn test_dynamic_type_mismatch() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    // Create an int value
    let value = Value::int(type_mgr, 42);

    // Try to extract as float - should fail
    let result = value.as_float();
    assert!(result.is_err());
}

#[test]
fn test_dynamic_array() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let int_ty = type_mgr.int();
    let array_ty = type_mgr.array(int_ty);

    // Use new API - validated construction
    let value = Value::array(
        &arena,
        array_ty,
        &[
            Value::int(type_mgr, 1),
            Value::int(type_mgr, 2),
            Value::int(type_mgr, 3),
        ],
    )
    .unwrap();

    // Extract as DynamicArray
    let array = value.as_array().unwrap();

    assert_eq!(array.len(), 3);
    // Elements are returned as Value, not typed!
    assert_eq!(array.get(0).unwrap().as_int().unwrap(), 1);
    assert_eq!(array.get(1).unwrap().as_int().unwrap(), 2);
    assert_eq!(array.get(2).unwrap().as_int().unwrap(), 3);
}

#[test]
fn test_dynamic_array_type_mismatch() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let int_ty = type_mgr.int();
    let array_ty = type_mgr.array(int_ty);

    let value = Value::array(
        &arena,
        array_ty,
        &[Value::int(type_mgr, 1), Value::int(type_mgr, 2)],
    )
    .unwrap();

    // Try to extract as int - should fail (it's an array)
    let result = value.as_int();
    assert!(result.is_err());
}

#[test]
fn test_dynamic_array_element_access() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let float_ty = type_mgr.float();
    let array_ty = type_mgr.array(float_ty);

    let value = Value::array(
        &arena,
        array_ty,
        &[
            Value::float(type_mgr, 1.1),
            Value::float(type_mgr, 2.2),
            Value::float(type_mgr, 3.3),
        ],
    )
    .unwrap();

    let array = value.as_array().unwrap();

    assert_eq!(array.get(0).unwrap().as_float().unwrap(), 1.1);
    assert_eq!(array.get(1).unwrap().as_float().unwrap(), 2.2);
    assert_eq!(array.get(2).unwrap().as_float().unwrap(), 3.3);
}

#[test]
fn test_dynamic_array_out_of_bounds() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let int_ty = type_mgr.int();
    let array_ty = type_mgr.array(int_ty);

    let value = Value::array(
        &arena,
        array_ty,
        &[Value::int(type_mgr, 1), Value::int(type_mgr, 2)],
    )
    .unwrap();

    let array = value.as_array().unwrap();

    // Out of bounds access returns None
    let result = array.get(5);
    assert!(result.is_none());
}

#[test]
fn test_dynamic_str() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let str_ty = type_mgr.str();
    let value = Value::str(&arena, str_ty, "hello world");

    let result = value.as_str().unwrap();
    assert_eq!(result, "hello world");
}

#[test]
fn test_dynamic_str_type_mismatch() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let value = Value::int(type_mgr, 42);

    // Try to extract as str - should fail
    let result = value.as_str();
    assert!(result.is_err());
}

#[test]
fn test_dynamic_bytes() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let bytes_ty = type_mgr.bytes();
    let input_bytes = b"hello\x00\xff";
    let value = Value::bytes(&arena, bytes_ty, input_bytes);

    let result = value.as_bytes().unwrap();
    assert_eq!(result, input_bytes);
}

#[test]
fn test_dynamic_bytes_type_mismatch() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let value = Value::int(type_mgr, 42);

    // Try to extract as bytes - should fail
    let result = value.as_bytes();
    assert!(result.is_err());
}

#[test]
fn test_dynamic_bytes_empty() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let bytes_ty = type_mgr.bytes();
    let value = Value::bytes(&arena, bytes_ty, b"");

    let result = value.as_bytes().unwrap();
    assert_eq!(result, b"");
    assert_eq!(result.len(), 0);
}
