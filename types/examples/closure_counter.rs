//! Example demonstrating ClosureVisitor for counting Int types.
//!
//! This shows how ClosureVisitor can simplify visitor implementations
//! when you don't need custom traversal control.
//!
//! Run with: cargo run --example closure_counter

use melbi_types::{
    BoxBuilder, ClosureVisitor, Scalar, TypeBuilder, TypeKind, TypeView, TypeVisitor,
};

fn main() {
    println!("=== ClosureVisitor Counter Example ===\n");

    let builder = BoxBuilder::new();

    // Simple type with one Int
    let int_ty = TypeKind::Scalar(Scalar::Int).intern(builder);
    let mut count = 0;
    let mut visitor = ClosureVisitor::new(builder, |ty| {
        if matches!(ty.view(builder), TypeKind::Scalar(Scalar::Int)) {
            count += 1;
        }
        false // Continue traversing
    });
    visitor.visit(int_ty);
    println!("Int type contains {} Int(s)", count);

    // Array[Array[Int]] - still just one Int
    let arr_arr_int = TypeKind::Array(
        TypeKind::Array(TypeKind::Scalar(Scalar::Int).intern(builder)).intern(builder),
    )
    .intern(builder);
    let mut count = 0;
    let mut visitor = ClosureVisitor::new(builder, |ty| {
        if matches!(ty.view(builder), TypeKind::Scalar(Scalar::Int)) {
            count += 1;
        }
        false
    });
    visitor.visit(arr_arr_int);
    println!("Array[Array[Int]] contains {} Int(s)", count);

    // Function: (Int, Int) => Int - three Ints!
    let func_ty = TypeKind::Function {
        params: builder.intern_types([
            TypeKind::Scalar(Scalar::Int).intern(builder),
            TypeKind::Scalar(Scalar::Int).intern(builder),
        ]),
        ret: TypeKind::Scalar(Scalar::Int).intern(builder),
    }
    .intern(builder);
    let mut count = 0;
    let mut visitor = ClosureVisitor::new(builder, |ty| {
        if matches!(ty.view(builder), TypeKind::Scalar(Scalar::Int)) {
            count += 1;
        }
        false
    });
    visitor.visit(func_ty);
    println!("(Int, Int) => Int contains {} Int(s)", count);

    // Complex nested type
    let complex = TypeKind::Array(
        TypeKind::Function {
            params: builder.intern_types([TypeKind::Map(
                TypeKind::Scalar(Scalar::Int).intern(builder),
                TypeKind::Scalar(Scalar::Int).intern(builder),
            )
            .intern(builder)]),
            ret: TypeKind::Array(TypeKind::Scalar(Scalar::Int).intern(builder)).intern(builder),
        }
        .intern(builder),
    )
    .intern(builder);
    let mut count = 0;
    let mut visitor = ClosureVisitor::new(builder, |ty| {
        if matches!(ty.view(builder), TypeKind::Scalar(Scalar::Int)) {
            count += 1;
        }
        false
    });
    visitor.visit(complex.clone());
    println!(
        "Array[(Map[Int, Int]) => Array[Int]] contains {} Int(s)",
        count
    );

    println!("\n=== Early Stopping Example ===\n");

    // Find if ANY Int exists (stop as soon as we find one)
    let mut found_int = false;
    let mut nodes_visited = 0;
    let mut visitor = ClosureVisitor::new(builder, |ty| {
        nodes_visited += 1;
        if matches!(ty.view(builder), TypeKind::Scalar(Scalar::Int)) {
            found_int = true;
            true // Stop traversing!
        } else {
            false // Keep going
        }
    });
    visitor.visit(complex);
    println!("Found Int: {}", found_int);
    println!("Nodes visited before stopping: {}", nodes_visited);
    println!("(Without early stopping, would visit all nested types)");

    println!("\n=== Done ===");
}
