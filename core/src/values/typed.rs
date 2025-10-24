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
    values::raw::{ArrayData, RawValue},
};

pub trait RawConvertible: Sized {
    fn to_raw_value(self) -> RawValue;
    unsafe fn from_raw_value(raw: RawValue) -> Self;
}

pub trait Bridge<'a>: RawConvertible {
    type Raw: Sized;
    fn type_from(type_mgr: &'a TypeManager<'a>) -> &'a Type<'a>;
}

impl RawConvertible for i64 {
    fn to_raw_value(self) -> RawValue {
        RawValue { int_value: self }
    }

    unsafe fn from_raw_value(raw: RawValue) -> Self {
        unsafe { raw.int_value }
    }
}

impl<'a> Bridge<'a> for i64 {
    type Raw = i64;
    fn type_from(type_mgr: &'a TypeManager<'a>) -> &'a Type<'a> {
        type_mgr.int()
    }
}

impl RawConvertible for f64 {
    fn to_raw_value(self) -> RawValue {
        RawValue { float_value: self }
    }

    unsafe fn from_raw_value(raw: RawValue) -> Self {
        unsafe { raw.float_value }
    }
}

impl<'a> Bridge<'a> for f64 {
    type Raw = f64;
    fn type_from(type_mgr: &'a TypeManager<'a>) -> &'a Type<'a> {
        type_mgr.float()
    }
}

impl RawConvertible for bool {
    fn to_raw_value(self) -> RawValue {
        RawValue { bool_value: self }
    }

    unsafe fn from_raw_value(raw: RawValue) -> Self {
        unsafe { raw.bool_value }
    }
}

impl<'a> Bridge<'a> for bool {
    type Raw = bool;
    fn type_from(type_mgr: &'a TypeManager<'a>) -> &'a Type<'a> {
        type_mgr.bool()
    }
}

// Note: &str and &[u8] don't implement Bridge because they're not statically-typed
// Melbi values. They're used for Value construction, not for static typing.

impl<'a, E: Bridge<'a>> Bridge<'a> for Array<'a, E> {
    type Raw = *const ArrayData;
    fn type_from(type_mgr: &'a TypeManager<'a>) -> &'a Type<'a> {
        let elem_ty = E::type_from(type_mgr);
        type_mgr.array(elem_ty)
    }
}

#[repr(C)]
pub struct Array<'a, T: Bridge<'a>> {
    ptr: *const ArrayData,
    _phantom: PhantomData<(&'a (), T)>,
}
assert_eq_size!(Array<'_, i64>, *const ArrayData);

// Array<T> - Same size as pointer, transmute via array field
impl<'a, T: Bridge<'a>> RawConvertible for Array<'a, T> {
    fn to_raw_value(self) -> RawValue {
        const {
            assert!(std::mem::size_of::<Self>() == std::mem::size_of::<RawValue>());
        }
        RawValue { array: self.ptr }
    }

    unsafe fn from_raw_value(raw: RawValue) -> Self {
        const {
            assert!(std::mem::size_of::<Self>() == std::mem::size_of::<RawValue>());
        }
        Self {
            ptr: unsafe { raw.array },
            _phantom: PhantomData,
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_type_from() {
        let arena = Bump::new();
        let type_mgr = TypeManager::new(&arena);

        // Test primitives
        assert_eq!(i64::type_from(type_mgr), type_mgr.int());
        assert_eq!(f64::type_from(type_mgr), type_mgr.float());
        assert_eq!(bool::type_from(type_mgr), type_mgr.bool());

        // Test simple arrays
        assert_eq!(
            Array::<i64>::type_from(type_mgr),
            type_mgr.array(type_mgr.int())
        );
        assert_eq!(
            Array::<f64>::type_from(type_mgr),
            type_mgr.array(type_mgr.float())
        );
        assert_eq!(
            Array::<bool>::type_from(type_mgr),
            type_mgr.array(type_mgr.bool())
        );

        // Test nested arrays
        assert_eq!(
            Array::<Array<i64>>::type_from(type_mgr),
            type_mgr.array(type_mgr.array(type_mgr.int()))
        );
        assert_eq!(
            Array::<Array<Array<i64>>>::type_from(type_mgr),
            type_mgr.array(type_mgr.array(type_mgr.array(type_mgr.int())))
        );
    }

    #[test]
    fn test_i64_roundtrip() {
        let value: i64 = 42;
        let raw = i64::to_raw_value(value);
        let result = unsafe { i64::from_raw_value(raw) };
        assert_eq!(result, 42);
    }

    #[test]
    fn test_f64_roundtrip() {
        let value: f64 = 3.14159;
        let raw = f64::to_raw_value(value);
        let result = unsafe { f64::from_raw_value(raw) };
        assert_eq!(result, 3.14159);
    }

    #[test]
    fn test_bool_roundtrip() {
        let raw_true = bool::to_raw_value(true);
        let raw_false = bool::to_raw_value(false);
        unsafe {
            assert_eq!(bool::from_raw_value(raw_true), true);
            assert_eq!(bool::from_raw_value(raw_false), false);
        }
    }

    #[test]
    fn test_array_i64_basic() {
        let arena = Bump::new();
        let arr = Array::<i64>::new(&arena, &[1, 2, 3, 4, 5]);
        assert_eq!(arr.len(), 5);
        assert_eq!(arr.get(0), Some(1));
        assert_eq!(arr.get(2), Some(3));
        assert_eq!(arr.get(4), Some(5));
        assert_eq!(arr.get(5), None);
    }

    #[test]
    fn test_array_f64_basic() {
        let arena = Bump::new();
        let arr = Array::<f64>::new(&arena, &[1.1, 2.2, 3.3]);
        assert_eq!(arr.len(), 3);
        assert_eq!(arr.get(0), Some(1.1));
        assert_eq!(arr.get(1), Some(2.2));
        assert_eq!(arr.get(2), Some(3.3));
    }

    #[test]
    fn test_array_bool_basic() {
        let arena = Bump::new();
        let arr = Array::<bool>::new(&arena, &[true, false, true]);
        assert_eq!(arr.len(), 3);
        assert_eq!(arr.get(0), Some(true));
        assert_eq!(arr.get(1), Some(false));
        assert_eq!(arr.get(2), Some(true));
    }

    #[test]
    fn test_array_empty() {
        let arena = Bump::new();
        let arr = Array::<i64>::new(&arena, &[]);
        assert_eq!(arr.len(), 0);
        assert!(arr.is_empty());
        assert_eq!(arr.get(0), None);
    }

    #[test]
    fn test_array_nested() {
        let arena = Bump::new();
        let arr = Array::<Array<i64>>::new(
            &arena,
            &[
                Array::<i64>::new(&arena, &[1, 2]),
                Array::<i64>::new(&arena, &[3, 4, 5]),
            ],
        );
        let mut sum = 0;
        for i in 0..arr.len() {
            for j in 0..arr.get(i).unwrap().len() {
                sum += arr.get(i).unwrap().get(j).unwrap();
            }
        }
        assert_eq!(sum, 15);
    }

    #[test]
    fn test_array_get_unchecked() {
        let arena = Bump::new();
        let arr = Array::<i64>::new(&arena, &[10, 20, 30]);
        unsafe {
            assert_eq!(arr.get_unchecked(0), 10);
            assert_eq!(arr.get_unchecked(1), 20);
            assert_eq!(arr.get_unchecked(2), 30);
        }
    }

    #[test]
    fn test_array_clone_copy() {
        let arena = Bump::new();
        let arr1 = Array::<i64>::new(&arena, &[1, 2, 3]);
        let arr2 = arr1;
        let arr3 = arr1.clone();
        assert_eq!(arr1.len(), 3);
        assert_eq!(arr2.len(), 3);
        assert_eq!(arr3.len(), 3);
        assert_eq!(arr1.get(0), Some(1));
        assert_eq!(arr2.get(0), Some(1));
        assert_eq!(arr3.get(0), Some(1));
    }

    #[test]
    fn test_array_large() {
        let arena = Bump::new();
        let values: Vec<i64> = (0..1000).collect();
        let arr = Array::<i64>::new(&arena, &values);
        assert_eq!(arr.len(), 1000);
        assert_eq!(arr.get(0), Some(0));
        assert_eq!(arr.get(500), Some(500));
        assert_eq!(arr.get(999), Some(999));
        assert_eq!(arr.get(1000), None);
    }

    #[test]
    fn test_array_negative_numbers() {
        let arena = Bump::new();
        let arr = Array::<i64>::new(&arena, &[-100, -50, 0, 50, 100]);
        assert_eq!(arr.get(0), Some(-100));
        assert_eq!(arr.get(1), Some(-50));
        assert_eq!(arr.get(2), Some(0));
        assert_eq!(arr.get(3), Some(50));
        assert_eq!(arr.get(4), Some(100));
    }

    #[test]
    fn test_array_special_floats() {
        let arena = Bump::new();
        let arr = Array::<f64>::new(&arena, &[0.0, -0.0, f64::INFINITY, f64::NEG_INFINITY]);
        assert_eq!(arr.len(), 4);
        assert_eq!(arr.get(0), Some(0.0));
        assert_eq!(arr.get(1), Some(-0.0));
        assert_eq!(arr.get(2), Some(f64::INFINITY));
        assert_eq!(arr.get(3), Some(f64::NEG_INFINITY));
    }

    #[test]
    fn test_array_raw_value_roundtrip() {
        let arena = Bump::new();
        let arr1 = Array::<i64>::new(&arena, &[1, 2, 3]);
        let raw = arr1.as_raw_value();
        let arr2 = unsafe { Array::<i64>::from_raw_value(raw) };
        assert_eq!(arr2.len(), 3);
        assert_eq!(arr2.get(0), Some(1));
        assert_eq!(arr2.get(1), Some(2));
        assert_eq!(arr2.get(2), Some(3));
    }
}
