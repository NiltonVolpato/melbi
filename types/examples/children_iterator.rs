//! Example demonstrating the children() iterator for TypeView.
//!
//! Run with: cargo run --example children_iterator

use melbi_types::{BoxBuilder, Scalar, TypeBuilder, TypeKind, TypeView};

fn main() {
    println!("=== TypeView children() Iterator Example ===\n");

    let builder = BoxBuilder::new();

    // Scalar types have no children
    let int_ty = TypeKind::Scalar(Scalar::Int).intern(builder);
    println!("Int type:");
    println!("  Children count: {}", int_ty.children(builder).count());

    // Array has one child (element type)
    let arr_int = TypeKind::Array(TypeKind::Scalar(Scalar::Int).intern(builder)).intern(builder);
    println!("\nArray[Int]:");
    println!("  Children count: {}", arr_int.children(builder).count());
    for (i, child) in arr_int.children(builder).enumerate() {
        println!(
            "  Child {}: {}",
            i,
            if matches!(child.view(builder), TypeKind::Scalar(Scalar::Int)) {
                "Int"
            } else {
                "Other"
            }
        );
    }

    // Map has two children (key and value)
    let map_ty = TypeKind::Map(
        TypeKind::Scalar(Scalar::Int).intern(builder),
        TypeKind::Scalar(Scalar::Bool).intern(builder),
    )
    .intern(builder);
    println!("\nMap[Int, Bool]:");
    println!("  Children count: {}", map_ty.children(builder).count());
    let children: Vec<_> = map_ty.children(builder).collect();
    println!("  Child 0 is Int: {}", children[0].is_int(builder));
    println!("  Child 1 is Bool: {}", children[1].is_bool(builder));

    // Function has params + return (multiple children)
    let func_ty = TypeKind::Function {
        params: builder.intern_types([
            TypeKind::Scalar(Scalar::Int).intern(builder),
            TypeKind::Scalar(Scalar::Bool).intern(builder),
            TypeKind::Scalar(Scalar::Str).intern(builder),
        ]),
        ret: TypeKind::Scalar(Scalar::Float).intern(builder),
    }
    .intern(builder);
    println!("\n(Int, Bool, Str) => Float:");
    println!("  Children count: {}", func_ty.children(builder).count());
    println!("  All children:");
    for (i, child) in func_ty.children(builder).enumerate() {
        let name = match child.view(builder) {
            TypeKind::Scalar(Scalar::Int) => "Int",
            TypeKind::Scalar(Scalar::Bool) => "Bool",
            TypeKind::Scalar(Scalar::Str) => "Str",
            TypeKind::Scalar(Scalar::Float) => "Float",
            _ => "Other",
        };
        println!("    {}: {}", i, name);
    }

    // Record with fields
    let record_ty = TypeKind::Record(builder.intern_field_types([
        ("x", TypeKind::Scalar(Scalar::Int).intern(builder)),
        ("y", TypeKind::Scalar(Scalar::Bool).intern(builder)),
        ("z", TypeKind::Scalar(Scalar::Float).intern(builder)),
    ]))
    .intern(builder);
    println!("\nRecord[x: Int, y: Bool, z: Float]:");
    println!("  Children count: {}", record_ty.children(builder).count());

    // Complex nested type
    let complex = TypeKind::Array(
        TypeKind::Function {
            params: builder.intern_types([TypeKind::Map(
                TypeKind::Scalar(Scalar::Int).intern(builder),
                TypeKind::Scalar(Scalar::Bool).intern(builder),
            )
            .intern(builder)]),
            ret: TypeKind::Scalar(Scalar::Str).intern(builder),
        }
        .intern(builder),
    )
    .intern(builder);
    println!("\nArray[(Map[Int, Bool]) => Str]:");
    println!(
        "  Direct children count: {}",
        complex.children(builder).count()
    );

    // Get the function type (only child of array)
    if let Some(func) = complex.children(builder).next() {
        println!(
            "  Function's children count: {}",
            func.children(builder).count()
        );

        // Get first param (Map type)
        if let Some(map_param) = func.children(builder).next() {
            println!(
                "    Map parameter's children count: {}",
                map_param.children(builder).count()
            );
        }
    }

    println!("\n=== Demonstrating ExactSizeIterator ===\n");

    // size_hint gives exact size
    let children_iter = func_ty.children(builder);
    let (lower, upper) = children_iter.size_hint();
    println!("Function children size_hint: ({}, {:?})", lower, upper);
    println!("len() method: {}", func_ty.children(builder).len());

    println!("\n=== Done ===");
}
