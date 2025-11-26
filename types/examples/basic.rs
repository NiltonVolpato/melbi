//! Basic example demonstrating both ArenaBuilder and BoxBuilder.
//!
//! Run with: cargo run --example basic

use bumpalo::Bump;
use melbi_types::{
    ArenaBuilder, BoxBuilder, Scalar, TyDisplay, TypeBuilder, TypeFolder, TypeKind, TypeView,
    TypeVisitor,
};

fn main() {
    println!("=== Melbi Types Example ===\n");

    // Example 1: Using BoxBuilder (simple, Rc-based)
    println!("1. BoxBuilder (reference counting):");
    let box_builder = BoxBuilder::new();

    let int_ty = TypeKind::Scalar(Scalar::Int).intern(box_builder);
    let bool_ty = TypeKind::Scalar(Scalar::Bool).intern(box_builder);
    let arr_int = TypeKind::Array(int_ty.clone()).intern(box_builder);
    let arr_arr_bool =
        TypeKind::Array(TypeKind::Array(bool_ty.clone()).intern(box_builder)).intern(box_builder);

    println!("   Int: {}", int_ty.display(box_builder));
    println!("   Bool: {}", bool_ty.display(box_builder));
    println!("   Array[Int]: {}", arr_int.display(box_builder));
    println!(
        "   Array[Array[Bool]]: {}",
        arr_arr_bool.display(box_builder)
    );

    // Example 2: Using ArenaBuilder (arena allocation)
    println!("\n2. ArenaBuilder (arena allocation):");
    let arena = Bump::new();
    let arena_builder = ArenaBuilder::new(&arena);

    let int_ty = TypeKind::Scalar(Scalar::Int).intern(arena_builder);
    let bool_ty = TypeKind::Scalar(Scalar::Bool).intern(arena_builder);
    let arr_int = TypeKind::Array(int_ty).intern(arena_builder);

    println!("   Int: {}", int_ty.display(arena_builder));
    println!("   Bool: {}", bool_ty.display(arena_builder));
    println!("   Array[Int]: {}", arr_int.display(arena_builder));

    // Example 3: Using the visitor pattern
    println!("\n3. Visitor Pattern - Counting Int types:");
    struct IntCounter<B: TypeBuilder> {
        count: usize,
        builder: B,
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

    let complex_ty = TypeKind::Array(
        TypeKind::Array(TypeKind::Scalar(Scalar::Int).intern(box_builder)).intern(box_builder),
    )
    .intern(box_builder);
    let mut counter = IntCounter {
        count: 0,
        builder: box_builder,
    };
    counter.visit(complex_ty.clone());
    println!("   Type: {}", complex_ty.display(box_builder));
    println!("   Contains {} Int types", counter.count);

    // Example 4: Using the folder pattern to transform types
    println!("\n4. Folder Pattern - Replace Int with Bool:");
    struct IntToBoolFolder {
        builder: BoxBuilder,
    }

    impl TypeFolder<BoxBuilder> for IntToBoolFolder {
        fn builder(&self) -> BoxBuilder {
            self.builder
        }

        fn fold_ty(
            &mut self,
            ty: <BoxBuilder as TypeBuilder>::TypeView,
        ) -> <BoxBuilder as TypeBuilder>::TypeView {
            if matches!(ty.view(self.builder), TypeKind::Scalar(Scalar::Int)) {
                TypeKind::Scalar(Scalar::Bool).intern(self.builder)
            } else {
                self.super_fold_ty(ty)
            }
        }
    }

    let original =
        TypeKind::Array(TypeKind::Scalar(Scalar::Int).intern(box_builder)).intern(box_builder);
    println!("   Original: {}", original.display(box_builder));

    let mut folder = IntToBoolFolder {
        builder: box_builder,
    };
    let transformed = folder.fold_ty(original);
    println!("   Transformed: {}", transformed.display(box_builder));

    // Example 5: Type checking methods
    println!("\n5. Type Checking Methods:");
    let arr_bool =
        TypeKind::Array(TypeKind::Scalar(Scalar::Bool).intern(box_builder)).intern(box_builder);
    println!("   Type: {}", arr_bool.display(box_builder));
    println!("   is_array: {}", arr_bool.is_array(box_builder));
    println!("   is_int: {}", arr_bool.is_int(box_builder));
    println!("   is_bool: {}", arr_bool.is_bool(box_builder));

    println!("\n=== Done ===");
}
