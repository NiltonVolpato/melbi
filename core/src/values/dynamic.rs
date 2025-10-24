use crate::{
    Type,
    types::manager::TypeManager,
    values::{
        from_raw::TypeError,
        raw::{ArrayData, RawValue, Slice},
    },
};

#[derive(Clone)]
pub struct Value<'ty_arena, 'value_arena> {
    pub ty: &'ty_arena Type<'ty_arena>,
    raw: RawValue,
    _phantom: std::marker::PhantomData<&'value_arena ()>,
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
    // ============================================================================
    // Safe Construction API - Primitives (simple values, no allocation)
    // ============================================================================
    //
    // Simple values take TypeManager (not Type) and don't return Result.
    // They can't fail because the value always matches the type.

    /// Create an integer value.
    ///
    /// Type is inferred from TypeManager. No allocation needed.
    pub fn int(type_mgr: &'ty_arena TypeManager<'ty_arena>, value: i64) -> Self {
        Self {
            ty: type_mgr.int(),
            raw: RawValue { int_value: value },
            _phantom: std::marker::PhantomData,
        }
    }

    /// Create a float value.
    ///
    /// Type is inferred from TypeManager. No allocation needed.
    pub fn float(type_mgr: &'ty_arena TypeManager<'ty_arena>, value: f64) -> Self {
        Self {
            ty: type_mgr.float(),
            raw: RawValue { float_value: value },
            _phantom: std::marker::PhantomData,
        }
    }

    /// Create a boolean value.
    ///
    /// Type is inferred from TypeManager. No allocation needed.
    pub fn bool(type_mgr: &'ty_arena TypeManager<'ty_arena>, value: bool) -> Self {
        Self {
            ty: type_mgr.bool(),
            raw: RawValue { bool_value: value },
            _phantom: std::marker::PhantomData,
        }
    }

    // ============================================================================
    // Safe Construction API - Compound Values (require allocation and validation)
    // ============================================================================
    //
    // Compound values require explicit type and arena, and return Result for validation.

    /// Create a string value.
    ///
    /// Requires arena for allocation and explicit type.
    pub fn str(
        arena: &'value_arena bumpalo::Bump,
        ty: &'ty_arena Type<'ty_arena>,
        value: &str,
    ) -> Self {
        let data = arena.alloc_slice_copy(value.as_bytes());
        Self {
            ty,
            raw: arena.alloc(Slice::new(arena, data)).as_raw_value(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Create a bytes value.
    ///
    /// Requires arena for allocation and explicit type.
    pub fn bytes(
        arena: &'value_arena bumpalo::Bump,
        ty: &'ty_arena Type<'ty_arena>,
        value: &[u8],
    ) -> Self {
        let data = arena.alloc_slice_copy(value);
        Self {
            ty,
            raw: arena.alloc(Slice::new(arena, data)).as_raw_value(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Create an array value with runtime type validation.
    ///
    /// Type must be Array(elem_ty). All elements must match elem_ty.
    /// Returns error if type is not Array or if any element has wrong type.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let arr = Value::array(
    ///     &arena,
    ///     type_mgr.array(type_mgr.int()),
    ///     &[
    ///         Value::int(type_mgr, 1),
    ///         Value::int(type_mgr, 2),
    ///     ]
    /// )?;
    /// ```
    pub fn array(
        arena: &'value_arena bumpalo::Bump,
        ty: &'ty_arena Type<'ty_arena>,
        elements: &[Value<'ty_arena, 'value_arena>],
    ) -> Result<Self, TypeError> {
        // Validate: ty must be Array(elem_ty)
        let Type::Array(elem_ty) = ty else {
            return Err(TypeError::Mismatch);
        };

        // Validate: all elements match elem_ty
        for elem in elements.iter() {
            if !std::ptr::eq(elem.ty, *elem_ty) {
                return Err(TypeError::Mismatch);
            }
        }

        // Extract raw values
        let raw_values: Vec<RawValue> = elements.iter().map(|v| v.raw).collect();

        // Allocate in arena
        let data = ArrayData::new_with(arena, &raw_values);

        Ok(Self {
            ty,
            raw: RawValue { array: data },
            _phantom: std::marker::PhantomData,
        })
    }

    // ============================================================================
    // Dynamic Extraction API
    // ============================================================================
    //
    // These methods extract values without requiring compile-time type knowledge.

    /// Extract integer value dynamically.
    ///
    /// Returns error if value is not an Int.
    pub fn as_int(&self) -> Result<i64, TypeError> {
        match self.ty {
            Type::Int => Ok(unsafe { self.raw.int_value }),
            _ => Err(TypeError::Mismatch),
        }
    }

    /// Extract float value dynamically.
    ///
    /// Returns error if value is not a Float.
    pub fn as_float(&self) -> Result<f64, TypeError> {
        match self.ty {
            Type::Float => Ok(unsafe { self.raw.float_value }),
            _ => Err(TypeError::Mismatch),
        }
    }

    /// Extract boolean value dynamically.
    ///
    /// Returns error if value is not a Bool.
    pub fn as_bool(&self) -> Result<bool, TypeError> {
        match self.ty {
            Type::Bool => Ok(unsafe { self.raw.bool_value }),
            _ => Err(TypeError::Mismatch),
        }
    }

    /// Extract string value dynamically.
    ///
    /// Returns error if value is not a Str.
    pub fn as_str(&self) -> Result<&str, TypeError> {
        match self.ty {
            Type::Str => {
                let slice = unsafe { &*self.raw.slice };
                let bytes = slice.as_slice();
                std::str::from_utf8(bytes).map_err(|_| TypeError::Mismatch)
            }
            _ => Err(TypeError::Mismatch),
        }
    }

    /// Get dynamic array view.
    ///
    /// Returns Array wrapper that allows iteration and indexing
    /// without compile-time type knowledge.
    pub fn as_array(&self) -> Result<Array<'ty_arena, 'value_arena>, TypeError> {
        match self.ty {
            Type::Array(elem_ty) => Ok(Array {
                elem_ty,
                data: unsafe { &*self.raw.array },
                _phantom: std::marker::PhantomData,
            }),
            _ => Err(TypeError::Mismatch),
        }
    }
}

// ============================================================================
// Array - Runtime array access without compile-time type knowledge
// ============================================================================

/// Dynamic view of an array that doesn't require compile-time element type.
///
/// Allows iteration and indexing, returning elements as `Value`.
pub struct Array<'ty_arena, 'value_arena> {
    elem_ty: &'ty_arena Type<'ty_arena>,
    data: &'value_arena ArrayData,
    _phantom: std::marker::PhantomData<&'value_arena ()>,
}

impl<'ty_arena, 'value_arena> Array<'ty_arena, 'value_arena> {
    /// Get the number of elements in the array.
    pub fn len(&self) -> usize {
        self.data.length()
    }

    /// Check if the array is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get element at index, returning it as a Value.
    ///
    /// Returns None if index is out of bounds.
    pub fn get(&self, index: usize) -> Option<Value<'ty_arena, 'value_arena>> {
        if index >= self.len() {
            return None;
        }

        let raw = unsafe { self.data.get(index) };
        Some(Value {
            ty: self.elem_ty,
            raw,
            _phantom: std::marker::PhantomData,
        })
    }

    /// Iterate over elements as Values.
    pub fn iter(&self) -> ArrayIter<'_, 'ty_arena, 'value_arena> {
        let start = self.data.as_ptr();
        let end = unsafe { start.add(self.len()) };
        ArrayIter {
            elem_ty: self.elem_ty,
            current: start,
            end,
            _phantom: std::marker::PhantomData,
        }
    }
}

/// Iterator over Array elements.
///
/// Uses start/end pointer strategy like C++ iterators for efficient iteration
/// without repeated bounds checks.
pub struct ArrayIter<'a, 'ty_arena, 'value_arena> {
    elem_ty: &'ty_arena Type<'ty_arena>,
    current: *const RawValue,
    end: *const RawValue,
    _phantom: std::marker::PhantomData<&'a Array<'ty_arena, 'value_arena>>,
}

impl<'a, 'ty_arena, 'value_arena> Iterator for ArrayIter<'a, 'ty_arena, 'value_arena> {
    type Item = Value<'ty_arena, 'value_arena>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.end {
            return None;
        }

        let raw = unsafe { *self.current };
        self.current = unsafe { self.current.add(1) };

        Some(Value {
            ty: self.elem_ty,
            raw,
            _phantom: std::marker::PhantomData,
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = unsafe { self.end.offset_from(self.current) as usize };
        (remaining, Some(remaining))
    }
}

impl<'a, 'ty_arena, 'value_arena> ExactSizeIterator for ArrayIter<'a, 'ty_arena, 'value_arena> {
    fn len(&self) -> usize {
        unsafe { self.end.offset_from(self.current) as usize }
    }
}
