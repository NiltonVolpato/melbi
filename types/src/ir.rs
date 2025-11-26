//! Intermediate Representation (IR) for Melbi's type system.
//!
//! This module contains the generic, builder-agnostic representation of types.
//! The IR is parameterized by the `TypeBuilder` trait, which allows different
//! implementations to choose how types are built and stored in memory.
//!
//! ## Structure
//!
//! - **Core types**: `TypeKind`, `Ty` - the logical structure of types
//! - **TypeBuilder trait**: Abstract interface for type construction and storage
//! - **TypeView trait**: Unified view over types from different builders
//! - **Generic algorithms**: Visitor and transformer patterns over types
//! - **Display**: Pretty-printing support

pub mod builder;
pub mod display;
pub mod fold;
pub mod scalar;
pub mod ty;
pub mod view;
pub mod visit;
pub mod zip;

pub use builder::TypeBuilder;
pub use display::{TyDisplay, TypeFormatter, TypeKindDisplay};
pub use fold::TypeFolder;
pub use scalar::Scalar;
pub use ty::{Ty, TyData, TypeKind};
pub use view::{TypeChildren, TypeView, convert_ty, types_cmp, types_equal};
pub use visit::{ClosureVisitor, TypeVisitor};
pub use zip::{Zip, Zipper};
