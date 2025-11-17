//! Cross-interner operations for types.
//!
//! This module provides functions for:
//! - Structural comparison of types across different interners
//! - Type conversion between interner representations
//! - Type ordering across interners
//!
//! These operations work by recursively traversing type structures and
//! comparing/converting them independently of the interner used.

use super::{Ty, TyData, TypeBuilder, TypeKind};
use core::cmp::Ordering;

/// Structurally compare two types from potentially different interners.
///
/// Returns `true` if the types have the same structure, regardless of
/// which interners were used to create them.
///
/// # Example
///
/// ```ignore
/// use melbi_types::{types_equal, TyKind, Scalar, ArenaBuilder, BoxBuilder};
/// use bumpalo::Bump;
///
/// let arena = Bump::new();
/// let arena_int = ArenaBuilder::new(&arena);
/// let box_int = BoxBuilder::new();
///
/// // Create Array[Int] in both interners
/// let int1 = TypeKind::Scalar(Scalar::Int).intern(arena_int);
/// let arr1 = TypeKind::Array(int1).intern(arena_int);
///
/// let int2 = TypeKind::Scalar(Scalar::Int).intern(box_int);
/// let arr2 = TypeKind::Array(int2).intern(box_int);
///
/// assert!(types_equal(arr1, arena_int, arr2, box_int));
/// ```
pub fn types_equal<I1: TypeBuilder, I2: TypeBuilder>(
    ty1: I1::TypeView,
    builder1: I1,
    ty2: I2::TypeView,
    builder2: I2,
) -> bool {
    match (ty1.view(builder1), ty2.view(builder2)) {
        // Base cases - direct comparison
        (TypeKind::TypeVar(id1), TypeKind::TypeVar(id2)) => id1 == id2,
        (TypeKind::Scalar(s1), TypeKind::Scalar(s2)) => s1 == s2,

        // Arrays - compare element types recursively
        (TypeKind::Array(e1), TypeKind::Array(e2)) => {
            types_equal(e1.clone(), builder1, e2.clone(), builder2)
        }

        // Maps - compare key and value types recursively
        (TypeKind::Map(k1, v1), TypeKind::Map(k2, v2)) => {
            types_equal(k1.clone(), builder1, k2.clone(), builder2)
                && types_equal(v1.clone(), builder1, v2.clone(), builder2)
        }

        // Records - compare field names and types
        (TypeKind::Record(fields1), TypeKind::Record(fields2)) => {
            let data1 = builder1.field_types_data(fields1);
            let data2 = builder2.field_types_data(fields2);

            // Must have same number of fields
            if data1.len() != data2.len() {
                return false;
            }

            // Compare each field (already sorted by name during interning)
            data1.iter().zip(data2.iter()).all(|((n1, t1), (n2, t2))| {
                // Field names must match
                n1.as_ref() == n2.as_ref()
                    // Field types must be structurally equal
                    && types_equal(t1.clone(), builder1, t2.clone(), builder2)
            })
        }

        // Functions - compare parameters and return type
        (
            TypeKind::Function {
                params: p1,
                ret: r1,
            },
            TypeKind::Function {
                params: p2,
                ret: r2,
            },
        ) => {
            let params1 = builder1.types_data(p1);
            let params2 = builder2.types_data(p2);

            // Must have same number of parameters
            if params1.len() != params2.len() {
                return false;
            }

            // All parameters must be structurally equal
            let params_match = params1
                .iter()
                .zip(params2.iter())
                .all(|(t1, t2)| types_equal(t1.clone(), builder1, t2.clone(), builder2));

            // Return types must be structurally equal
            let ret_match = types_equal(r1.clone(), builder1, r2.clone(), builder2);

            params_match && ret_match
        }

        // Symbols - compare parts
        (TypeKind::Symbol(parts1), TypeKind::Symbol(parts2)) => {
            let data1 = builder1.symbol_parts_data(parts1);
            let data2 = builder2.symbol_parts_data(parts2);

            // Must have same number of parts
            if data1.len() != data2.len() {
                return false;
            }

            // All parts must match (already sorted during interning)
            data1
                .iter()
                .zip(data2.iter())
                .all(|(p1, p2)| p1.as_ref() == p2.as_ref())
        }

        // Different kinds are not equal
        _ => false,
    }
}

/// Structurally compare two types and return ordering.
///
/// This is useful for sorting types from different interners or
/// implementing ordered collections of heterogeneous types.
///
/// The ordering is defined as:
/// 1. Compare type kinds (TypeVar < Scalar < Array < Map < Record < Function < Symbol)
/// 2. Within same kind, compare recursively
///
/// # Example
///
/// ```ignore
/// use melbi_types::{types_cmp, TyKind, Scalar, BoxBuilder};
///
/// let interner = BoxBuilder::new();
/// let int = TypeKind::Scalar(Scalar::Int).intern(interner);
/// let float = TypeKind::Scalar(Scalar::Float).intern(interner);
/// let arr = TypeKind::Array(int).intern(interner);
///
/// assert!(types_cmp(int, interner, arr, interner) == Ordering::Less);
/// ```
pub fn types_cmp<I1: TypeBuilder, I2: TypeBuilder>(
    ty1: I1::TypeView,
    builder1: I1,
    ty2: I2::TypeView,
    builder2: I2,
) -> Ordering {
    // Helper to get discriminant for ordering
    fn discriminant<B: TypeBuilder>(kind: &TypeKind<B>) -> u8 {
        match kind {
            TypeKind::TypeVar(_) => 0,
            TypeKind::Scalar(_) => 1,
            TypeKind::Array(_) => 2,
            TypeKind::Map(_, _) => 3,
            TypeKind::Record(_) => 4,
            TypeKind::Function { .. } => 5,
            TypeKind::Symbol(_) => 6,
        }
    }

    let kind1 = ty1.view(builder1);
    let kind2 = ty2.view(builder2);

    // First compare discriminants
    match discriminant(kind1).cmp(&discriminant(kind2)) {
        Ordering::Equal => {
            // Same kind, compare within kind
            match (kind1, kind2) {
                (TypeKind::TypeVar(id1), TypeKind::TypeVar(id2)) => id1.cmp(id2),
                (TypeKind::Scalar(s1), TypeKind::Scalar(s2)) => s1.cmp(s2),

                (TypeKind::Array(e1), TypeKind::Array(e2)) => {
                    types_cmp(e1.clone(), builder1, e2.clone(), builder2)
                }

                (TypeKind::Map(k1, v1), TypeKind::Map(k2, v2)) => {
                    match types_cmp(k1.clone(), builder1, k2.clone(), builder2) {
                        Ordering::Equal => types_cmp(v1.clone(), builder1, v2.clone(), builder2),
                        ord => ord,
                    }
                }

                (TypeKind::Record(f1), TypeKind::Record(f2)) => {
                    let data1 = builder1.field_types_data(f1);
                    let data2 = builder2.field_types_data(f2);

                    // Compare lexicographically
                    for ((n1, t1), (n2, t2)) in data1.iter().zip(data2.iter()) {
                        match n1.as_ref().cmp(n2.as_ref()) {
                            Ordering::Equal => {
                                match types_cmp(t1.clone(), builder1, t2.clone(), builder2) {
                                    Ordering::Equal => continue,
                                    ord => return ord,
                                }
                            }
                            ord => return ord,
                        }
                    }
                    data1.len().cmp(&data2.len())
                }

                (
                    TypeKind::Function {
                        params: p1,
                        ret: r1,
                    },
                    TypeKind::Function {
                        params: p2,
                        ret: r2,
                    },
                ) => {
                    let params1 = builder1.types_data(p1);
                    let params2 = builder2.types_data(p2);

                    // Compare parameters lexicographically
                    for (t1, t2) in params1.iter().zip(params2.iter()) {
                        match types_cmp(t1.clone(), builder1, t2.clone(), builder2) {
                            Ordering::Equal => continue,
                            ord => return ord,
                        }
                    }

                    // If all params equal, compare length
                    match params1.len().cmp(&params2.len()) {
                        Ordering::Equal => types_cmp(r1.clone(), builder1, r2.clone(), builder2),
                        ord => ord,
                    }
                }

                (TypeKind::Symbol(s1), TypeKind::Symbol(s2)) => {
                    let data1 = builder1.symbol_parts_data(s1);
                    let data2 = builder2.symbol_parts_data(s2);

                    // Compare lexicographically
                    for (p1, p2) in data1.iter().zip(data2.iter()) {
                        match p1.as_ref().cmp(p2.as_ref()) {
                            Ordering::Equal => continue,
                            ord => return ord,
                        }
                    }
                    data1.len().cmp(&data2.len())
                }

                _ => unreachable!("discriminants matched but kinds don't"),
            }
        }
        ord => ord,
    }
}

/// Convert a type from one interner to another.
///
/// This creates a structurally equivalent type in the target interner
/// by traversing the source type and reconstructing it.
///
/// # Example
///
/// ```ignore
/// use melbi_types::{convert_ty, TyKind, Scalar, ArenaBuilder, BoxBuilder};
/// use bumpalo::Bump;
///
/// let arena = Bump::new();
/// let arena_int = ArenaBuilder::new(&arena);
/// let box_int = BoxBuilder::new();
///
/// // Create type in arena
/// let int = TypeKind::Scalar(Scalar::Int).intern(arena_int);
/// let arr = TypeKind::Array(int).intern(arena_int);
///
/// // Convert to box interner
/// let arr_boxed = convert_ty(arr, arena_int, box_int);
///
/// // Verify they're structurally equal
/// assert!(types_equal(arr, arena_int, arr_boxed, box_int));
/// ```
pub fn convert_ty<I1: TypeBuilder, I2: TypeBuilder>(
    ty: I1::TypeView,
    from_builder: I1,
    to_builder: I2,
) -> I2::TypeView
where
    I2::TypeView: From<Ty<I2>>,
{
    match ty.view(from_builder) {
        // Base cases - reconstruct directly
        TypeKind::TypeVar(id) => TypeKind::TypeVar(*id).intern(to_builder).into(),
        TypeKind::Scalar(s) => TypeKind::Scalar(*s).intern(to_builder).into(),

        // Arrays - convert element type
        TypeKind::Array(elem) => {
            let new_elem = convert_ty(elem.clone(), from_builder, to_builder);
            TypeKind::Array(new_elem).intern(to_builder).into()
        }

        // Maps - convert key and value types
        TypeKind::Map(key, val) => {
            let new_key = convert_ty(key.clone(), from_builder, to_builder);
            let new_val = convert_ty(val.clone(), from_builder, to_builder);
            TypeKind::Map(new_key, new_val).intern(to_builder).into()
        }

        // Records - convert field types and re-intern field names
        TypeKind::Record(fields) => {
            let new_fields = from_builder
                .field_types_data(fields)
                .iter()
                .map(|(name, ty)| {
                    let new_ty = convert_ty(ty.clone(), from_builder, to_builder);
                    (name.as_ref(), new_ty)
                });
            TypeKind::Record(to_builder.intern_field_types(new_fields))
                .intern(to_builder)
                .into()
        }

        // Functions - convert parameter and return types
        TypeKind::Function { params, ret } => {
            let new_params = from_builder
                .types_data(params)
                .iter()
                .map(|param_ty| convert_ty(param_ty.clone(), from_builder, to_builder))
                .collect::<alloc::vec::Vec<_>>();
            let new_ret = convert_ty(ret.clone(), from_builder, to_builder);
            TypeKind::Function {
                params: to_builder.intern_types(new_params),
                ret: new_ret,
            }
            .intern(to_builder)
            .into()
        }

        // Symbols - re-intern parts
        TypeKind::Symbol(parts) => {
            let new_parts = from_builder
                .symbol_parts_data(parts)
                .iter()
                .map(|p| p.as_ref());
            TypeKind::Symbol(to_builder.intern_symbol_parts(new_parts))
                .intern(to_builder)
                .into()
        }
    }
}

/// TypeView trait for viewing types uniformly across different builders.
///
/// This trait provides a common interface for accessing type structure
/// regardless of which TypeBuilder was used to create the type.
pub trait TypeView<B: TypeBuilder>: Sized + Clone {
    /// View the structure of this type.
    ///
    /// Returns a reference to the TypeKind, allowing pattern matching
    /// and inspection without needing the builder as a parameter.
    fn view(&self, builder: B) -> &TypeKind<B>;

    /// Get the full type data including flags.
    ///
    /// Returns a reference to the TyData, which includes both the
    /// type structure and cached metadata.
    fn data(&self, builder: B) -> &TyData<B>;
}

/// Implementation of TypeView for `Ty<B>`.
impl<B: TypeBuilder> TypeView<B> for Ty<B> {
    fn view(&self, builder: B) -> &TypeKind<B> {
        self.kind(builder)
    }

    fn data(&self, builder: B) -> &TyData<B> {
        self.data(builder)
    }
}
