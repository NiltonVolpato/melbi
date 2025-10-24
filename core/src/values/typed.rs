//! Tier 1: Statically-typed, compile-time safe value API
//!
//! This module provides zero-overhead, compile-time type-safe wrappers around
//! the untyped RawValue representation. Types are guaranteed at compile time,
//! eliminating the need for runtime type checking or TypeManager.

use std::marker::PhantomData;

use bumpalo::Bump;
use static_assertions::assert_eq_size;

use crate::{
    Type,
    types::manager::TypeManager,
    values::raw::{ArrayData, RawValue, Slice},
};

pub trait RawConvertible: Sized {
    fn to_raw_value(self) -> RawValue;
    unsafe fn from_raw_value(raw: RawValue) -> Self;
}

pub trait Bridge<'a>: RawConvertible {
    type Raw: Sized;
    fn type_from(type_mgr: &'a TypeManager<'a>) -> &'a Type<'a>;
}

// Blanket implementation
impl<'a, T: Bridge<'a>> RawConvertible for T {
    fn to_raw_value(self) -> RawValue {
        const {
            assert!(std::mem::size_of::<Self>() == std::mem::size_of::<RawValue>());
        }
        unsafe { std::mem::transmute_copy(&self) }
    }

    unsafe fn from_raw_value(raw: RawValue) -> Self {
        const {
            assert!(std::mem::size_of::<Self>() == std::mem::size_of::<RawValue>());
        }
        unsafe { std::mem::transmute_copy(&raw) }
    }
}

impl<'a> Bridge<'a> for i64 {
    type Raw = i64;
    fn type_from(type_mgr: &'a TypeManager<'a>) -> &'a Type<'a> {
        type_mgr.int()
    }
}

impl<'a> Bridge<'a> for f64 {
    type Raw = f64;
    fn type_from(type_mgr: &'a TypeManager<'a>) -> &'a Type<'a> {
        type_mgr.float()
    }
}

impl<'a> Bridge<'a> for bool {
    type Raw = bool;
    fn type_from(type_mgr: &'a TypeManager<'a>) -> &'a Type<'a> {
        type_mgr.bool()
    }
}

impl<'a> Bridge<'a> for &str {
    type Raw = *const Slice;
    fn type_from(type_mgr: &'a TypeManager<'a>) -> &'a Type<'a> {
        type_mgr.str()
    }
}

impl<'a> Bridge<'a> for &[u8] {
    type Raw = *const Slice;
    fn type_from(type_mgr: &'a TypeManager<'a>) -> &'a Type<'a> {
        type_mgr.bytes()
    }
}

impl<'a, E: Bridge<'a>> Bridge<'a> for Array<'a, E> {
    type Raw = *const ArrayData;
    fn type_from(type_mgr: &'a TypeManager<'a>) -> &'a Type<'a> {
        let elem_ty = E::type_from(type_mgr);
        type_mgr.array(elem_ty)
    }
}

/// Trait for Rust types that can be stored in Melbi values.
///
/// This trait provides zero-cost conversion between Rust types and the
/// untyped RawValue representation used internally.
// pub trait MelbiType<'a>: Sized {
//     /// Convert a Rust value to its RawValue representation
//     fn to_raw(arena: &'a Bump, value: Self) -> RawValue;

//     /// Extract a Rust value from its RawValue representation
//     ///
//     /// # Safety
//     ///
//     /// Caller must ensure the RawValue actually contains a value of this type.
//     /// Accessing the wrong union field is undefined behavior.
//     unsafe fn from_raw_unchecked(raw: RawValue) -> Self;
// }

// Primitive type implementations

// impl MelbiType<'_> for i64 {
//     fn to_raw(_arena: &Bump, value: Self) -> RawValue {
//         RawValue { int_value: value }
//     }

//     unsafe fn from_raw_unchecked(raw: RawValue) -> Self {
//         unsafe { raw.int_value }
//     }
// }

// impl MelbiType<'_> for f64 {
//     fn to_raw(_arena: &Bump, value: Self) -> RawValue {
//         RawValue { float_value: value }
//     }

//     unsafe fn from_raw_unchecked(raw: RawValue) -> Self {
//         unsafe { raw.float_value }
//     }
// }

// impl MelbiType<'_> for bool {
//     fn to_raw(_arena: &Bump, value: Self) -> RawValue {
//         RawValue { bool_value: value }
//     }

//     unsafe fn from_raw_unchecked(raw: RawValue) -> Self {
//         unsafe { raw.bool_value }
//     }
// }

// impl<'a, T: MelbiType<'a>> MelbiType<'a> for Array<'a, T> {
//     fn to_raw(_arena: &'a Bump, value: Self) -> RawValue {
//         RawValue { array: value.ptr }
//     }

//     unsafe fn from_raw_unchecked(raw: RawValue) -> Self {
//         unsafe { Self::from_raw_value(raw) }
//     }
// }

/// Statically-typed array with compile-time element type checking.
///
/// Unlike the dynamically-typed Array in `from_raw.rs`, this array knows
/// its element type at compile time, providing zero-overhead access without
/// runtime type checking.
///
/// # Example
///
/// ```ignore
/// let arena = Bump::new();
/// let arr = Array::<i64>::new(&arena, &[1, 2, 3]);
/// assert_eq!(arr.get(0), Some(1));
/// assert_eq!(arr.len(), 3);
/// ```
#[repr(C)]
pub struct Array<'a, T: Bridge<'a>> {
    ptr: *const ArrayData,
    _phantom: PhantomData<(&'a (), T)>,
}
assert_eq_size!(Array<'_, i64>, *const ArrayData);

impl<'a, T: Bridge<'a>> Array<'a, T> {
    /// Create a new array from a slice of values.
    ///
    /// This is the primary user-facing constructor for creating typed arrays.
    /// Values are converted to RawValue representation and stored in the arena.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let arr = Array::<i64>::new(&arena, &[1, 2, 3, 4, 5]);
    /// ```
    pub fn new(arena: &'a Bump, values: &[T]) -> Self
    where
        T: Copy,
    {
        // Convert Rust values to RawValue representation
        let raw_values: Vec<RawValue> = values.iter().map(|&v| v.to_raw_value()).collect();

        // Allocate in arena
        let data = ArrayData::new_with(arena, &raw_values);

        Self {
            ptr: data as *const ArrayData,
            _phantom: PhantomData,
        }
    }

    /// Get the element at the specified index.
    ///
    /// Returns `None` if the index is out of bounds.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let arr = Array::<i64>::new(&arena, &[10, 20, 30]);
    /// assert_eq!(arr.get(1), Some(20));
    /// assert_eq!(arr.get(5), None);
    /// ```
    pub fn get(&self, index: usize) -> Option<T> {
        unsafe {
            let data = &*self.ptr;
            if index >= data.length() {
                return None;
            }
            let raw = data.get(index);
            Some(T::from_raw_value(raw))
        }
    }

    /// Get the element at the specified index without bounds checking.
    ///
    /// # Safety
    ///
    /// Caller must ensure the index is within bounds.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let arr = Array::<i64>::new(&arena, &[10, 20, 30]);
    /// unsafe {
    ///     assert_eq!(arr.get_unchecked(1), 20);
    /// }
    /// ```
    pub unsafe fn get_unchecked(&self, index: usize) -> T {
        unsafe {
            let data = &*self.ptr;
            debug_assert!(index < data.length(), "Index out of bounds");
            let raw = data.get(index);
            T::from_raw_value(raw)
        }
    }

    /// Returns the number of elements in the array.
    pub fn len(&self) -> usize {
        unsafe { (*self.ptr).length() }
    }

    /// Returns `true` if the array is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get a pointer to the underlying ArrayData for FFI/VM use.
    ///
    /// This is useful for bridging to Tier 2 (DynamicValue) or Tier 3 (RawValue).
    pub fn as_raw_value(&self) -> RawValue {
        RawValue { array: self.ptr }
    }

    /// Create an array from a raw value (unsafe, for FFI/VM use).
    ///
    /// # Safety
    ///
    /// Caller must ensure:
    /// - RawValue holds a variant pointing to ArrayData
    /// - The ArrayData pointed to by the RawValue is valid
    /// - The ArrayData contains values of type T
    /// - The ArrayData lives for at least 'a
    pub unsafe fn from_raw_value(raw: RawValue) -> Self {
        Self {
            ptr: unsafe { raw.array },
            _phantom: PhantomData,
        }
    }
}

impl<'a, T: Bridge<'a>> Clone for Array<'a, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T: Bridge<'a>> Copy for Array<'a, T> {}
