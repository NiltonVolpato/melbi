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

// ============================================================================
// Equality Tests
// ============================================================================

#[test]
fn test_int_equality() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let a = Value::int(type_mgr, 42);
    let b = Value::int(type_mgr, 42);
    let c = Value::int(type_mgr, 43);

    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn test_float_equality() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let a = Value::float(type_mgr, 3.14);
    let b = Value::float(type_mgr, 3.14);
    let c = Value::float(type_mgr, 2.71);

    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn test_float_nan_inequality() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let nan1 = Value::float(type_mgr, f64::NAN);
    let nan2 = Value::float(type_mgr, f64::NAN);

    // Standard behavior: NaN != NaN
    assert_ne!(nan1, nan2);
}

#[test]
fn test_bool_equality() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let t1 = Value::bool(type_mgr, true);
    let t2 = Value::bool(type_mgr, true);
    let f = Value::bool(type_mgr, false);

    assert_eq!(t1, t2);
    assert_ne!(t1, f);
}

#[test]
fn test_str_equality() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let a = Value::str(&arena, type_mgr.str(), "hello");
    let b = Value::str(&arena, type_mgr.str(), "hello");
    let c = Value::str(&arena, type_mgr.str(), "world");

    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn test_str_empty_equality() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let a = Value::str(&arena, type_mgr.str(), "");
    let b = Value::str(&arena, type_mgr.str(), "");
    let c = Value::str(&arena, type_mgr.str(), "x");

    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn test_bytes_equality() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let a = Value::bytes(&arena, type_mgr.bytes(), b"hello");
    let b = Value::bytes(&arena, type_mgr.bytes(), b"hello");
    let c = Value::bytes(&arena, type_mgr.bytes(), b"world");

    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn test_bytes_with_nulls_equality() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let a = Value::bytes(&arena, type_mgr.bytes(), b"hello\x00\xff");
    let b = Value::bytes(&arena, type_mgr.bytes(), b"hello\x00\xff");
    let c = Value::bytes(&arena, type_mgr.bytes(), b"hello\x00\xfe");

    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn test_array_equality() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let arr_ty = type_mgr.array(type_mgr.int());

    let a = Value::array(
        &arena,
        arr_ty,
        &[
            Value::int(type_mgr, 1),
            Value::int(type_mgr, 2),
            Value::int(type_mgr, 3),
        ],
    )
    .unwrap();

    let b = Value::array(
        &arena,
        arr_ty,
        &[
            Value::int(type_mgr, 1),
            Value::int(type_mgr, 2),
            Value::int(type_mgr, 3),
        ],
    )
    .unwrap();

    let c = Value::array(
        &arena,
        arr_ty,
        &[
            Value::int(type_mgr, 1),
            Value::int(type_mgr, 2),
            Value::int(type_mgr, 4),
        ],
    )
    .unwrap();

    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn test_array_different_length_inequality() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let arr_ty = type_mgr.array(type_mgr.int());

    let a = Value::array(
        &arena,
        arr_ty,
        &[Value::int(type_mgr, 1), Value::int(type_mgr, 2)],
    )
    .unwrap();

    let b = Value::array(
        &arena,
        arr_ty,
        &[
            Value::int(type_mgr, 1),
            Value::int(type_mgr, 2),
            Value::int(type_mgr, 3),
        ],
    )
    .unwrap();

    assert_ne!(a, b);
}

#[test]
fn test_empty_array_equality() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let arr_ty = type_mgr.array(type_mgr.int());

    let a = Value::array(&arena, arr_ty, &[]).unwrap();
    let b = Value::array(&arena, arr_ty, &[]).unwrap();

    assert_eq!(a, b);
}

#[test]
fn test_nested_array_equality() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let inner_ty = type_mgr.array(type_mgr.int());
    let outer_ty = type_mgr.array(inner_ty);

    let inner1 = Value::array(
        &arena,
        inner_ty,
        &[Value::int(type_mgr, 1), Value::int(type_mgr, 2)],
    )
    .unwrap();

    let inner2 = Value::array(
        &arena,
        inner_ty,
        &[Value::int(type_mgr, 1), Value::int(type_mgr, 2)],
    )
    .unwrap();

    let inner3 = Value::array(
        &arena,
        inner_ty,
        &[Value::int(type_mgr, 3), Value::int(type_mgr, 4)],
    )
    .unwrap();

    let a = Value::array(&arena, outer_ty, &[inner1]).unwrap();
    let b = Value::array(&arena, outer_ty, &[inner2]).unwrap();
    let c = Value::array(&arena, outer_ty, &[inner3]).unwrap();

    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn test_record_equality() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let rec_ty = type_mgr.record(vec![("x", type_mgr.int()), ("y", type_mgr.float())]);

    let a = Value::record(
        &arena,
        rec_ty,
        &[
            ("x", Value::int(type_mgr, 42)),
            ("y", Value::float(type_mgr, 3.14)),
        ],
    )
    .unwrap();

    let b = Value::record(
        &arena,
        rec_ty,
        &[
            ("x", Value::int(type_mgr, 42)),
            ("y", Value::float(type_mgr, 3.14)),
        ],
    )
    .unwrap();

    let c = Value::record(
        &arena,
        rec_ty,
        &[
            ("x", Value::int(type_mgr, 99)),
            ("y", Value::float(type_mgr, 3.14)),
        ],
    )
    .unwrap();

    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn test_empty_record_equality() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let rec_ty = type_mgr.record(vec![]);

    let a = Value::record(&arena, rec_ty, &[]).unwrap();
    let b = Value::record(&arena, rec_ty, &[]).unwrap();

    assert_eq!(a, b);
}

#[test]
fn test_nested_record_equality() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let inner_ty = type_mgr.record(vec![("a", type_mgr.int())]);
    let outer_ty = type_mgr.record(vec![("inner", inner_ty)]);

    let inner1 = Value::record(&arena, inner_ty, &[("a", Value::int(type_mgr, 10))]).unwrap();
    let inner2 = Value::record(&arena, inner_ty, &[("a", Value::int(type_mgr, 10))]).unwrap();
    let inner3 = Value::record(&arena, inner_ty, &[("a", Value::int(type_mgr, 20))]).unwrap();

    let a = Value::record(&arena, outer_ty, &[("inner", inner1)]).unwrap();
    let b = Value::record(&arena, outer_ty, &[("inner", inner2)]).unwrap();
    let c = Value::record(&arena, outer_ty, &[("inner", inner3)]).unwrap();

    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn test_different_types_inequality() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let int_val = Value::int(type_mgr, 42);
    let float_val = Value::float(type_mgr, 42.0);
    let bool_val = Value::bool(type_mgr, true);

    assert_ne!(int_val, float_val);
    assert_ne!(int_val, bool_val);
    assert_ne!(float_val, bool_val);
}

// ============================================================================
// Hash Tests
// ============================================================================

use core::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

fn hash_value<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn test_int_hash_consistency() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let a = Value::int(type_mgr, 42);
    let b = Value::int(type_mgr, 42);

    // Same values should have same hash
    assert_eq!(hash_value(&a), hash_value(&b));
}

#[test]
fn test_int_hash_inequality() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let a = Value::int(type_mgr, 42);
    let b = Value::int(type_mgr, 43);

    // Different values should (usually) have different hashes
    // Note: This is not guaranteed but should be true for simple cases
    assert_ne!(hash_value(&a), hash_value(&b));
}

#[test]
fn test_float_hash_consistency() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let a = Value::float(type_mgr, 3.14);
    let b = Value::float(type_mgr, 3.14);

    assert_eq!(hash_value(&a), hash_value(&b));
}

#[test]
fn test_float_hash_zero_canonicalization() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let pos_zero = Value::float(type_mgr, 0.0);
    let neg_zero = Value::float(type_mgr, -0.0);

    // +0.0 == -0.0, so they must have the same hash (Hash/Eq invariant)
    assert_eq!(pos_zero, neg_zero);
    assert_eq!(hash_value(&pos_zero), hash_value(&neg_zero));
}

#[test]
fn test_float_hash_nan_canonicalization() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let nan1 = Value::float(type_mgr, f64::NAN);
    let nan2 = Value::float(type_mgr, f64::NAN);
    // Create a different NaN bit pattern
    let nan3 = Value::float(type_mgr, f64::from_bits(0x7ff8000000000001));

    // All NaN values should hash the same for consistency
    assert_eq!(hash_value(&nan1), hash_value(&nan2));
    assert_eq!(hash_value(&nan1), hash_value(&nan3));
}

#[test]
fn test_bool_hash() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let t1 = Value::bool(type_mgr, true);
    let t2 = Value::bool(type_mgr, true);
    let f = Value::bool(type_mgr, false);

    assert_eq!(hash_value(&t1), hash_value(&t2));
    assert_ne!(hash_value(&t1), hash_value(&f));
}

#[test]
fn test_str_hash() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let a = Value::str(&arena, type_mgr.str(), "hello");
    let b = Value::str(&arena, type_mgr.str(), "hello");
    let c = Value::str(&arena, type_mgr.str(), "world");

    assert_eq!(hash_value(&a), hash_value(&b));
    assert_ne!(hash_value(&a), hash_value(&c));
}

#[test]
fn test_bytes_hash() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let a = Value::bytes(&arena, type_mgr.bytes(), b"hello");
    let b = Value::bytes(&arena, type_mgr.bytes(), b"hello");
    let c = Value::bytes(&arena, type_mgr.bytes(), b"world");

    assert_eq!(hash_value(&a), hash_value(&b));
    assert_ne!(hash_value(&a), hash_value(&c));
}

#[test]
fn test_array_hash() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let arr_ty = type_mgr.array(type_mgr.int());

    let a = Value::array(
        &arena,
        arr_ty,
        &[
            Value::int(type_mgr, 1),
            Value::int(type_mgr, 2),
            Value::int(type_mgr, 3),
        ],
    )
    .unwrap();

    let b = Value::array(
        &arena,
        arr_ty,
        &[
            Value::int(type_mgr, 1),
            Value::int(type_mgr, 2),
            Value::int(type_mgr, 3),
        ],
    )
    .unwrap();

    let c = Value::array(
        &arena,
        arr_ty,
        &[
            Value::int(type_mgr, 1),
            Value::int(type_mgr, 2),
            Value::int(type_mgr, 4),
        ],
    )
    .unwrap();

    assert_eq!(hash_value(&a), hash_value(&b));
    assert_ne!(hash_value(&a), hash_value(&c));
}

#[test]
fn test_array_different_length_hash() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let arr_ty = type_mgr.array(type_mgr.int());

    let a = Value::array(
        &arena,
        arr_ty,
        &[Value::int(type_mgr, 1), Value::int(type_mgr, 2)],
    )
    .unwrap();

    let b = Value::array(
        &arena,
        arr_ty,
        &[
            Value::int(type_mgr, 1),
            Value::int(type_mgr, 2),
            Value::int(type_mgr, 3),
        ],
    )
    .unwrap();

    assert_ne!(hash_value(&a), hash_value(&b));
}

#[test]
fn test_nested_array_hash() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let inner_ty = type_mgr.array(type_mgr.int());
    let outer_ty = type_mgr.array(inner_ty);

    let inner1 = Value::array(
        &arena,
        inner_ty,
        &[Value::int(type_mgr, 1), Value::int(type_mgr, 2)],
    )
    .unwrap();

    let inner2 = Value::array(
        &arena,
        inner_ty,
        &[Value::int(type_mgr, 1), Value::int(type_mgr, 2)],
    )
    .unwrap();

    let a = Value::array(&arena, outer_ty, &[inner1]).unwrap();
    let b = Value::array(&arena, outer_ty, &[inner2]).unwrap();

    assert_eq!(hash_value(&a), hash_value(&b));
}

#[test]
fn test_hash_eq_consistency() {
    // Test that equal values have equal hashes (Hash/Eq invariant)
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    // Test with various types
    let values = vec![
        (
            Value::int(type_mgr, 42),
            Value::int(type_mgr, 42),
            "int",
        ),
        (
            Value::float(type_mgr, 3.14),
            Value::float(type_mgr, 3.14),
            "float",
        ),
        (
            Value::bool(type_mgr, true),
            Value::bool(type_mgr, true),
            "bool",
        ),
        (
            Value::str(&arena, type_mgr.str(), "hello"),
            Value::str(&arena, type_mgr.str(), "hello"),
            "str",
        ),
    ];

    for (a, b, type_name) in values {
        if a == b {
            assert_eq!(
                hash_value(&a),
                hash_value(&b),
                "Equal {} values must have equal hashes",
                type_name
            );
        }
    }
}

#[test]
fn test_different_types_different_hashes() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let int_val = Value::int(type_mgr, 1);
    let float_val = Value::float(type_mgr, 1.0);
    let bool_val = Value::bool(type_mgr, true);

    // Different types should have different hashes
    assert_ne!(hash_value(&int_val), hash_value(&float_val));
    assert_ne!(hash_value(&int_val), hash_value(&bool_val));
    assert_ne!(hash_value(&float_val), hash_value(&bool_val));
}
