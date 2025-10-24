use crate::{
    Type,
    types::manager::TypeManager,
    values::{
        from_raw::{FromRawValue, TypeError},
        raw::{RawValue, Slice},
    },
};

#[derive(Clone)]
pub struct Value<'ty_arena, 'value_arena> {
    pub ty: &'ty_arena Type<'ty_arena>,
    pub raw: RawValue,
    pub _phantom: std::marker::PhantomData<&'value_arena ()>,
}

impl<'ty_arena, 'value_arena> Eq for Value<'ty_arena, 'value_arena> {}
impl<'ty_arena, 'value_arena> PartialEq for Value<'ty_arena, 'value_arena> {
    fn eq(&self, _other: &Self) -> bool {
        unimplemented!()
    }
}

impl<'ty_arena, 'value_arena> std::fmt::Debug for Value<'ty_arena, 'value_arena> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Value<{:?}>", self.ty) // XXX
    }
}

impl<'ty_arena, 'value_arena> std::fmt::Display for Value<'ty_arena, 'value_arena> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.ty {
            Type::Int => {
                let value = unsafe { self.raw.int_value };
                write!(f, "{}", value)
            }
            Type::Float => {
                let value = unsafe { self.raw.float_value };
                format_float(f, value)
            }
            Type::Bool => {
                let value = unsafe { self.raw.bool_value };
                write!(f, "{}", value)
            }
            Type::Str => {
                // TODO: Consider using single quotes if the string contains double quotes.
                let slice = unsafe { &*self.raw.slice };
                let bytes = slice.as_slice();
                let s = std::str::from_utf8(bytes).expect("Invalid UTF-8 in string value");
                write!(f, "\"{}\"", escape_string(s))
            }
            Type::Bytes => {
                // TODO: Consider using single quotes if the bytes literal contains double quotes.
                let slice = unsafe { &*self.raw.slice };
                let bytes = slice.as_slice();
                write!(f, "b\"")?;
                for &byte in bytes {
                    write!(f, "\\x{:02x}", byte)?;
                }
                write!(f, "\"")
            }
            Type::Array(elem_ty) => {
                let array_data = unsafe { &*self.raw.array };
                write!(f, "[")?;
                for i in 0..array_data.length() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    let elem_raw = unsafe { array_data.get(i) };
                    let elem_value = Value {
                        ty: elem_ty,
                        raw: elem_raw,
                        _phantom: std::marker::PhantomData,
                    };
                    write!(f, "{}", elem_value)?;
                }
                write!(f, "]")
            }
            Type::Map(_, _) => {
                todo!("Map display not yet implemented")
            }
            Type::Record(_) => {
                todo!("Record display not yet implemented")
            }
            Type::Function { .. } => {
                todo!("Function display not yet implemented")
            }
            Type::Symbol(_) => {
                todo!("Symbol display not yet implemented")
            }
            Type::TypeVar(_) => {
                todo!("TypeVar display not yet implemented")
            }
        }
    }
}

/// Format a float ensuring it always has a decimal point (Melbi requirement)
fn format_float(f: &mut std::fmt::Formatter<'_>, value: f64) -> std::fmt::Result {
    if value.is_nan() {
        write!(f, "nan")
    } else if value.is_infinite() {
        if value.is_sign_positive() {
            write!(f, "inf")
        } else {
            write!(f, "-inf")
        }
    } else {
        let s = value.to_string();
        if s.contains('.') || s.contains('e') || s.contains('E') {
            write!(f, "{}", s)
        } else {
            write!(f, "{}.", s)
        }
    }
}

// TODO: Create a single escaping/unescaping utility used throughout Melbi
/// Escape special characters in strings for Melbi literals
fn escape_string(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            '"' => vec!['\\', '"'],
            '\\' => vec!['\\', '\\'],
            '\n' => vec!['\\', 'n'],
            '\r' => vec!['\\', 'r'],
            '\t' => vec!['\\', 't'],
            c if c.is_control() => format!("\\u{{{:04x}}}", c as u32).chars().collect(),
            c => vec![c],
        })
        .collect()
}

impl<'ty_arena, 'value_arena> Value<'ty_arena, 'value_arena> {
    pub fn from_raw(
        arena: &'value_arena bumpalo::Bump,
        ty: &'ty_arena Type<'ty_arena>,
        raw: RawValue,
    ) -> Self {
        let _ = arena;
        Self {
            ty,
            raw,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn int(
        arena: &'value_arena bumpalo::Bump,
        ty: &'ty_arena Type<'ty_arena>,
        int_value: i64,
    ) -> Self {
        let _ = arena;
        Self {
            ty,
            raw: RawValue { int_value },
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn float(
        arena: &'value_arena bumpalo::Bump,
        ty: &'ty_arena Type<'ty_arena>,
        float_value: f64,
    ) -> Self {
        let _ = arena;
        Self {
            ty,
            raw: RawValue { float_value },
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn bool(
        arena: &'value_arena bumpalo::Bump,
        ty: &'ty_arena Type<'ty_arena>,
        bool_value: bool,
    ) -> Self {
        let _ = arena;
        Self {
            ty,
            raw: RawValue { bool_value },
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn str(
        arena: &'value_arena bumpalo::Bump,
        ty: &'ty_arena Type<'ty_arena>,
        str: &str,
    ) -> Self {
        // TODO: Create a factory function that doesn't copy the data.
        let data = arena.alloc_slice_copy(str.as_bytes());
        Self {
            ty,
            raw: arena.alloc(Slice::new(arena, data)).as_raw_value(),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn bytes(
        arena: &'value_arena bumpalo::Bump,
        ty: &'ty_arena Type<'ty_arena>,
        bytes: &[u8],
    ) -> Self {
        // TODO: Create a factory function that doesn't copy the data.
        let data = arena.alloc_slice_copy(bytes);
        Self {
            ty,
            raw: arena.alloc(Slice::new(arena, data)).as_raw_value(),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn get<T: FromRawValue<'ty_arena>>(
        &self,
        type_mgr: &'ty_arena TypeManager<'ty_arena>,
    ) -> Result<T, TypeError> {
        T::from_raw(type_mgr, self.ty, self.raw)
    }
}
