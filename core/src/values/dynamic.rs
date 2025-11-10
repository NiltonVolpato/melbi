use alloc::string::ToString;

use crate::{
    Vec,
    syntax::{
        bytes_literal::{QuoteStyle as BytesQuoteStyle, escape_bytes},
        string_literal::{QuoteStyle, escape_string},
    },
    types::Type,
    types::manager::TypeManager,
    values::{
        from_raw::TypeError,
        function::Function,
        raw::{ArrayData, RawValue, RecordData, Slice},
    },
};

#[derive(Clone, Copy)]
pub struct Value<'ty_arena: 'value_arena, 'value_arena> {
    pub ty: &'ty_arena Type<'ty_arena>,
    // Keep these private - the abstraction should not leak!
    // Use constructors (int, float, str, etc.) and extractors (as_int, as_float, etc.)
    raw: RawValue,
    _phantom: core::marker::PhantomData<&'value_arena ()>,
}

impl<'ty_arena: 'value_arena, 'value_arena> Eq for Value<'ty_arena, 'value_arena> {}
impl<'ty_arena: 'value_arena, 'value_arena> PartialEq for Value<'ty_arena, 'value_arena> {
    fn eq(&self, _other: &Self) -> bool {
        unimplemented!()
    }
}

impl<'ty_arena: 'value_arena, 'value_arena> core::fmt::Debug for Value<'ty_arena, 'value_arena> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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
                let s = self.as_str().unwrap();
                escape_string(f, s, QuoteStyle::default())
            }
            Type::Bytes => {
                let bytes = self.as_bytes().unwrap();
                escape_bytes(f, bytes, BytesQuoteStyle::default())
            }
            Type::Array(_) => {
                let array = self.as_array().unwrap();
                write!(f, "[")?;
                for (i, elem) in array.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}", elem)?;
                }
                write!(f, "]")
            }
            Type::Map(_, _) => {
                // TODO: Implement proper Map display (e.g., iterate over key-value pairs)
                // For now, print a placeholder with the pointer address
                let ptr = unsafe { self.raw.boxed };
                write!(f, "<Map@{:p}>", ptr)
            }
            Type::Record(_) => {
                let record = self.as_record().unwrap();
                write!(f, "{{")?;
                for (i, (field_name, field_value)) in record.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{} = {:?}", field_name, field_value)?;
                }
                write!(f, "}}")
            }
            Type::Function { .. } => {
                // TODO: Implement proper Function display (e.g., show function signature or name)
                // For now, print a placeholder with the pointer address
                let ptr = unsafe { self.raw.function as usize };
                write!(f, "<Function@{:p}>", ptr as *const ())
            }
            Type::Symbol(_) => {
                // TODO: Implement proper Symbol display (e.g., show symbol name or value)
                // For now, print a placeholder with the pointer address
                let ptr = unsafe { self.raw.boxed };
                write!(f, "<Symbol@{:p}>", ptr)
            }
            Type::TypeVar(_) => {
                // TODO: Implement proper TypeVar display (e.g., show type variable name)
                // For now, print a placeholder with the pointer address
                let ptr = unsafe { self.raw.boxed };
                write!(f, "<TypeVar@{:p}>", ptr)
            }
        }
    }
}

impl<'ty_arena: 'value_arena, 'value_arena> core::fmt::Display for Value<'ty_arena, 'value_arena> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.ty {
            // Primitives: use native Display (no quotes, respects format flags)
            Type::Int => {
                let value = unsafe { self.raw.int_value };
                write!(f, "{}", value)
            }
            Type::Float => {
                let value = unsafe { self.raw.float_value };
                write!(f, "{}", value)
            }
            Type::Bool => {
                let value = unsafe { self.raw.bool_value };
                write!(f, "{}", value)
            }
            Type::Str => {
                let s = self.as_str().unwrap();
                write!(f, "{}", s)
            }

            // Complex types and Bytes: delegate to Debug
            _ => write!(f, "{:?}", self),
        }
    }
}

/// Format a float ensuring it always has a decimal point (Melbi requirement)
fn format_float(f: &mut core::fmt::Formatter<'_>, value: f64) -> core::fmt::Result {
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

impl<'ty_arena: 'value_arena, 'value_arena> Value<'ty_arena, 'value_arena> {
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
            _phantom: core::marker::PhantomData,
        }
    }

    /// Create a float value.
    ///
    /// Type is inferred from TypeManager. No allocation needed.
    pub fn float(type_mgr: &'ty_arena TypeManager<'ty_arena>, value: f64) -> Self {
        Self {
            ty: type_mgr.float(),
            raw: RawValue { float_value: value },
            _phantom: core::marker::PhantomData,
        }
    }

    /// Create a boolean value.
    ///
    /// Type is inferred from TypeManager. No allocation needed.
    pub fn bool(type_mgr: &'ty_arena TypeManager<'ty_arena>, value: bool) -> Self {
        Self {
            ty: type_mgr.bool(),
            raw: RawValue { bool_value: value },
            _phantom: core::marker::PhantomData,
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
            _phantom: core::marker::PhantomData,
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
            _phantom: core::marker::PhantomData,
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
            if !core::ptr::eq(elem.ty, *elem_ty) {
                return Err(TypeError::Mismatch);
            }
        }

        // Extract raw values
        let raw_values: Vec<RawValue> = elements.iter().map(|v| v.raw).collect();

        // Allocate in arena
        let data = ArrayData::new_with(arena, &raw_values);

        Ok(Self {
            ty,
            raw: data.as_raw_value(),
            _phantom: core::marker::PhantomData,
        })
    }

    /// Create a record value with runtime type validation.
    ///
    /// Type must be Record(fields). Field names and types must match.
    /// Fields must be provided in sorted order by name.
    /// Returns error if type is not Record or if fields don't match.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let rec = Value::record(
    ///     &arena,
    ///     type_mgr.record(&[("x", type_mgr.int()), ("y", type_mgr.float())]),
    ///     &[
    ///         ("x", Value::int(type_mgr, 42)),
    ///         ("y", Value::float(type_mgr, 3.14)),
    ///     ]
    /// )?;
    /// ```
    pub fn record(
        arena: &'value_arena bumpalo::Bump,
        ty: &'ty_arena Type<'ty_arena>,
        fields: &[(&'ty_arena str, Value<'ty_arena, 'value_arena>)],
    ) -> Result<Self, TypeError> {
        // Validate: ty must be Record(field_types)
        let Type::Record(field_types) = ty else {
            return Err(TypeError::Mismatch);
        };

        // Validate: field count matches
        if fields.len() != field_types.len() {
            return Err(TypeError::Mismatch);
        }

        // Validate: field names and types match (both are sorted)
        for (i, (field_name, field_value)) in fields.iter().enumerate() {
            let (expected_name, expected_ty) = field_types[i];
            if *field_name != expected_name {
                return Err(TypeError::Mismatch);
            }
            if !core::ptr::eq(field_value.ty, expected_ty) {
                return Err(TypeError::Mismatch);
            }
        }

        // Extract raw values
        let raw_values: Vec<RawValue> = fields.iter().map(|(_, v)| v.raw).collect();

        // Allocate in arena
        let data = RecordData::new_with(arena, &raw_values);

        Ok(Self {
            ty,
            raw: data.as_raw_value(),
            _phantom: core::marker::PhantomData,
        })
    }

    /// Create a function value.
    ///
    /// The function's type is obtained from `func.ty()` and must be a Function type.
    /// The function is allocated in the arena and can be called through the evaluator.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use crate::values::function::NativeFunction;
    ///
    /// let func_ty = type_mgr.function(&[type_mgr.int(), type_mgr.int()], type_mgr.int());
    /// let value = Value::function(&arena, NativeFunction::new(func_ty, add_function))?;
    /// ```
    pub fn function<T: Function<'ty_arena, 'value_arena>>(
        arena: &'value_arena bumpalo::Bump,
        func: T,
    ) -> Result<Self, TypeError> {
        // Trait objects are fat pointers (16 bytes: data pointer + vtable pointer),
        // but RawValue can only hold thin pointers (8 bytes). To work around this,
        // we use a single allocation containing both the fat pointer and the function object:
        //
        // Memory layout: [*const dyn Function (16 bytes)][T object (sizeof<T> bytes)]
        //                 ^                               ^
        //                 |                               |
        //                 storage                         storage + value_offset
        //
        // The fat pointer's data component points to the T object in the same allocation.
        // RawValue.function stores a thin pointer to the fat pointer's location.
        let (layout, value_offset) = {
            let ptr_layout =
                core::alloc::Layout::new::<*const dyn Function<'ty_arena, 'value_arena>>();
            let value_layout = core::alloc::Layout::new::<T>();
            let (layout, value_offset) = ptr_layout.extend(value_layout).unwrap();
            (layout.pad_to_align(), value_offset)
        };
        let storage = arena.alloc_layout(layout);

        // Initialize the allocation in two steps:
        // 1. Write the function object T at offset `value_offset`
        // 2. Write the fat pointer (*const dyn Function) at the beginning,
        //    with its data component pointing to the T object we just wrote
        let func_ptr = unsafe {
            let func_ptr = storage.add(value_offset).as_ptr().cast::<T>();
            core::ptr::write(func_ptr, func);

            // Create fat pointer: Rust automatically constructs vtable when casting T* to dyn Function*
            let fat_ptr: *const dyn Function<'ty_arena, 'value_arena> = func_ptr;
            core::ptr::write(
                storage.as_ptr() as *mut *const dyn Function<'ty_arena, 'value_arena>,
                fat_ptr,
            );
            &*fat_ptr
        };

        // Now we can safely get the type from the allocated function
        let ty = func_ptr.ty();

        // Validate: ty must be Function
        let Type::Function { .. } = ty else {
            return Err(TypeError::Mismatch);
        };

        Ok(Self {
            ty,
            raw: RawValue {
                // Store thin pointer to the allocated fat pointer storage
                function: storage.as_ptr() as *const (),
            },
            _phantom: core::marker::PhantomData,
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
                unsafe { Ok(core::str::from_utf8_unchecked(bytes)) }
            }
            _ => Err(TypeError::Mismatch),
        }
    }

    /// Extract bytes value dynamically.
    ///
    /// Returns error if value is not Bytes.
    pub fn as_bytes(&self) -> Result<&[u8], TypeError> {
        match self.ty {
            Type::Bytes => {
                let slice = unsafe { &*self.raw.slice };
                Ok(slice.as_slice())
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
                data: ArrayData::from_raw_value(self.raw),
                _phantom: core::marker::PhantomData,
            }),
            _ => Err(TypeError::Mismatch),
        }
    }

    /// Get dynamic record view.
    ///
    /// Returns Record wrapper that allows field access and iteration
    /// without compile-time type knowledge.
    pub fn as_record(&self) -> Result<Record<'ty_arena, 'value_arena>, TypeError> {
        match self.ty {
            Type::Record(field_types) => Ok(Record {
                field_types,
                data: RecordData::from_raw_value(self.raw),
                _phantom: core::marker::PhantomData,
            }),
            _ => Err(TypeError::Mismatch),
        }
    }

    /// Extract function trait object dynamically.
    ///
    /// Returns reference to Function trait object if value is a Function.
    /// Returns error if value is not a Function.
    ///
    /// TODO: Consider adding a checked wrapper API that provides runtime validation.
    pub fn as_function(
        &self,
    ) -> Result<&'value_arena dyn Function<'ty_arena, 'value_arena>, TypeError> {
        match self.ty {
            Type::Function { .. } => {
                // Read the fat pointer from the allocated storage
                let storage_ptr = unsafe {
                    self.raw.function as *const *const dyn Function<'ty_arena, 'value_arena>
                };
                let func_ptr = unsafe { *storage_ptr };
                Ok(unsafe { &*func_ptr })
            }
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
    data: ArrayData<'value_arena>,
    _phantom: core::marker::PhantomData<&'value_arena ()>,
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
            _phantom: core::marker::PhantomData,
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
            _phantom: core::marker::PhantomData,
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
    _phantom: core::marker::PhantomData<&'a Array<'ty_arena, 'value_arena>>,
}

impl<'a, 'ty_arena: 'value_arena, 'value_arena> Iterator
    for ArrayIter<'a, 'ty_arena, 'value_arena>
{
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
            _phantom: core::marker::PhantomData,
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = unsafe { self.end.offset_from(self.current) as usize };
        (remaining, Some(remaining))
    }
}

impl<'a, 'ty_arena: 'value_arena, 'value_arena> ExactSizeIterator
    for ArrayIter<'a, 'ty_arena, 'value_arena>
{
    fn len(&self) -> usize {
        unsafe { self.end.offset_from(self.current) as usize }
    }
}

// ============================================================================
// Record - Runtime record access without compile-time type knowledge
// ============================================================================

/// Dynamic view of a record that doesn't require compile-time field types.
///
/// Allows field access by name and iteration over fields.
pub struct Record<'ty_arena, 'value_arena> {
    field_types: &'ty_arena [(&'ty_arena str, &'ty_arena Type<'ty_arena>)],
    data: RecordData<'value_arena>,
    _phantom: core::marker::PhantomData<&'value_arena ()>,
}

impl<'ty_arena, 'value_arena> Record<'ty_arena, 'value_arena> {
    /// Get the number of fields in the record.
    pub fn len(&self) -> usize {
        self.field_types.len()
    }

    /// Check if the record is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get field by name, returning it as a Value.
    ///
    /// Returns None if field name is not found.
    /// Uses binary search since fields are sorted by name.
    pub fn get(&self, field_name: &str) -> Option<Value<'ty_arena, 'value_arena>> {
        // Binary search for field name
        let index = self
            .field_types
            .binary_search_by_key(&field_name, |(name, _)| *name)
            .ok()?;

        let (_, field_ty) = self.field_types[index];
        let raw = unsafe { self.data.get(index) };

        Some(Value {
            ty: field_ty,
            raw,
            _phantom: core::marker::PhantomData,
        })
    }

    /// Iterate over fields as (name, Value) pairs.
    pub fn iter(&self) -> RecordIter<'_, 'ty_arena, 'value_arena> {
        RecordIter {
            field_types: self.field_types,
            data: self.data,
            index: 0,
            _phantom: core::marker::PhantomData,
        }
    }
}

/// Iterator over Record fields.
///
/// Yields (field_name, field_value) pairs in sorted order by field name.
pub struct RecordIter<'a, 'ty_arena, 'value_arena> {
    field_types: &'ty_arena [(&'ty_arena str, &'ty_arena Type<'ty_arena>)],
    data: RecordData<'value_arena>,
    index: usize,
    _phantom: core::marker::PhantomData<&'a Record<'ty_arena, 'value_arena>>,
}

impl<'a, 'ty_arena: 'value_arena, 'value_arena> Iterator
    for RecordIter<'a, 'ty_arena, 'value_arena>
{
    type Item = (&'ty_arena str, Value<'ty_arena, 'value_arena>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.field_types.len() {
            return None;
        }

        let (field_name, field_ty) = self.field_types[self.index];
        let raw = unsafe { self.data.get(self.index) };
        self.index += 1;

        Some((
            field_name,
            Value {
                ty: field_ty,
                raw,
                _phantom: core::marker::PhantomData,
            },
        ))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.field_types.len() - self.index;
        (remaining, Some(remaining))
    }
}

impl<'a, 'ty_arena: 'value_arena, 'value_arena> ExactSizeIterator
    for RecordIter<'a, 'ty_arena, 'value_arena>
{
    fn len(&self) -> usize {
        self.field_types.len() - self.index
    }
}
