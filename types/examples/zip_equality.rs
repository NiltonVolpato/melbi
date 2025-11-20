//! Example demonstrating the Zip trait for checking type equality.
//!
//! This shows how to implement a Zipper to compare two types structurally,
//! even if they come from different builders.
//!
//! Run with: cargo run --example zip_equality

use melbi_types::{BoxBuilder, Scalar, TypeBuilder, TypeKind, TypeView, Zip, Zipper};

/// A simple zipper that checks if two types are structurally equal.
struct EqualityZipper<B1: TypeBuilder, B2: TypeBuilder> {
    builder1: B1,
    builder2: B2,
}

impl<B1, B2> EqualityZipper<B1, B2>
where
    B1: TypeBuilder,
    B2: TypeBuilder,
{
    fn new(builder1: B1, builder2: B2) -> Self {
        Self { builder1, builder2 }
    }

    fn check_equal(ty1: B1::TypeView, builder1: B1, ty2: B2::TypeView, builder2: B2) -> bool {
        let mut zipper = Self::new(builder1, builder2);
        zipper.zip_tys(ty1, ty2).is_ok()
    }
}

impl<B1, B2> Zipper<B1, B2> for EqualityZipper<B1, B2>
where
    B1: TypeBuilder,
    B2: TypeBuilder,
{
    fn zip_tys(&mut self, a: B1::TypeView, b: B2::TypeView) -> Result<(), ()> {
        // Recursively zip the type structures
        Zip::zip_with(self, a.view(self.builder1), b.view(self.builder2))
    }

    fn builder1(&self) -> B1 {
        self.builder1
    }

    fn builder2(&self) -> B2 {
        self.builder2
    }
}

fn main() {
    println!("=== Zip Equality Checker Example ===\n");

    let builder = BoxBuilder::new();

    // Test 1: Same scalar types
    let int1 = TypeKind::Scalar(Scalar::Int).intern(builder);
    let int2 = TypeKind::Scalar(Scalar::Int).intern(builder);
    let equal = EqualityZipper::check_equal(int1, builder, int2, builder);
    println!("Int == Int: {}", equal);
    assert!(equal);

    // Test 2: Different scalar types
    let int_ty = TypeKind::Scalar(Scalar::Int).intern(builder);
    let bool_ty = TypeKind::Scalar(Scalar::Bool).intern(builder);
    let equal = EqualityZipper::check_equal(int_ty, builder, bool_ty, builder);
    println!("Int == Bool: {}", equal);
    assert!(!equal);

    // Test 3: Array types with same elements
    let arr_int1 = TypeKind::Array(TypeKind::Scalar(Scalar::Int).intern(builder)).intern(builder);
    let arr_int2 = TypeKind::Array(TypeKind::Scalar(Scalar::Int).intern(builder)).intern(builder);
    let equal = EqualityZipper::check_equal(arr_int1, builder, arr_int2, builder);
    println!("Array[Int] == Array[Int]: {}", equal);
    assert!(equal);

    // Test 4: Array types with different elements
    let arr_int = TypeKind::Array(TypeKind::Scalar(Scalar::Int).intern(builder)).intern(builder);
    let arr_bool = TypeKind::Array(TypeKind::Scalar(Scalar::Bool).intern(builder)).intern(builder);
    let equal = EqualityZipper::check_equal(arr_int, builder, arr_bool, builder);
    println!("Array[Int] == Array[Bool]: {}", equal);
    assert!(!equal);

    // Test 5: Nested arrays
    let nested1 = TypeKind::Array(
        TypeKind::Array(TypeKind::Scalar(Scalar::Int).intern(builder)).intern(builder),
    )
    .intern(builder);
    let nested2 = TypeKind::Array(
        TypeKind::Array(TypeKind::Scalar(Scalar::Int).intern(builder)).intern(builder),
    )
    .intern(builder);
    let equal = EqualityZipper::check_equal(nested1, builder, nested2, builder);
    println!("Array[Array[Int]] == Array[Array[Int]]: {}", equal);
    assert!(equal);

    // Test 6: Map types
    let map1 = TypeKind::Map(
        TypeKind::Scalar(Scalar::Int).intern(builder),
        TypeKind::Scalar(Scalar::Bool).intern(builder),
    )
    .intern(builder);
    let map2 = TypeKind::Map(
        TypeKind::Scalar(Scalar::Int).intern(builder),
        TypeKind::Scalar(Scalar::Bool).intern(builder),
    )
    .intern(builder);
    let equal = EqualityZipper::check_equal(map1, builder, map2, builder);
    println!("Map[Int, Bool] == Map[Int, Bool]: {}", equal);
    assert!(equal);

    // Test 7: Map with different value types
    let map_int_bool = TypeKind::Map(
        TypeKind::Scalar(Scalar::Int).intern(builder),
        TypeKind::Scalar(Scalar::Bool).intern(builder),
    )
    .intern(builder);
    let map_int_str = TypeKind::Map(
        TypeKind::Scalar(Scalar::Int).intern(builder),
        TypeKind::Scalar(Scalar::Str).intern(builder),
    )
    .intern(builder);
    let equal = EqualityZipper::check_equal(map_int_bool, builder, map_int_str, builder);
    println!("Map[Int, Bool] == Map[Int, Str]: {}", equal);
    assert!(!equal);

    // Test 8: Records with same fields
    let rec1 = TypeKind::Record(builder.intern_field_types([
        ("x", TypeKind::Scalar(Scalar::Int).intern(builder)),
        ("y", TypeKind::Scalar(Scalar::Bool).intern(builder)),
    ]))
    .intern(builder);
    let rec2 = TypeKind::Record(builder.intern_field_types([
        ("x", TypeKind::Scalar(Scalar::Int).intern(builder)),
        ("y", TypeKind::Scalar(Scalar::Bool).intern(builder)),
    ]))
    .intern(builder);
    let equal = EqualityZipper::check_equal(rec1, builder, rec2, builder);
    println!(
        "Record[x: Int, y: Bool] == Record[x: Int, y: Bool]: {}",
        equal
    );
    assert!(equal);

    // Test 9: Records with different field types
    let rec_int = TypeKind::Record(
        builder.intern_field_types([("value", TypeKind::Scalar(Scalar::Int).intern(builder))]),
    )
    .intern(builder);
    let rec_bool = TypeKind::Record(
        builder.intern_field_types([("value", TypeKind::Scalar(Scalar::Bool).intern(builder))]),
    )
    .intern(builder);
    let equal = EqualityZipper::check_equal(rec_int, builder, rec_bool, builder);
    println!("Record[value: Int] == Record[value: Bool]: {}", equal);
    assert!(!equal);

    // Test 10: Records with different field names
    let rec_x = TypeKind::Record(
        builder.intern_field_types([("x", TypeKind::Scalar(Scalar::Int).intern(builder))]),
    )
    .intern(builder);
    let rec_y = TypeKind::Record(
        builder.intern_field_types([("y", TypeKind::Scalar(Scalar::Int).intern(builder))]),
    )
    .intern(builder);
    let equal = EqualityZipper::check_equal(rec_x, builder, rec_y, builder);
    println!("Record[x: Int] == Record[y: Int]: {}", equal);
    assert!(!equal);

    // Test 11: Functions with same signature
    let func1 = TypeKind::Function {
        params: builder.intern_types([
            TypeKind::Scalar(Scalar::Int).intern(builder),
            TypeKind::Scalar(Scalar::Bool).intern(builder),
        ]),
        ret: TypeKind::Scalar(Scalar::Str).intern(builder),
    }
    .intern(builder);
    let func2 = TypeKind::Function {
        params: builder.intern_types([
            TypeKind::Scalar(Scalar::Int).intern(builder),
            TypeKind::Scalar(Scalar::Bool).intern(builder),
        ]),
        ret: TypeKind::Scalar(Scalar::Str).intern(builder),
    }
    .intern(builder);
    let equal = EqualityZipper::check_equal(func1, builder, func2, builder);
    println!("(Int, Bool) => Str == (Int, Bool) => Str: {}", equal);
    assert!(equal);

    // Test 12: Functions with different return types
    let func_str = TypeKind::Function {
        params: builder.intern_types([TypeKind::Scalar(Scalar::Int).intern(builder)]),
        ret: TypeKind::Scalar(Scalar::Str).intern(builder),
    }
    .intern(builder);
    let func_bool = TypeKind::Function {
        params: builder.intern_types([TypeKind::Scalar(Scalar::Int).intern(builder)]),
        ret: TypeKind::Scalar(Scalar::Bool).intern(builder),
    }
    .intern(builder);
    let equal = EqualityZipper::check_equal(func_str, builder, func_bool, builder);
    println!("(Int) => Str == (Int) => Bool: {}", equal);
    assert!(!equal);

    println!("\n=== All tests passed! ===");
}
