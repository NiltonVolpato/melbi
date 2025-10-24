//! Tests for Display trait on Value - printing Melbi literals

use crate::{
    types::manager::TypeManager,
    values::{ArrayData, RawValue, Value},
};
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

    // Floats must always have a decimal point in Melbi
    let value = Value::float(type_mgr, 42.0);
    let output = format!("{}", value);
    assert!(
        output.contains('.'),
        "Float must have decimal point: {}",
        output
    );
    assert_eq!(output, "42.");
}

#[test]
fn test_display_float_zero() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let value = Value::float(type_mgr, 0.0);
    let output = format!("{}", value);
    assert!(
        output.contains('.'),
        "Float must have decimal point: {}",
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
    assert_eq!(format!("{}", value), "inf");
}

#[test]
fn test_display_float_neg_infinity() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let value = Value::float(type_mgr, f64::NEG_INFINITY);
    assert_eq!(format!("{}", value), "-inf");
}

#[test]
fn test_display_float_nan() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let value = Value::float(type_mgr, f64::NAN);
    assert_eq!(format!("{}", value), "nan");
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
    assert_eq!(format!("{}", value), "\"hello\"");
}

#[test]
fn test_display_str_empty() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let value = Value::str(&arena, type_mgr.str(), "");
    assert_eq!(format!("{}", value), "\"\"");
}

#[test]
fn test_display_str_with_quotes() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let value = Value::str(&arena, type_mgr.str(), "say \"hi\"");
    assert_eq!(format!("{}", value), "\"say \\\"hi\\\"\"");
}

#[test]
fn test_display_str_with_newline() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let value = Value::str(&arena, type_mgr.str(), "hello\nworld");
    assert_eq!(format!("{}", value), "\"hello\\nworld\"");
}

#[test]
fn test_display_str_with_backslash() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let value = Value::str(&arena, type_mgr.str(), "path\\to\\file");
    assert_eq!(format!("{}", value), "\"path\\\\to\\\\file\"");
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
    assert_eq!(format!("{}", value), "b\"\\x48\\x69\"");
}

#[test]
fn test_display_bytes_full_range() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let value = Value::bytes(&arena, type_mgr.bytes(), &[0x00, 0xFF, 0x42]);
    assert_eq!(format!("{}", value), "b\"\\x00\\xff\\x42\"");
}

#[test]
fn test_display_array_empty() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let int_ty = type_mgr.int();
    let array_ty = type_mgr.array(int_ty);

    let array_data = ArrayData::new_with(&arena, &[]);
    let value = Value::from_raw(
        &arena,
        array_ty,
        RawValue {
            boxed: array_data as *const ArrayData as *const RawValue,
        },
    );

    assert_eq!(format!("{}", value), "[]");
}

#[test]
fn test_display_array_int_simple() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let int_ty = type_mgr.int();
    let array_ty = type_mgr.array(int_ty);

    let raw_values = [
        RawValue { int_value: 1 },
        RawValue { int_value: 2 },
        RawValue { int_value: 3 },
    ];

    let array_data = ArrayData::new_with(&arena, &raw_values);
    let value = Value::from_raw(
        &arena,
        array_ty,
        RawValue {
            boxed: array_data as *const ArrayData as *const RawValue,
        },
    );

    assert_eq!(format!("{}", value), "[1, 2, 3]");
}

#[test]
fn test_display_array_float() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let float_ty = type_mgr.float();
    let array_ty = type_mgr.array(float_ty);

    let raw_values = [
        RawValue { float_value: 1.1 },
        RawValue { float_value: 2.0 },
        RawValue { float_value: 3.14 },
        RawValue { float_value: 0.5 },
    ];

    let array_data = ArrayData::new_with(&arena, &raw_values);
    let value = Value::from_raw(
        &arena,
        array_ty,
        RawValue {
            boxed: array_data as *const ArrayData as *const RawValue,
        },
    );

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

    let raw_values = [
        RawValue { bool_value: true },
        RawValue { bool_value: false },
        RawValue { bool_value: true },
    ];

    let array_data = ArrayData::new_with(&arena, &raw_values);
    let value = Value::from_raw(
        &arena,
        array_ty,
        RawValue {
            boxed: array_data as *const ArrayData as *const RawValue,
        },
    );

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
    let inner1_data = ArrayData::new_with(
        &arena,
        &[RawValue { int_value: 1 }, RawValue { int_value: 2 }],
    );
    let inner2_data = ArrayData::new_with(
        &arena,
        &[RawValue { int_value: 3 }, RawValue { int_value: 4 }],
    );

    // Create outer array containing the two inner arrays
    let outer_data = ArrayData::new_with(
        &arena,
        &[
            RawValue {
                boxed: inner1_data as *const ArrayData as *const RawValue,
            },
            RawValue {
                boxed: inner2_data as *const ArrayData as *const RawValue,
            },
        ],
    );

    let value = Value::from_raw(
        &arena,
        outer_array_ty,
        RawValue {
            boxed: outer_data as *const ArrayData as *const RawValue,
        },
    );

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
    let l1 = ArrayData::new_with(
        &arena,
        &[RawValue { int_value: 1 }, RawValue { int_value: 2 }],
    );
    let l2 = ArrayData::new_with(
        &arena,
        &[RawValue {
            boxed: l1 as *const ArrayData as *const RawValue,
        }],
    );
    let l3 = ArrayData::new_with(
        &arena,
        &[RawValue {
            boxed: l2 as *const ArrayData as *const RawValue,
        }],
    );

    let value = Value::from_raw(
        &arena,
        level3_ty,
        RawValue {
            boxed: l3 as *const ArrayData as *const RawValue,
        },
    );

    assert_eq!(format!("{}", value), "[[[1, 2]]]");
}

#[test]
fn test_display_array_with_negatives() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let int_ty = type_mgr.int();
    let array_ty = type_mgr.array(int_ty);

    let raw_values = [
        RawValue { int_value: -10 },
        RawValue { int_value: 0 },
        RawValue { int_value: 10 },
    ];

    let array_data = ArrayData::new_with(&arena, &raw_values);
    let value = Value::from_raw(
        &arena,
        array_ty,
        RawValue {
            boxed: array_data as *const ArrayData as *const RawValue,
        },
    );

    assert_eq!(format!("{}", value), "[-10, 0, 10]");
}

#[test]
fn test_display_array_single_element() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let int_ty = type_mgr.int();
    let array_ty = type_mgr.array(int_ty);

    let raw_values = [RawValue { int_value: 42 }];

    let array_data = ArrayData::new_with(&arena, &raw_values);
    let value = Value::from_raw(
        &arena,
        array_ty,
        RawValue {
            boxed: array_data as *const ArrayData as *const RawValue,
        },
    );

    assert_eq!(format!("{}", value), "[42]");
}

#[test]
fn test_display_large_array() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let int_ty = type_mgr.int();
    let array_ty = type_mgr.array(int_ty);

    let raw_values: Vec<RawValue> = (0..100).map(|i| RawValue { int_value: i }).collect();

    let array_data = ArrayData::new_with(&arena, &raw_values);
    let value = Value::from_raw(
        &arena,
        array_ty,
        RawValue {
            boxed: array_data as *const ArrayData as *const RawValue,
        },
    );

    let output = format!("{}", value);
    assert!(output.starts_with('['));
    assert!(output.ends_with(']'));
    assert!(output.contains("0"));
    assert!(output.contains("99"));
}
