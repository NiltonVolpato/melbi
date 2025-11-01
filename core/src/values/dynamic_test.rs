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

#[test]
fn test_empty_record() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let rec_ty = type_mgr.record(vec![]);
    let rec = Value::record(&arena, rec_ty, &[]).unwrap();

    let record = rec.as_record().unwrap();
    assert_eq!(record.len(), 0);
    assert!(record.is_empty());
    assert_eq!(format!("{}", rec), "{}");
}

#[test]
fn test_simple_record() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let rec_ty = type_mgr.record(vec![("x", type_mgr.int()), ("y", type_mgr.float())]);
    let x_val = Value::int(type_mgr, 42);
    let y_val = Value::float(type_mgr, 3.14);

    let rec = Value::record(&arena, rec_ty, &[("x", x_val), ("y", y_val)]).unwrap();

    let record = rec.as_record().unwrap();
    assert_eq!(record.len(), 2);
    assert!(!record.is_empty());

    // Test field access by name
    let x = record.get("x").unwrap();
    assert_eq!(x.as_int().unwrap(), 42);

    let y = record.get("y").unwrap();
    assert!((y.as_float().unwrap() - 3.14).abs() < 0.0001);

    // Non-existent field
    assert!(record.get("z").is_none());
}

#[test]
fn test_record_display() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let rec_ty = type_mgr.record(vec![("age", type_mgr.int()), ("name", type_mgr.str())]);

    let name_val = Value::str(&arena, type_mgr.str(), "Alice");
    let age_val = Value::int(type_mgr, 30);

    let rec = Value::record(&arena, rec_ty, &[("age", age_val), ("name", name_val)]).unwrap();

    let display = format!("{}", rec);
    assert_eq!(display, r#"{age = 30, name = "Alice"}"#);
}

#[test]
fn test_record_iteration() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let rec_ty = type_mgr.record(vec![("a", type_mgr.int()),
    ("b", type_mgr.int()),
    ("c", type_mgr.int()),]);

    let rec = Value::record(
        &arena,
        rec_ty,
        &[
            ("a", Value::int(type_mgr, 1)),
            ("b", Value::int(type_mgr, 2)),
            ("c", Value::int(type_mgr, 3)),
        ],
    )
    .unwrap();

    let record = rec.as_record().unwrap();

    // Collect field names and values
    let fields: Vec<_> = record
        .iter()
        .map(|(name, val)| (name, val.as_int().unwrap()))
        .collect();

    assert_eq!(fields.len(), 3);
    assert_eq!(fields[0], ("a", 1));
    assert_eq!(fields[1], ("b", 2));
    assert_eq!(fields[2], ("c", 3));
}

#[test]
fn test_record_exact_size_iterator() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let rec_ty = type_mgr.record(vec![("x", type_mgr.int()),
    ("y", type_mgr.int()),
    ("z", type_mgr.int()),]);

    let rec = Value::record(
        &arena,
        rec_ty,
        &[
            ("x", Value::int(type_mgr, 1)),
            ("y", Value::int(type_mgr, 2)),
            ("z", Value::int(type_mgr, 3)),
        ],
    )
    .unwrap();

    let record = rec.as_record().unwrap();
    let mut iter = record.iter();

    assert_eq!(iter.len(), 3);
    iter.next();
    assert_eq!(iter.len(), 2);
    iter.next();
    assert_eq!(iter.len(), 1);
    iter.next();
    assert_eq!(iter.len(), 0);
    assert!(iter.next().is_none());
}

#[test]
fn test_nested_record() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    // Inner record: { x: Int, y: Int }
    let inner_ty = type_mgr.record(vec![("x", type_mgr.int()), ("y", type_mgr.int())]);

    let inner = Value::record(
        &arena,
        inner_ty,
        &[
            ("x", Value::int(type_mgr, 10)),
            ("y", Value::int(type_mgr, 20)),
        ],
    )
    .unwrap();

    // Outer record: { name: Str, point: { x: Int, y: Int } }
    let outer_ty = type_mgr.record(vec![("name", type_mgr.str()), ("point", inner_ty)]);

    let name_val = Value::str(&arena, type_mgr.str(), "origin");

    let outer = Value::record(&arena, outer_ty, &[("name", name_val), ("point", inner)]).unwrap();

    // Test nested access
    let outer_rec = outer.as_record().unwrap();
    let point = outer_rec.get("point").unwrap();
    let point_rec = point.as_record().unwrap();

    let x = point_rec.get("x").unwrap();
    assert_eq!(x.as_int().unwrap(), 10);

    let y = point_rec.get("y").unwrap();
    assert_eq!(y.as_int().unwrap(), 20);

    // Test display with nested record
    let display = format!("{}", outer);
    assert_eq!(display, r#"{name = "origin", point = {x = 10, y = 20}}"#);
}

#[test]
fn test_record_type_validation_wrong_type() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    // Try to construct with Array type instead of Record
    let arr_ty = type_mgr.array(type_mgr.int());
    let result = Value::record(&arena, arr_ty, &[]);
    assert!(result.is_err());
}

#[test]
fn test_record_field_count_mismatch() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let rec_ty = type_mgr.record(vec![("x", type_mgr.int())]);

    // Provide no fields when type expects one
    let result = Value::record(&arena, rec_ty, &[]);
    assert!(result.is_err());

    // Provide two fields when type expects one
    let result = Value::record(
        &arena,
        rec_ty,
        &[
            ("x", Value::int(type_mgr, 1)),
            ("y", Value::int(type_mgr, 2)),
        ],
    );
    assert!(result.is_err());
}

#[test]
fn test_record_field_name_mismatch() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let rec_ty = type_mgr.record(vec![("x", type_mgr.int())]);

    // Provide wrong field name
    let result = Value::record(&arena, rec_ty, &[("y", Value::int(type_mgr, 42))]);
    assert!(result.is_err());
}

#[test]
fn test_record_field_type_mismatch() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let rec_ty = type_mgr.record(vec![("x", type_mgr.int())]);

    // Provide wrong field type (Float instead of Int)
    let result = Value::record(&arena, rec_ty, &[("x", Value::float(type_mgr, 3.14))]);
    assert!(result.is_err());
}

#[test]
fn test_as_record_type_error() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    // Try to extract record from an int value
    let val = Value::int(type_mgr, 42);
    assert!(val.as_record().is_err());

    // Try to extract record from an array
    let arr_ty = type_mgr.array(type_mgr.int());
    let arr = Value::array(&arena, arr_ty, &[]).unwrap();
    assert!(arr.as_record().is_err());
}
