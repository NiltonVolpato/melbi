use serde::Serialize;

use core::{
    fmt::Display,
    hash::{Hash, Hasher},
};

#[derive(Serialize, Debug, Clone, Hash)]
#[repr(C, u8)]
pub enum Type<'a> {
    // Type variables.
    TypeVar(u16) = 0,

    // Primitives.
    Int = 1,
    Float = 2,
    Bool = 3,
    Str = 4,
    Bytes = 5,

    // Collections.
    Array(&'a Type<'a>) = 6,
    Map(&'a Type<'a>, &'a Type<'a>) = 7,

    // Structural records.
    Record(&'a [(&'a str, &'a Type<'a>)]) = 8, // Must be sorted by field name.

    // Functions.
    Function {
        params: &'a [&'a Type<'a>],
        ret: &'a Type<'a>,
    } = 9,

    // Symbols.
    Symbol(&'a [&'a str]) = 10, // Must be sorted.

                                // TODO: More types to add later:
                                //   Custom(&'a str),
                                //   Union(&'a [&'a Type<'a>]),  // Must be sorted.
}

pub(super) struct CompareTypeArgs<'a>(pub(super) Type<'a>);

impl Hash for CompareTypeArgs<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        core::mem::discriminant(&self.0).hash(state);
        match &self.0 {
            // Primitives - just discriminant is enough (no additional data)
            Type::Int | Type::Float | Type::Bool | Type::Str | Type::Bytes => {}

            // TypeVar - hash the ID
            Type::TypeVar(id) => {
                id.hash(state);
            }

            Type::Array(elem) => {
                (*elem as *const Type<'_>).hash(state);
            }
            Type::Map(key, val) => {
                (*key as *const Type<'_>).hash(state);
                (*val as *const Type<'_>).hash(state);
            }
            Type::Function { params, ret } => {
                for param in *params {
                    (*param as *const Type<'_>).hash(state);
                }
                (*ret as *const Type<'_>).hash(state);
            }
            Type::Symbol(parts) => {
                for part in *parts {
                    (*part as *const str).hash(state);
                }
            }
            Type::Record(fields) => {
                for (name, ty) in *fields {
                    (*name as *const str).hash(state);
                    (*ty as *const Type<'_>).hash(state);
                }
            }
        }
    }
}

impl PartialEq for CompareTypeArgs<'_> {
    fn eq(&self, other: &Self) -> bool {
        core::mem::discriminant(&self.0) == core::mem::discriminant(&other.0)
            && match (&self.0, &other.0) {
                // Primitives - discriminant comparison is sufficient
                (Type::Int, Type::Int)
                | (Type::Float, Type::Float)
                | (Type::Bool, Type::Bool)
                | (Type::Str, Type::Str)
                | (Type::Bytes, Type::Bytes) => true,

                // TypeVar - compare IDs
                (Type::TypeVar(id1), Type::TypeVar(id2)) => id1 == id2,

                (Type::Array(elem1), Type::Array(elem2)) => core::ptr::eq(*elem1, *elem2),
                (Type::Map(key1, val1), Type::Map(key2, val2)) => {
                    core::ptr::eq(*key1, *key2) && core::ptr::eq(*val1, *val2)
                }
                (
                    Type::Function {
                        params: params1,
                        ret: ret1,
                    },
                    Type::Function {
                        params: params2,
                        ret: ret2,
                    },
                ) => {
                    params1.len() == params2.len()
                        && params1
                            .iter()
                            .zip(*params2)
                            .all(|(&a, &b)| core::ptr::eq(a, b))
                        && core::ptr::eq(*ret1, *ret2)
                }
                (Type::Symbol(parts1), Type::Symbol(parts2)) => {
                    parts1.len() == parts2.len()
                        && parts1
                            .iter()
                            .zip(*parts2)
                            .all(|(a, b)| core::ptr::eq(*a as *const str, *b as *const str))
                }
                (Type::Record(fields1), Type::Record(fields2)) => {
                    fields1.len() == fields2.len()
                        && fields1
                            .iter()
                            .zip(*fields2)
                            .all(|((name1, ty1), (name2, ty2))| {
                                core::ptr::eq(*name1 as *const str, *name2 as *const str)
                                    && core::ptr::eq(*ty1, *ty2)
                            })
                }
                _ => false,
            }
    }
}

impl Eq for CompareTypeArgs<'_> {}

// Pointer-based equality for &Type (used by TypeView trait)
// Two type references are equal if they point to the same arena-allocated type
// This enables fast O(1) equality checks via interning
impl<'a> PartialEq for &'a Type<'a> {
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(*self as *const Type<'a>, *other as *const Type<'a>)
    }
}

impl<'a> Eq for &'a Type<'a> {}

impl Display for Type<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Delegate to the generic display_type function
        write!(f, "{}", crate::types::type_traits::display_type(self))
    }
}
