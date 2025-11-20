//! Example demonstrating a custom visitor that counts Int types.
//!
//! Run with: cargo run --example int_counter

use melbi_types::{BoxBuilder, Scalar, TypeBuilder, TypeKind, TypeView, TypeVisitor};

/// Example visitor: Count occurrences of Int type.
pub struct IntCounter<B: TypeBuilder> {
    pub count: usize,
    pub builder: B,
}

impl<B: TypeBuilder> TypeVisitor<B> for IntCounter<B> {
    fn builder(&self) -> B {
        self.builder
    }

    fn visit(&mut self, ty: B::TypeView) {
        if matches!(ty.view(self.builder), TypeKind::Scalar(Scalar::Int)) {
            self.count += 1;
        }
        self.super_visit(ty);
    }
}

fn main() {
    println!("=== IntCounter Example ===\n");

    let builder = BoxBuilder::new();

    // Simple type with one Int
    let int_ty = TypeKind::Scalar(Scalar::Int).intern(builder);
    let mut counter = IntCounter { count: 0, builder };
    counter.visit(int_ty);
    println!("Int type contains {} Int(s)", counter.count);

    // Array[Int] - still just one Int
    let arr_int = TypeKind::Array(TypeKind::Scalar(Scalar::Int).intern(builder)).intern(builder);
    let mut counter = IntCounter { count: 0, builder };
    counter.visit(arr_int);
    println!("Array[Int] contains {} Int(s)", counter.count);

    // Array[Array[Int]] - still just one Int
    let arr_arr_int = TypeKind::Array(
        TypeKind::Array(TypeKind::Scalar(Scalar::Int).intern(builder)).intern(builder),
    )
    .intern(builder);
    let mut counter = IntCounter { count: 0, builder };
    counter.visit(arr_arr_int);
    println!("Array[Array[Int]] contains {} Int(s)", counter.count);

    // Map[Int, Bool] - one Int
    let map_int_bool = TypeKind::Map(
        TypeKind::Scalar(Scalar::Int).intern(builder),
        TypeKind::Scalar(Scalar::Bool).intern(builder),
    )
    .intern(builder);
    let mut counter = IntCounter { count: 0, builder };
    counter.visit(map_int_bool);
    println!("Map[Int, Bool] contains {} Int(s)", counter.count);

    // Function: (Int, Int) => Int - three Ints!
    let func_ty = TypeKind::Function {
        params: builder.intern_types([
            TypeKind::Scalar(Scalar::Int).intern(builder),
            TypeKind::Scalar(Scalar::Int).intern(builder),
        ]),
        ret: TypeKind::Scalar(Scalar::Int).intern(builder),
    }
    .intern(builder);
    let mut counter = IntCounter { count: 0, builder };
    counter.visit(func_ty);
    println!("(Int, Int) => Int contains {} Int(s)", counter.count);

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
    let mut counter = IntCounter { count: 0, builder };
    counter.visit(complex);
    println!(
        "Array[(Map[Int, Int]) => Array[Int]] contains {} Int(s)",
        counter.count
    );

    println!("\n=== Done ===");
}
