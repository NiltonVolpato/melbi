extern crate alloc;

use alloc::vec::Vec;
use bumpalo::Bump;
use melbi_core::{parser::parse, types::manager::TypeManager};
use postcard::to_allocvec;
use std::ops::Deref;

#[test]
fn test_postcard() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    // Test primitives
    let int_ty = type_mgr.int();
    let v = to_allocvec(int_ty).unwrap();
    let deserialized = type_mgr.deserialize_type(&v).unwrap();
    assert!(core::ptr::eq(deserialized, int_ty));
    println!("✓ Int round-trip");

    // Test Map
    let map_ty = type_mgr.map(type_mgr.int(), type_mgr.float());
    let v = to_allocvec(map_ty).unwrap();
    assert_eq!(&[6, 0, 1], v.deref());
    let deserialized = type_mgr.deserialize_type(&v).unwrap();
    assert!(core::ptr::eq(deserialized, map_ty));
    println!("✓ Map round-trip");

    // Test TypeVar
    let var_ty = type_mgr.map(type_mgr.fresh_type_var(), type_mgr.fresh_type_var());
    let v = to_allocvec(var_ty).unwrap();
    assert_eq!(&[6, 10, 0, 10, 1], v.deref());
    let deserialized = type_mgr.deserialize_type(&v).unwrap();
    assert!(core::ptr::eq(deserialized, var_ty));
    println!("✓ TypeVar round-trip");

    // Test Record
    let record_ty = type_mgr.record(vec![("name", type_mgr.str()), ("age", type_mgr.int())]);
    let v = to_allocvec(record_ty).unwrap();
    assert_eq!(
        &[7, 2, 3, 97, 103, 101, 0, 4, 110, 97, 109, 101, 3],
        v.deref()
    );
    let deserialized = type_mgr.deserialize_type(&v).unwrap();
    assert!(core::ptr::eq(deserialized, record_ty));
    println!("✓ Record round-trip");

    // Test Array
    let array_ty = type_mgr.array(type_mgr.bool());
    let v = to_allocvec(array_ty).unwrap();
    let deserialized = type_mgr.deserialize_type(&v).unwrap();
    assert!(core::ptr::eq(deserialized, array_ty));
    println!("✓ Array round-trip");

    // Test Function
    let func_ty = type_mgr.function(&[type_mgr.int(), type_mgr.str()], type_mgr.bool());
    let v = to_allocvec(func_ty).unwrap();
    let deserialized = type_mgr.deserialize_type(&v).unwrap();
    assert!(core::ptr::eq(deserialized, func_ty));
    println!("✓ Function round-trip");

    // Test Symbol
    let symbol_ty = type_mgr.symbol(vec!["success", "error", "pending"]);
    let v = to_allocvec(symbol_ty).unwrap();
    let deserialized = type_mgr.deserialize_type(&v).unwrap();
    assert!(core::ptr::eq(deserialized, symbol_ty));
    println!("✓ Symbol round-trip");

    // Test complex nested type
    let complex = type_mgr.function(
        &[type_mgr.map(type_mgr.str(), type_mgr.array(type_mgr.int()))],
        type_mgr.record(vec![("result", type_mgr.bool()), ("count", type_mgr.int())]),
    );
    let v = to_allocvec(complex).unwrap();
    let deserialized = type_mgr.deserialize_type(&v).unwrap();
    assert!(core::ptr::eq(deserialized, complex));
    println!("✓ Complex nested type round-trip");
}

#[test]
fn test_parsed_expr() {
    let arena = Bump::new();
    let input = "x + y where { x = 1, y = 2 }";
    let parsed = parse(&arena, input).unwrap();

    let v: Vec<u8> = to_allocvec(parsed.expr).unwrap();
    println!("{:#?}", parsed.expr);
    assert_eq!(
        &[
            9, 0, 0, 16, 1, 120, 16, 1, 121, 2, 1, 120, 15, 0, 2, 0, 1, 121, 15, 0, 4, 0
        ],
        v.deref()
    );
}
