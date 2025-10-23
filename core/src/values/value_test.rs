use std::marker::PhantomData;

use crate::{
    types::manager::TypeManager,
    values::{
        RawValue, Value,
        from_raw::{Array, FromRawValue as _},
        raw::ArrayData,
    },
};

// #[test]
// fn test_raw_i64() {
//     let arena = bumpalo::Bump::new();
//     let type_mgr = TypeManager::new(&arena);

//     let raw_value = RawValue { int_value: 42 };
//     let value = Value::from_raw(&arena, type_mgr.int(), raw_value);
//     assert_eq!(value.into::<i64>(type_mgr), 42);
// }

// #[test]
// fn test_raw_unboxed_values() {
//     let arena = bumpalo::Bump::new();
//     let type_mgr = TypeManager::new(&arena);

//     // Test integer value
//     let int_raw = RawValue { int_value: 100 };
//     let int_value = Value::from_raw(&arena, type_mgr.int(), int_raw);
//     assert_eq!(int_value.into::<i64>(type_mgr), 100);

//     // Test float value
//     let float_raw = RawValue { float_value: 3.14 };
//     let float_value = Value::from_raw(&arena, type_mgr.float(), float_raw);
//     assert_eq!(float_value.into::<f64>(type_mgr), 3.14);

//     // Test boolean value
//     let bool_raw = RawValue { bool_value: true };
//     let bool_value = Value::from_raw(&arena, type_mgr.bool(), bool_raw);
//     assert_eq!(bool_value.into::<bool>(type_mgr), true);
// }

#[test]
fn test_array_value() {
    let arena = bumpalo::Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let int_ty = type_mgr.int();
    let array_ty = type_mgr.array(int_ty);

    let raw_values = [
        RawValue { int_value: 1 },
        RawValue { int_value: 2 },
        RawValue { int_value: 3 },
    ];

    let array_data = ArrayData::new_with(&arena, &raw_values);

    let array_raw = RawValue {
        boxed: array_data as *const ArrayData as *const RawValue,
    };
    let array_value = Value {
        ty: array_ty,
        raw: array_raw,
        _phantom: PhantomData,
    };

    let array: Array<'_, f64> = array_value.get::<Array<f64>>(type_mgr).unwrap();

    assert_eq!(array.len(), 3);
    // assert_eq!(array.get(type_mgr, 0).unwrap(), 1);
    // assert_eq!(array.get(type_mgr, 1).unwrap(), 2);
    // assert_eq!(array.get(type_mgr, 2).unwrap(), 3);
}

struct List<T> {
    _phantom: PhantomData<T>,
}
impl<T> List<T> {
    pub fn new<U>() -> List<U> {
        List {
            _phantom: PhantomData,
        }
    }
}

fn foo() {
    let _: List<i64> = List::<i64>::new();
}
