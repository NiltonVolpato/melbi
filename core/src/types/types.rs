use alloc::string::ToString;
use serde::Serialize;

use crate::{String, Vec, format};
use core::fmt::Display;

use crate::types::effects::Effects;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputationType<'a> {
    pub data: &'a Type<'a>,
    pub effects: Effects,
}

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

#[derive(Serialize, Debug, Clone, PartialEq, Eq, Hash)]
#[repr(C, u8)]
pub enum Type<'a> {
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

    // Type variables.
    TypeVar(u16) = 10,
    // TODO: More types to add later:
    //   Custom(&'a str),
    //   Union(&'a [&'a Type<'a>]),  // Must be sorted.
}

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
