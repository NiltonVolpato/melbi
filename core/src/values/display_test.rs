//! Tests for Display and Debug traits on Value
//!
//! Display: User-facing output (strings without quotes, native formatting)
//! Debug: Melbi literal representation (strings with quotes, decimal points on floats)

use crate::{Vec, format, types::manager::TypeManager, values::dynamic::Value};
use bumpalo::Bump;

#[test]
fn test_display_int_positive() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let value = Value::int(type_mgr, 42);
    assert_eq!(format!("{}", value), "42");
}

#[test]
fn test_display_int_negative() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let value = Value::int(type_mgr, -100);
    assert_eq!(format!("{}", value), "-100");
}

#[test]
fn test_display_int_zero() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let value = Value::int(type_mgr, 0);
    assert_eq!(format!("{}", value), "0");
}

#[test]
fn test_display_float_with_decimal() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let value = Value::float(type_mgr, 3.14);
    assert_eq!(format!("{}", value), "3.14");
}

#[test]
fn test_display_float_whole_number() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    // Display uses native Rust formatting (no decimal point required)
    let value = Value::float(type_mgr, 42.0);
    assert_eq!(format!("{}", value), "42");

    // Debug enforces Melbi convention (decimal point required)
    let output = format!("{:?}", value);
    assert!(
        output.contains('.'),
        "Float Debug must have decimal point: {}",
        output
    );
    assert_eq!(output, "42.");
}

#[test]
fn test_display_float_zero() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    // Display uses native Rust formatting
    let value = Value::float(type_mgr, 0.0);
    assert_eq!(format!("{}", value), "0");

    // Debug enforces Melbi convention (decimal point required)
    let output = format!("{:?}", value);
    assert!(
        output.contains('.'),
        "Float Debug must have decimal point: {}",
        output
    );
    assert_eq!(output, "0.");
}

#[test]
fn test_display_float_negative() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let value = Value::float(type_mgr, -3.14);
    assert_eq!(format!("{}", value), "-3.14");
}

#[test]
fn test_display_float_infinity() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let value = Value::float(type_mgr, f64::INFINITY);
    // Display uses native Rust formatting
    assert_eq!(format!("{}", value), "inf");
    // Debug uses Melbi convention
    assert_eq!(format!("{:?}", value), "inf");
}

#[test]
fn test_display_float_neg_infinity() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let value = Value::float(type_mgr, f64::NEG_INFINITY);
    // Display uses native Rust formatting
    assert_eq!(format!("{}", value), "-inf");
    // Debug uses Melbi convention
    assert_eq!(format!("{:?}", value), "-inf");
}

#[test]
fn test_display_float_nan() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let value = Value::float(type_mgr, f64::NAN);
    // Display uses native Rust formatting
    assert_eq!(format!("{}", value), "NaN");
    // Debug uses Melbi convention (lowercase)
    assert_eq!(format!("{:?}", value), "nan");
}

#[test]
fn test_display_bool_true() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let value = Value::bool(type_mgr, true);
    assert_eq!(format!("{}", value), "true");
}

#[test]
fn test_display_bool_false() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let value = Value::bool(type_mgr, false);
    assert_eq!(format!("{}", value), "false");
}

#[test]
fn test_display_str_simple() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let value = Value::str(&arena, type_mgr.str(), "hello");
    // Display: no quotes (for format strings)
    assert_eq!(format!("{}", value), "hello");
    // Debug: with quotes (for Melbi literals)
    assert_eq!(format!("{:?}", value), "\"hello\"");
}

#[test]
fn test_display_str_empty() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let value = Value::str(&arena, type_mgr.str(), "");
    // Display: no quotes
    assert_eq!(format!("{}", value), "");
    // Debug: with quotes
    assert_eq!(format!("{:?}", value), "\"\"");
}

#[test]
fn test_display_str_with_quotes() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let value = Value::str(&arena, type_mgr.str(), "say \"hi\"");
    // Display: raw string content (no escaping)
    assert_eq!(format!("{}", value), "say \"hi\"");
    // Debug: with quotes and escaped (prefers single quotes when string has double quotes)
    assert_eq!(format!("{:?}", value), "'say \"hi\"'");
}

#[test]
fn test_display_str_with_newline() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let value = Value::str(&arena, type_mgr.str(), "hello\nworld");
    // Display: raw string content (actual newline)
    assert_eq!(format!("{}", value), "hello\nworld");
    // Debug: with quotes and escaped
    assert_eq!(format!("{:?}", value), "\"hello\\nworld\"");
}

#[test]
fn test_display_str_with_backslash() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let value = Value::str(&arena, type_mgr.str(), "path\\to\\file");
    // Display: raw string content (actual backslashes)
    assert_eq!(format!("{}", value), "path\\to\\file");
    // Debug: with quotes and escaped
    assert_eq!(format!("{:?}", value), "\"path\\\\to\\\\file\"");
}

#[test]
fn test_display_bytes_empty() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let value = Value::bytes(&arena, type_mgr.bytes(), &[]);
    assert_eq!(format!("{}", value), "b\"\"");
}

#[test]
fn test_display_bytes_simple() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let value = Value::bytes(&arena, type_mgr.bytes(), &[0x48, 0x69]);
    assert_eq!(format!("{}", value), "b\"Hi\"");
}

#[test]
fn test_display_bytes_full_range() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let value = Value::bytes(&arena, type_mgr.bytes(), &[0x00, 0xFF, 0x42]);
    assert_eq!(format!("{}", value), "b\"\\x00\\xffB\"");
}

#[test]
fn test_display_array_empty() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let int_ty = type_mgr.int();
    let array_ty = type_mgr.array(int_ty);

    let value = Value::array(&arena, array_ty, &[]).unwrap();

    assert_eq!(format!("{}", value), "[]");
}

#[test]
fn test_display_array_int_simple() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let int_ty = type_mgr.int();
    let array_ty = type_mgr.array(int_ty);

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

    assert_eq!(format!("{}", value), "[1, 2, 3]");
}

#[test]
fn test_display_array_float() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let float_ty = type_mgr.float();
    let array_ty = type_mgr.array(float_ty);

    let value = Value::array(
        &arena,
        array_ty,
        &[
            Value::float(type_mgr, 1.1),
            Value::float(type_mgr, 2.0),
            Value::float(type_mgr, 3.14),
            Value::float(type_mgr, 0.5),
        ],
    )
    .unwrap();

    let output = format!("{}", value);
    assert_eq!("[1.1, 2., 3.14, 0.5]", output);
    // All floats must have decimal points
    assert!(output.contains("1.1") || output.contains("1.0"));
    assert!(output.contains("2."));
    assert!(output.contains("3.14"));
    assert!(output.contains(".5"));
}

#[test]
fn test_display_array_bool() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let bool_ty = type_mgr.bool();
    let array_ty = type_mgr.array(bool_ty);

    let value = Value::array(
        &arena,
        array_ty,
        &[
            Value::bool(type_mgr, true),
            Value::bool(type_mgr, false),
            Value::bool(type_mgr, true),
        ],
    )
    .unwrap();

    assert_eq!(format!("{}", value), "[true, false, true]");
}

#[test]
fn test_display_array_nested() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let int_ty = type_mgr.int();
    let inner_array_ty = type_mgr.array(int_ty);
    let outer_array_ty = type_mgr.array(inner_array_ty);

    // Create inner arrays [1, 2] and [3, 4]
    let inner1 = Value::array(
        &arena,
        inner_array_ty,
        &[Value::int(type_mgr, 1), Value::int(type_mgr, 2)],
    )
    .unwrap();

    let inner2 = Value::array(
        &arena,
        inner_array_ty,
        &[Value::int(type_mgr, 3), Value::int(type_mgr, 4)],
    )
    .unwrap();

    // Create outer array containing the two inner arrays
    let value = Value::array(&arena, outer_array_ty, &[inner1, inner2]).unwrap();

    assert_eq!(format!("{}", value), "[[1, 2], [3, 4]]");
}

#[test]
fn test_display_array_deeply_nested() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let int_ty = type_mgr.int();
    let level1_ty = type_mgr.array(int_ty);
    let level2_ty = type_mgr.array(level1_ty);
    let level3_ty = type_mgr.array(level2_ty);

    // [[[1, 2]]]
    let l1 = Value::array(
        &arena,
        level1_ty,
        &[Value::int(type_mgr, 1), Value::int(type_mgr, 2)],
    )
    .unwrap();

    let l2 = Value::array(&arena, level2_ty, &[l1]).unwrap();

    let value = Value::array(&arena, level3_ty, &[l2]).unwrap();

    assert_eq!(format!("{}", value), "[[[1, 2]]]");
}

#[test]
fn test_display_array_with_negatives() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let int_ty = type_mgr.int();
    let array_ty = type_mgr.array(int_ty);

    let value = Value::array(
        &arena,
        array_ty,
        &[
            Value::int(type_mgr, -10),
            Value::int(type_mgr, 0),
            Value::int(type_mgr, 10),
        ],
    )
    .unwrap();

    assert_eq!(format!("{}", value), "[-10, 0, 10]");
}

#[test]
fn test_display_array_single_element() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let int_ty = type_mgr.int();
    let array_ty = type_mgr.array(int_ty);

    let value = Value::array(&arena, array_ty, &[Value::int(type_mgr, 42)]).unwrap();

    assert_eq!(format!("{}", value), "[42]");
}

#[test]
fn test_display_large_array() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let int_ty = type_mgr.int();
    let array_ty = type_mgr.array(int_ty);

    let values: Vec<_> = (0..100).map(|i| Value::int(type_mgr, i)).collect();

    let value = Value::array(&arena, array_ty, &values).unwrap();

    let output = format!("{}", value);
    assert!(output.starts_with('['));
    assert!(output.ends_with(']'));
    assert!(output.contains("0"));
    assert!(output.contains("99"));
}
