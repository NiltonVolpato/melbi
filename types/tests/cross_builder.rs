//! Tests for cross-interner operations (comparison, conversion, etc.)

use bumpalo::Bump;
use melbi_types::{
    ArenaBuilder, BoxBuilder, Scalar, TypeBuilder, TypeKind, convert_ty, types_cmp, types_equal,
};
use std::cmp::Ordering;

#[test]
fn test_types_equal_scalars() {
    let arena = Bump::new();
    let arena_int = ArenaBuilder::new(&arena);
    let box_int = BoxBuilder::new();

    // Create Int in both interners
    let int1 = TypeKind::Scalar(Scalar::Int).intern(arena_int);
    let int2 = TypeKind::Scalar(Scalar::Int).intern(box_int);

    assert!(types_equal(int1, arena_int, int2, box_int));

    // Create Float in box interner
    let float2 = TypeKind::Scalar(Scalar::Float).intern(box_int);

    assert!(!types_equal(int1, arena_int, float2, box_int));
}

#[test]
fn test_types_equal_arrays() {
    let arena = Bump::new();
    let arena_int = ArenaBuilder::new(&arena);
    let box_int = BoxBuilder::new();

    // Create Array[Int] in both interners
    let int1 = TypeKind::Scalar(Scalar::Int).intern(arena_int);
    let arr1 = TypeKind::Array(int1).intern(arena_int);

    let int2 = TypeKind::Scalar(Scalar::Int).intern(box_int);
    let arr2 = TypeKind::Array(int2).intern(box_int);

    assert!(types_equal(arr1, arena_int, arr2, box_int));

    // Create Array[Float] in box interner
    let float2 = TypeKind::Scalar(Scalar::Float).intern(box_int);
    let arr_float = TypeKind::Array(float2).intern(box_int);

    assert!(!types_equal(arr1, arena_int, arr_float, box_int));
}

#[test]
fn test_types_equal_maps() {
    let arena = Bump::new();
    let arena_int = ArenaBuilder::new(&arena);
    let box_int = BoxBuilder::new();

    // Create Map[Str, Int] in both interners
    let str1 = TypeKind::Scalar(Scalar::Str).intern(arena_int);
    let int1 = TypeKind::Scalar(Scalar::Int).intern(arena_int);
    let map1 = TypeKind::Map(str1, int1).intern(arena_int);

    let str2 = TypeKind::Scalar(Scalar::Str).intern(box_int);
    let int2 = TypeKind::Scalar(Scalar::Int).intern(box_int);
    let map2 = TypeKind::Map(str2.clone(), int2.clone()).intern(box_int);

    assert!(types_equal(map1, arena_int, map2, box_int));

    // Create Map[Int, Str] in box interner (swapped key/value)
    let map_swapped = TypeKind::Map(int2, str2).intern(box_int);

    assert!(!types_equal(map1, arena_int, map_swapped, box_int));
}

#[test]
fn test_types_equal_records() {
    let arena = Bump::new();
    let arena_int = ArenaBuilder::new(&arena);
    let box_int = BoxBuilder::new();

    // Create Record[x: Int, y: Float] in both interners
    let int1 = TypeKind::Scalar(Scalar::Int).intern(arena_int);
    let float1 = TypeKind::Scalar(Scalar::Float).intern(arena_int);
    let rec1 = TypeKind::Record(arena_int.intern_field_types([("x", int1), ("y", float1)]))
        .intern(arena_int);

    let int2 = TypeKind::Scalar(Scalar::Int).intern(box_int);
    let float2 = TypeKind::Scalar(Scalar::Float).intern(box_int);
    let rec2 =
        TypeKind::Record(box_int.intern_field_types([("x", int2.clone()), ("y", float2.clone())]))
            .intern(box_int);

    assert!(types_equal(rec1, arena_int, rec2, box_int));

    // Create Record[x: Float, y: Int] (swapped types)
    let rec_swapped =
        TypeKind::Record(box_int.intern_field_types([("x", float2.clone()), ("y", int2.clone())]))
            .intern(box_int);

    assert!(!types_equal(rec1, arena_int, rec_swapped, box_int));

    // Create Record[a: Int, y: Float] (different field name)
    let rec_diff_name =
        TypeKind::Record(box_int.intern_field_types([("a", int2), ("y", float2)])).intern(box_int);

    assert!(!types_equal(rec1, arena_int, rec_diff_name, box_int));
}

#[test]
fn test_types_equal_functions() {
    let arena = Bump::new();
    let arena_int = ArenaBuilder::new(&arena);
    let box_int = BoxBuilder::new();

    // Create (Int, Float) => Bool in both interners
    let int1 = TypeKind::Scalar(Scalar::Int).intern(arena_int);
    let float1 = TypeKind::Scalar(Scalar::Float).intern(arena_int);
    let bool1 = TypeKind::Scalar(Scalar::Bool).intern(arena_int);
    let func1 = TypeKind::Function {
        params: arena_int.intern_types([int1, float1]),
        ret: bool1,
    }
    .intern(arena_int);

    let int2 = TypeKind::Scalar(Scalar::Int).intern(box_int);
    let float2 = TypeKind::Scalar(Scalar::Float).intern(box_int);
    let bool2 = TypeKind::Scalar(Scalar::Bool).intern(box_int);
    let func2 = TypeKind::Function {
        params: box_int.intern_types([int2.clone(), float2.clone()]),
        ret: bool2.clone(),
    }
    .intern(box_int);

    assert!(types_equal(func1, arena_int, func2, box_int));

    // Create (Float, Int) => Bool (swapped params)
    let func_swapped = TypeKind::Function {
        params: box_int.intern_types([float2, int2]),
        ret: bool2,
    }
    .intern(box_int);

    assert!(!types_equal(func1, arena_int, func_swapped, box_int));
}

#[test]
fn test_types_equal_symbols() {
    let arena = Bump::new();
    let arena_int = ArenaBuilder::new(&arena);
    let box_int = BoxBuilder::new();

    // Create Symbol[error|pending|success] in both interners
    let sym1 = TypeKind::Symbol(arena_int.intern_symbol_parts(["error", "pending", "success"]))
        .intern(arena_int);

    let sym2 = TypeKind::Symbol(box_int.intern_symbol_parts(["error", "pending", "success"]))
        .intern(box_int);

    assert!(types_equal(sym1, arena_int, sym2, box_int));

    // Create Symbol[error|success] (missing "pending")
    let sym_diff =
        TypeKind::Symbol(box_int.intern_symbol_parts(["error", "success"])).intern(box_int);

    assert!(!types_equal(sym1, arena_int, sym_diff, box_int));
}

#[test]
fn test_types_equal_nested() {
    let arena = Bump::new();
    let arena_int = ArenaBuilder::new(&arena);
    let box_int = BoxBuilder::new();

    // Create complex nested type: Array[Map[Str, Record[x: Int]]]
    let int1 = TypeKind::Scalar(Scalar::Int).intern(arena_int);
    let str1 = TypeKind::Scalar(Scalar::Str).intern(arena_int);
    let rec1 = TypeKind::Record(arena_int.intern_field_types([("x", int1)])).intern(arena_int);
    let map1 = TypeKind::Map(str1, rec1).intern(arena_int);
    let arr1 = TypeKind::Array(map1).intern(arena_int);

    let int2 = TypeKind::Scalar(Scalar::Int).intern(box_int);
    let str2 = TypeKind::Scalar(Scalar::Str).intern(box_int);
    let rec2 = TypeKind::Record(box_int.intern_field_types([("x", int2)])).intern(box_int);
    let map2 = TypeKind::Map(str2, rec2).intern(box_int);
    let arr2 = TypeKind::Array(map2).intern(box_int);

    assert!(types_equal(arr1, arena_int, arr2, box_int));
}

#[test]
fn test_types_cmp_scalars() {
    let box_int = BoxBuilder::new();

    let int = TypeKind::Scalar(Scalar::Int).intern(box_int);
    let float = TypeKind::Scalar(Scalar::Float).intern(box_int);
    let bool_ty = TypeKind::Scalar(Scalar::Bool).intern(box_int);

    // Scalar ordering: Bool < Int < Float < Str < Bytes
    assert_eq!(
        types_cmp(bool_ty, box_int, int.clone(), box_int),
        Ordering::Less
    );
    assert_eq!(
        types_cmp(int.clone(), box_int, float, box_int),
        Ordering::Less
    );
    assert_eq!(
        types_cmp(int.clone(), box_int, int, box_int),
        Ordering::Equal
    );
}

#[test]
fn test_types_cmp_kinds() {
    let box_int = BoxBuilder::new();

    let typevar = TypeKind::TypeVar(0).intern(box_int);
    let scalar = TypeKind::Scalar(Scalar::Int).intern(box_int);
    let array = TypeKind::Array(scalar.clone()).intern(box_int);

    // Kind ordering: TypeVar < Scalar < Array < Map < Record < Function < Symbol
    assert_eq!(
        types_cmp(typevar, box_int, scalar.clone(), box_int),
        Ordering::Less
    );
    assert_eq!(types_cmp(scalar, box_int, array, box_int), Ordering::Less);
}

#[test]
fn test_convert_ty_simple() {
    let arena = Bump::new();
    let arena_int = ArenaBuilder::new(&arena);
    let box_int = BoxBuilder::new();

    // Convert Int from arena to box
    let int_arena = TypeKind::Scalar(Scalar::Int).intern(arena_int);
    let int_box = convert_ty(int_arena, arena_int, box_int);

    // Should be structurally equal
    assert!(types_equal(int_arena, arena_int, int_box.clone(), box_int));

    // Verify it's actually in the box interner
    assert!(int_box.is_int(box_int));
}

#[test]
fn test_convert_ty_array() {
    let arena = Bump::new();
    let arena_int = ArenaBuilder::new(&arena);
    let box_int = BoxBuilder::new();

    // Convert Array[Int] from arena to box
    let int_arena = TypeKind::Scalar(Scalar::Int).intern(arena_int);
    let arr_arena = TypeKind::Array(int_arena).intern(arena_int);

    let arr_box = convert_ty(arr_arena, arena_int, box_int);

    assert!(types_equal(arr_arena, arena_int, arr_box.clone(), box_int));
    assert!(arr_box.is_array(box_int));
}

#[test]
fn test_convert_ty_record() {
    let arena = Bump::new();
    let arena_int = ArenaBuilder::new(&arena);
    let box_int = BoxBuilder::new();

    // Convert Record[name: Str, age: Int] from arena to box
    let str_arena = TypeKind::Scalar(Scalar::Str).intern(arena_int);
    let int_arena = TypeKind::Scalar(Scalar::Int).intern(arena_int);
    let rec_arena =
        TypeKind::Record(arena_int.intern_field_types([("name", str_arena), ("age", int_arena)]))
            .intern(arena_int);

    let rec_box = convert_ty(rec_arena, arena_int, box_int);

    assert!(types_equal(rec_arena, arena_int, rec_box.clone(), box_int));
    assert!(rec_box.is_record(box_int));

    // Verify field names are preserved
    if let TypeKind::Record(fields) = rec_box.kind(box_int) {
        let field_data = box_int.field_types_data(fields);
        assert_eq!(field_data.len(), 2);
        assert_eq!(field_data[0].0.as_ref(), "age"); // Sorted!
        assert_eq!(field_data[1].0.as_ref(), "name");
    } else {
        panic!("Expected Record");
    }
}

#[test]
fn test_convert_ty_function() {
    let arena = Bump::new();
    let arena_int = ArenaBuilder::new(&arena);
    let box_int = BoxBuilder::new();

    // Convert (Str, Int) => Bool from arena to box
    let str_arena = TypeKind::Scalar(Scalar::Str).intern(arena_int);
    let int_arena = TypeKind::Scalar(Scalar::Int).intern(arena_int);
    let bool_arena = TypeKind::Scalar(Scalar::Bool).intern(arena_int);
    let func_arena = TypeKind::Function {
        params: arena_int.intern_types([str_arena, int_arena]),
        ret: bool_arena,
    }
    .intern(arena_int);

    let func_box = convert_ty(func_arena, arena_int, box_int);

    assert!(types_equal(
        func_arena,
        arena_int,
        func_box.clone(),
        box_int
    ));
    assert!(func_box.is_function(box_int));
}

#[test]
fn test_convert_ty_complex_nested() {
    let arena = Bump::new();
    let arena_int = ArenaBuilder::new(&arena);
    let box_int = BoxBuilder::new();

    // Create: Map[Str, Array[Record[x: Int, y: Float]]]
    let str_arena = TypeKind::Scalar(Scalar::Str).intern(arena_int);
    let int_arena = TypeKind::Scalar(Scalar::Int).intern(arena_int);
    let float_arena = TypeKind::Scalar(Scalar::Float).intern(arena_int);
    let rec_arena =
        TypeKind::Record(arena_int.intern_field_types([("x", int_arena), ("y", float_arena)]))
            .intern(arena_int);
    let arr_arena = TypeKind::Array(rec_arena).intern(arena_int);
    let map_arena = TypeKind::Map(str_arena, arr_arena).intern(arena_int);

    // Convert to box interner
    let map_box = convert_ty(map_arena, arena_int, box_int);

    // Verify structural equality
    assert!(types_equal(map_arena, arena_int, map_box.clone(), box_int));

    // Verify structure is preserved
    if let TypeKind::Map(key, val) = map_box.kind(box_int) {
        assert!(key.is_scalar(box_int));
        assert!(val.is_array(box_int));

        if let TypeKind::Array(elem) = val.kind(box_int) {
            assert!(elem.is_record(box_int));
        } else {
            panic!("Expected Array");
        }
    } else {
        panic!("Expected Map");
    }
}

#[test]
fn test_round_trip_conversion() {
    let arena = Bump::new();
    let arena_int = ArenaBuilder::new(&arena);
    let box_int = BoxBuilder::new();

    // Create type in arena
    let int_arena = TypeKind::Scalar(Scalar::Int).intern(arena_int);
    let arr_arena = TypeKind::Array(int_arena).intern(arena_int);

    // Convert to box
    let arr_box = convert_ty(arr_arena, arena_int, box_int);

    // Convert back to arena
    let arr_arena2 = convert_ty(arr_box, box_int, arena_int);

    // Both arena versions should be structurally equal
    assert!(types_equal(arr_arena, arena_int, arr_arena2, arena_int));
}
