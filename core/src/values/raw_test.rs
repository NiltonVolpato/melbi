//! Tests for Tier 3: Unsafe/untyped raw value manipulation
//!
//! These tests demonstrate direct manipulation of RawValue unions,
//! which is what the VM would do internally.

use super::*;
use bumpalo::Bump;

#[test]
fn test_raw_value_int() {
    let raw = RawValue { int_value: 42 };
    unsafe {
        assert_eq!(raw.int_value, 42);
    }
}

#[test]
fn test_raw_value_float() {
    let raw = RawValue { float_value: 3.14 };
    unsafe {
        assert_eq!(raw.float_value, 3.14);
    }
}

#[test]
fn test_raw_value_bool() {
    let raw_true = RawValue { bool_value: true };
    let raw_false = RawValue { bool_value: false };
    unsafe {
        assert_eq!(raw_true.bool_value, true);
        assert_eq!(raw_false.bool_value, false);
    }
}

#[test]
fn test_raw_value_copy() {
    let raw1 = RawValue { int_value: 100 };
    let raw2 = raw1;
    unsafe {
        assert_eq!(raw1.int_value, 100);
        assert_eq!(raw2.int_value, 100);
    }
}

#[test]
fn test_array_data_creation() {
    let arena = Bump::new();
    let values = [
        RawValue { int_value: 1 },
        RawValue { int_value: 2 },
        RawValue { int_value: 3 },
    ];
    let array_data = ArrayData::new_with(&arena, &values);
    assert_eq!(array_data.length(), 3);
    unsafe {
        assert_eq!(array_data.get(0).int_value, 1);
        assert_eq!(array_data.get(1).int_value, 2);
        assert_eq!(array_data.get(2).int_value, 3);
    }
}

#[test]
fn test_array_data_uninitialized() {
    let arena = Bump::new();
    let array_data = ArrayData::new_uninitialized_in(&arena, 5);
    assert_eq!(array_data.length(), 5);
    unsafe {
        array_data.set(0, RawValue { int_value: 10 });
        array_data.set(1, RawValue { int_value: 20 });
        array_data.set(2, RawValue { int_value: 30 });
        array_data.set(3, RawValue { int_value: 40 });
        array_data.set(4, RawValue { int_value: 50 });
        assert_eq!(array_data.get(0).int_value, 10);
        assert_eq!(array_data.get(1).int_value, 20);
        assert_eq!(array_data.get(2).int_value, 30);
        assert_eq!(array_data.get(3).int_value, 40);
        assert_eq!(array_data.get(4).int_value, 50);
    }
}

#[test]
fn test_array_data_mixed_types() {
    let arena = Bump::new();
    let values = [
        RawValue { int_value: 42 },
        RawValue { float_value: 3.14 },
        RawValue { bool_value: true },
    ];
    let array_data = ArrayData::new_with(&arena, &values);
    assert_eq!(array_data.length(), 3);
    unsafe {
        assert_eq!(array_data.get(0).int_value, 42);
        assert_eq!(array_data.get(1).float_value, 3.14);
        assert_eq!(array_data.get(2).bool_value, true);
    }
}

#[test]
fn test_array_data_empty() {
    let arena = Bump::new();
    let array_data = ArrayData::new_with(&arena, &[]);
    assert_eq!(array_data.length(), 0);
}

#[test]
fn test_array_data_large() {
    let arena = Bump::new();
    let values: Vec<RawValue> = (0..1000).map(|i| RawValue { int_value: i }).collect();
    let array_data = ArrayData::new_with(&arena, &values);
    assert_eq!(array_data.length(), 1000);
    unsafe {
        assert_eq!(array_data.get(0).int_value, 0);
        assert_eq!(array_data.get(500).int_value, 500);
        assert_eq!(array_data.get(999).int_value, 999);
    }
}

#[test]
fn test_raw_value_pointer() {
    let arena = Bump::new();
    let values = [RawValue { int_value: 1 }];
    let array_data = ArrayData::new_with(&arena, &values);
    let raw_ptr = RawValue {
        array: array_data,
    };
    unsafe {
        let retrieved_ptr = raw_ptr.array;
        assert_eq!((*retrieved_ptr).length(), 1);
    }
}

#[test]
#[should_panic]
fn test_type_confusion_danger() {
    let raw = RawValue { int_value: 42 };
    unsafe {
        let _ = raw.float_value;
        panic!("Type confusion detected");
    }
}
