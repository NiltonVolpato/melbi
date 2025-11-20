//! Traits for "zipping" types, walking through two structures and checking that they match.
//!
//! This module is inspired by Chalk's zip implementation but simplified for melbi-types' needs.
//! The main use case is type unification, but the infrastructure supports any operation that
//! needs to walk two types in parallel (equality checking, subtyping, comparison, etc.).
//!
//! ## Design
//!
//! The zip pattern separates two concerns:
//!
//! 1. **Structural recursion** - How to walk through type structures (`Zip` trait)
//! 2. **Leaf handling** - What to do when you reach matching elements (`Zipper` trait)
//!
//! ### The `Zipper` Trait
//!
//! Users implement this trait to define custom behavior. For example, an equality checker
//! just recursively zips everything. A unifier binds type variables and checks constraints.
//!
//! ### The `Zip` Trait
//!
//! Types implement this to define how to structurally walk themselves. For example,
//! `TypeKind` checks that variants match, then recursively zips child types.
//!
//! ## Example
//!
//! ```ignore
//! use melbi_types::{Zipper, Zip, TypeBuilder};
//!
//! struct EqualityChecker<B1, B2> {
//!     builder1: B1,
//!     builder2: B2,
//! }
//!
//! impl<B1, B2> Zipper<B1, B2> for EqualityChecker<B1, B2>
//! where
//!     B1: TypeBuilder,
//!     B2: TypeBuilder,
//! {
//!     fn zip_tys(&mut self, a: B1::TypeView, b: B2::TypeView) -> Result<(), ()> {
//!         Zip::zip_with(self, a.view(self.builder1), b.view(self.builder2))
//!     }
//!
//!     fn builder1(&self) -> B1 { self.builder1 }
//!     fn builder2(&self) -> B2 { self.builder2 }
//! }
//! ```

use super::{Scalar, TypeBuilder, TypeKind};

/// Callback trait for zipping two types.
///
/// Implement this trait to define custom behavior when walking two types in parallel.
/// The `Zip` trait implementations will call your `zip_tys` method whenever they
/// encounter matching type structures.
///
/// Common implementations:
/// - **Equality checker** - Recursively zip everything, fail on mismatch
/// - **Unifier** - Bind type variables, check constraints, merge types
/// - **Subtype checker** - Verify one type is a subtype of another
pub trait Zipper<B1: TypeBuilder, B2: TypeBuilder> {
    /// Called when two types are found in matching positions.
    ///
    /// The implementation should decide what to do with these types.
    /// Typically this involves recursively zipping their structure via
    /// `Zip::zip_with`.
    fn zip_tys(&mut self, a: B1::TypeView, b: B2::TypeView) -> Result<(), ()>;

    /// Get the first type builder.
    fn builder1(&self) -> B1;

    /// Get the second type builder.
    fn builder2(&self) -> B2;
}

/// Trait for types that can be structurally zipped.
///
/// This trait defines how to walk through a type's structure, ensuring that
/// two values match. When leaf values (like types themselves) are encountered,
/// the corresponding `Zipper` callback is invoked.
///
/// Most types should implement this with pattern matching that:
/// 1. Checks that variants/discriminants match
/// 2. Recursively zips child components
/// 3. Returns `Err(())` for mismatches
pub trait Zip<B1, B2, Other: ?Sized = Self>
where
    B1: TypeBuilder,
    B2: TypeBuilder,
{
    /// Uses the zipper to walk through two values, ensuring that they match.
    fn zip_with<Z: Zipper<B1, B2>>(zipper: &mut Z, a: &Self, b: &Other) -> Result<(), ()>;
}

// ============================================================================
// Generic Implementations
// ============================================================================

/// Zip for unit type - always matches.
impl<B1, B2> Zip<B1, B2> for ()
where
    B1: TypeBuilder,
    B2: TypeBuilder,
{
    fn zip_with<Z: Zipper<B1, B2>>(_: &mut Z, _: &Self, _: &Self) -> Result<(), ()> {
        Ok(())
    }
}

/// Zip for vectors - check length, then zip elements.
impl<T, B1, B2> Zip<B1, B2> for alloc::vec::Vec<T>
where
    T: Zip<B1, B2>,
    B1: TypeBuilder,
    B2: TypeBuilder,
{
    fn zip_with<Z: Zipper<B1, B2>>(zipper: &mut Z, a: &Self, b: &Self) -> Result<(), ()> {
        <[T] as Zip<B1, B2>>::zip_with(zipper, a, b)
    }
}

/// Zip for slices - check length, then zip elements.
impl<T, B1, B2> Zip<B1, B2> for [T]
where
    T: Zip<B1, B2>,
    B1: TypeBuilder,
    B2: TypeBuilder,
{
    fn zip_with<Z: Zipper<B1, B2>>(zipper: &mut Z, a: &Self, b: &Self) -> Result<(), ()> {
        if a.len() != b.len() {
            return Err(());
        }

        for (a_elem, b_elem) in a.iter().zip(b.iter()) {
            Zip::zip_with(zipper, a_elem, b_elem)?;
        }

        Ok(())
    }
}

/// Zip for Box - unwrap and zip contents.
impl<T, B1, B2> Zip<B1, B2> for alloc::boxed::Box<T>
where
    T: Zip<B1, B2>,
    B1: TypeBuilder,
    B2: TypeBuilder,
{
    fn zip_with<Z: Zipper<B1, B2>>(zipper: &mut Z, a: &Self, b: &Self) -> Result<(), ()> {
        <T as Zip<B1, B2>>::zip_with(zipper, a, b)
    }
}

/// Zip for Option - check both are Some/None, then zip contents.
impl<T, B1, B2> Zip<B1, B2> for Option<T>
where
    T: Zip<B1, B2>,
    B1: TypeBuilder,
    B2: TypeBuilder,
{
    fn zip_with<Z: Zipper<B1, B2>>(zipper: &mut Z, a: &Self, b: &Self) -> Result<(), ()> {
        match (a, b) {
            (Some(a), Some(b)) => Zip::zip_with(zipper, a, b),
            (None, None) => Ok(()),
            _ => Err(()),
        }
    }
}

// ============================================================================
// TypeKind Implementation
// ============================================================================

impl<B1, B2> Zip<B1, B2, TypeKind<B2>> for TypeKind<B1>
where
    B1: TypeBuilder,
    B2: TypeBuilder,
{
    fn zip_with<Z: Zipper<B1, B2>>(zipper: &mut Z, a: &Self, b: &TypeKind<B2>) -> Result<(), ()> {
        match (a, b) {
            // Type variables - must have same ID
            (TypeKind::TypeVar(id1), TypeKind::TypeVar(id2)) => {
                if id1 == id2 {
                    Ok(())
                } else {
                    Err(())
                }
            }

            // Scalars - must be identical
            (TypeKind::Scalar(s1), TypeKind::Scalar(s2)) => {
                if s1 == s2 {
                    Ok(())
                } else {
                    Err(())
                }
            }

            // Arrays - zip element types
            (TypeKind::Array(elem1), TypeKind::Array(elem2)) => {
                zipper.zip_tys(elem1.clone(), elem2.clone())
            }

            // Maps - zip key and value types
            (TypeKind::Map(key1, val1), TypeKind::Map(key2, val2)) => {
                zipper.zip_tys(key1.clone(), key2.clone())?;
                zipper.zip_tys(val1.clone(), val2.clone())
            }

            // Records - check fields match by name, then zip types
            (TypeKind::Record(fields1), TypeKind::Record(fields2)) => {
                let builder1 = zipper.builder1();
                let builder2 = zipper.builder2();
                let data1 = builder1.field_types_data(fields1);
                let data2 = builder2.field_types_data(fields2);

                // Must have same number of fields
                if data1.len() != data2.len() {
                    return Err(());
                }

                // Since fields are sorted by name during interning,
                // we can zip them directly and check names match
                for ((name1, ty1), (name2, ty2)) in data1.iter().zip(data2.iter()) {
                    // Field names must match
                    if name1.as_ref() != name2.as_ref() {
                        return Err(());
                    }

                    // Recursively zip field types
                    zipper.zip_tys(ty1.clone(), ty2.clone())?;
                }

                Ok(())
            }

            // Functions - zip parameter types and return type
            (
                TypeKind::Function {
                    params: params1,
                    ret: ret1,
                },
                TypeKind::Function {
                    params: params2,
                    ret: ret2,
                },
            ) => {
                let builder1 = zipper.builder1();
                let builder2 = zipper.builder2();
                let param_data1 = builder1.types_data(params1);
                let param_data2 = builder2.types_data(params2);

                // Must have same number of parameters
                if param_data1.len() != param_data2.len() {
                    return Err(());
                }

                // Zip all parameter types
                for (param1, param2) in param_data1.iter().zip(param_data2.iter()) {
                    zipper.zip_tys(param1.clone(), param2.clone())?;
                }

                // Zip return types
                zipper.zip_tys(ret1.clone(), ret2.clone())
            }

            // Symbols - check parts match
            (TypeKind::Symbol(parts1), TypeKind::Symbol(parts2)) => {
                let builder1 = zipper.builder1();
                let builder2 = zipper.builder2();
                let data1 = builder1.symbol_parts_data(parts1);
                let data2 = builder2.symbol_parts_data(parts2);

                // Must have same number of parts
                if data1.len() != data2.len() {
                    return Err(());
                }

                // All parts must match exactly
                for (part1, part2) in data1.iter().zip(data2.iter()) {
                    if part1.as_ref() != part2.as_ref() {
                        return Err(());
                    }
                }

                Ok(())
            }

            // Mismatched variants - incompatible types
            _ => Err(()),
        }
    }
}

// ============================================================================
// Scalar Implementation (for completeness)
// ============================================================================

impl<B1, B2> Zip<B1, B2> for Scalar
where
    B1: TypeBuilder,
    B2: TypeBuilder,
{
    fn zip_with<Z: Zipper<B1, B2>>(_zipper: &mut Z, a: &Self, b: &Self) -> Result<(), ()> {
        if a == b { Ok(()) } else { Err(()) }
    }
}
