use alloc::string::ToString;
use serde::Serialize;

use crate::{String, Vec, format};
use core::{
    fmt::Display,
    hash::{Hash, Hasher},
};

use crate::types::effects::Effects;

#[derive(Debug, Clone)]
pub struct ComputationType<'a> {
    pub data: &'a Type<'a>,
    pub effects: Effects,
}

impl<'a> PartialEq for ComputationType<'a> {
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self.data, other.data) && self.effects == other.effects
    }
}

impl<'a> Eq for ComputationType<'a> {}

impl<'a> ComputationType<'a> {
    pub fn new(data: &'a Type<'a>) -> Self {
        ComputationType {
            data,
            effects: Effects::TOTAL,
        }
    }

    pub fn with_effects(&self, effects: Effects) -> Self {
        ComputationType {
            data: self.data,
            effects,
        }
    }

    // Check if type is total (no effects)
    pub fn is_total(&self) -> bool {
        self.effects == Effects::TOTAL
    }

    // Check specific effects
    pub fn can_error(&self) -> bool {
        self.effects.can_error
    }

    pub fn is_impure(&self) -> bool {
        self.effects.is_impure
    }
}

#[derive(Serialize, Debug, Clone, Hash)]
#[repr(C, u8)]
pub enum Type<'a> {
    // Type variables.
    TypeVar(u16) = 10, // TODO: Renumber to zero.

    // Primitives.
    Int = 0,
    Float = 1,
    Bool = 2,
    Str = 3,
    Bytes = 4,

    // Collections.
    Array(&'a Type<'a>) = 5,
    Map(&'a Type<'a>, &'a Type<'a>) = 6,

    // Structural records.
    Record(&'a [(&'a str, &'a Type<'a>)]) = 7, // Must be sorted by field name.

    // Functions.
    Function {
        params: &'a [&'a Type<'a>],
        ret: &'a Type<'a>,
    } = 8,

    // Symbols.
    Symbol(&'a [&'a str]) = 9, // Must be sorted.

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

impl Display for Type<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Type::Int => write!(f, "Int"),
            Type::Float => write!(f, "Float"),
            Type::Bool => write!(f, "Bool"),
            Type::Str => write!(f, "Str"),
            Type::Bytes => write!(f, "Bytes"),
            Type::Array(elem_ty) => write!(f, "Array[{}]", elem_ty),
            Type::Map(key_ty, val_ty) => write!(f, "Map[{}, {}]", key_ty, val_ty),
            Type::Record(fields) => {
                let field_strs: Vec<String> = fields
                    .iter()
                    .map(|(name, ty)| format!("{}: {}", name, ty))
                    .collect();
                write!(f, "Record[{}]", field_strs.join(", "))
            }
            Type::Function { params, ret } => {
                let param_strs: Vec<String> = params.iter().map(|ty| format!("{}", ty)).collect();
                write!(f, "({}) => {}", param_strs.join(", "), ret)
            }
            Type::Symbol(parts) => {
                let part_strs: Vec<String> = parts.iter().map(|p| p.to_string()).collect();
                write!(f, "Symbol[{}]", part_strs.join("|"))
            }
            Type::TypeVar(id) => write!(f, "_{}", id),
        }
    }
}
