//! Melbi type system with pluggable type builders.
//!
//! This crate provides a generic type representation that works with
//! different storage strategies (arena, RC-based, encoded, etc.).
//!
//! # Example
//!
//! ```ignore
//! use melbi_types::{TypeBuilder, ArenaBuilder, Scalar};
//! use bumpalo::Bump;
//!
//! let arena = Bump::new();
//! let builder = ArenaBuilder::new(&arena);
//!
//! let int_ty = builder.int();
//! let arr_ty = builder.array(int_ty);
//! ```

#![no_std]
extern crate alloc;

// Intermediate Representation - generic type system
pub mod ir;

// Concrete builder implementations
pub mod arena_builder;
pub mod box_builder;

// Re-export IR types for convenience
pub use ir::{
    ClosureVisitor, Scalar, Ty, TyData, TyDisplay, TypeBuilder, TypeChildren, TypeFolder,
    TypeFormatter, TypeKind, TypeKindDisplay, TypeView, TypeVisitor, Zip, Zipper, convert_ty,
    types_cmp, types_equal,
};

// Re-export concrete builders
pub use arena_builder::ArenaBuilder;
pub use box_builder::BoxBuilder;
