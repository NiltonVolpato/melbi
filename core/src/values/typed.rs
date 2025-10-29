//! Tier 1: Statically-typed, compile-time safe value API
//!
//! This module provides zero-overhead, compile-time type-safe wrappers around
//! the untyped RawValue representation. Types are guaranteed at compile time,
//! eliminating the need for runtime type checking or TypeManager.

use crate::{String, Vec};
use core::marker::PhantomData;
use core::ops::Deref;

use bumpalo::Bump;

use crate::{
    Type,
    types::manager::TypeManager,
    values::raw::{ArrayData, RawValue, Slice},
};

pub trait RawConvertible<'arena>: Sized {
    fn to_raw_value(arena: &'arena Bump, value: Self) -> RawValue;
    unsafe fn from_raw_value(raw: RawValue) -> Self;
}

pub trait Bridge<'a>: RawConvertible<'a> {
    type Raw: Sized;
    fn type_from(type_mgr: &'a TypeManager<'a>) -> &'a Type<'a>;
}

/// Typed wrapper around a string slice stored in the arena.
///
/// Internally stores a pointer to a Slice. Can be constructed from
/// both `&str` and `String`, with the latter taking ownership and
/// allocating in the arena.
///
/// Implements `Deref<Target = str>` for seamless usage as a string slice.
#[repr(transparent)]
#[derive(Debug)]
pub struct Str<'a> {
    slice: *const Slice,
    _phantom: PhantomData<&'a ()>,
}

impl<'a> Copy for Str<'a> {}
impl<'a> Clone for Str<'a> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a> Str<'a> {
    /// Create from a &str by allocating in the arena
    pub fn from_str(arena: &'a Bump, s: &str) -> Self {
        // Allocate the string's bytes into the arena to ensure they live long enough
        let bytes: &'a [u8] = arena.alloc_slice_copy(s.as_bytes());
        let slice = Slice::new(arena, bytes);
        Str {
            slice: slice as *const Slice,
            _phantom: PhantomData,
        }
    }

    /// Create from a String by taking ownership and allocating in arena
    pub fn from_string(arena: &'a Bump, s: String) -> Self {
        // Allocate the string's bytes into the arena first
        let bytes: &'a [u8] = arena.alloc_slice_copy(s.as_bytes());
        let slice = Slice::new(arena, bytes);
        Str {
            slice: slice as *const Slice,
            _phantom: PhantomData,
        }
    }

    /// Get the underlying &str
    pub fn as_str(&self) -> &'a str {
        unsafe {
            let slice = &*self.slice;
            let bytes = slice.as_slice();
            core::str::from_utf8_unchecked(bytes)
        }
    }

    /// Get the length in bytes
    pub fn len(&self) -> usize {
        unsafe { (*self.slice).length() }
    }

    /// Check if the string is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<'a> Deref for Str<'a> {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl<'a> From<Str<'a>> for &'a str {
    fn from(s: Str<'a>) -> &'a str {
        s.as_str()
    }
}

impl<'a> AsRef<str> for Str<'a> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<'a> PartialEq for Str<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl<'a> PartialEq<str> for Str<'a> {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl<'a> PartialEq<&str> for Str<'a> {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl<'a> Eq for Str<'a> {}

impl<'arena> RawConvertible<'arena> for i64 {
    fn to_raw_value(_arena: &'arena Bump, value: Self) -> RawValue {
        RawValue { int_value: value }
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

impl<'arena> RawConvertible<'arena> for f64 {
    fn to_raw_value(_arena: &'arena Bump, value: Self) -> RawValue {
        RawValue { float_value: value }
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

impl<'arena> RawConvertible<'arena> for bool {
    fn to_raw_value(_arena: &'arena Bump, value: Self) -> RawValue {
        RawValue { bool_value: value }
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

impl<'a> RawConvertible<'a> for Str<'a> {
    fn to_raw_value(_arena: &'a Bump, value: Self) -> RawValue {
        RawValue { slice: value.slice }
    }

    unsafe fn from_raw_value(raw: RawValue) -> Self {
        Str {
            slice: unsafe { raw.slice },
            _phantom: PhantomData,
        }
    }
}

impl<'a> Bridge<'a> for Str<'a> {
    type Raw = *const Slice;
    fn type_from(type_mgr: &'a TypeManager<'a>) -> &'a Type<'a> {
        type_mgr.str()
    }
}

impl<'a> RawConvertible<'a> for &'a [u8] {
    fn to_raw_value(arena: &'a Bump, value: Self) -> RawValue {
        let slice = Slice::new(arena, value);
        slice.as_raw_value()
    }

    unsafe fn from_raw_value(raw: RawValue) -> Self {
        let slice = unsafe { &*raw.slice };
        slice.as_slice()
    }
}

impl<'a> Bridge<'a> for &'a [u8] {
    type Raw = &'a [u8];
    fn type_from(type_mgr: &'a TypeManager<'a>) -> &'a Type<'a> {
        type_mgr.bytes()
    }
}

impl<'a, E: Bridge<'a>> Bridge<'a> for Array<'a, E> {
    type Raw = ArrayData<'a>;
    fn type_from(type_mgr: &'a TypeManager<'a>) -> &'a Type<'a> {
        let elem_ty = E::type_from(type_mgr);
        type_mgr.array(elem_ty)
    }
}

/// Static typed array with compile-time element type.
///
/// # Lifetime Safety with Strings
///
/// The lifetime parameter `'a` ensures references remain valid:
///
/// ```compile_fail
/// # use bumpalo::Bump;
/// # use melbi_core::values::{Array, Str};
/// let arena = Bump::new();
/// let arr = {
///     let shorter_lived_arena = Bump::new();
///     let s = Str::from_str(&shorter_lived_arena, "temp");
///     Array::new(&arena, &[s]) // ERROR: `shorter_lived_arena` dropped here while still borrowed
/// };
/// ```
#[repr(transparent)]
pub struct Array<'a, T: Bridge<'a>> {
    array_data: ArrayData<'a>,
    _phantom: PhantomData<(&'a (), T)>,
}

// Array<T> - Same size as pointer, transmute via array field
impl<'a, T: Bridge<'a>> RawConvertible<'a> for Array<'a, T> {
    fn to_raw_value(_arena: &'a Bump, value: Self) -> RawValue {
        const {
            assert!(core::mem::size_of::<Self>() == core::mem::size_of::<RawValue>());
        }
        value.as_raw_value()
    }

    unsafe fn from_raw_value(raw: RawValue) -> Self {
        const {
            assert!(core::mem::size_of::<Self>() == core::mem::size_of::<RawValue>());
        }
        Self {
            array_data: ArrayData::from_raw_value(raw),
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
        let raw_values: Vec<RawValue> = values.iter().map(|&v| T::to_raw_value(arena, v)).collect();

        // Allocate in arena
        let data = ArrayData::new_with(arena, &raw_values);

        Self {
            array_data: data,
            _phantom: PhantomData,
        }
    }

    /// Create a new array from owned values that will be moved into the arena.
    ///
    /// This is useful for types that are not Copy, like String.
    /// The values are consumed and allocated in the arena.
    pub fn from_iter(arena: &'a Bump, values: impl IntoIterator<Item = T>) -> Self {
        // Convert Rust values to RawValue representation
        let raw_values: Vec<RawValue> = values
            .into_iter()
            .map(|v| T::to_raw_value(arena, v))
            .collect();

        // Allocate in arena
        let data = ArrayData::new_with(arena, &raw_values);

        Self {
            array_data: data,
            _phantom: PhantomData,
        }
    }
}

impl<'a> Array<'a, Str<'a>> {
    /// Create an array of strings from an iterator of string-like values.
    ///
    /// Accepts both `&str` and `String` values, allocating owned strings
    /// into the arena.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let arr = Array::from_strs(&arena, vec!["hello", "world"]);
    /// let arr = Array::from_strs(&arena, vec![s1, s2]); // where s1, s2: String
    /// ```
    pub fn from_strs(arena: &'a Bump, strs: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        let str_values: Vec<Str<'a>> = strs
            .into_iter()
            .map(|s| Str::from_str(arena, s.as_ref()))
            .collect();

        Self::from_iter(arena, str_values)
    }
}

impl<'a, T: Bridge<'a>> Array<'a, T> {
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
            if index >= self.array_data.length() {
                return None;
            }
            let raw = self.array_data.get(index);
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
            debug_assert!(index < self.array_data.length(), "Index out of bounds");
            let raw = self.array_data.get(index);
            T::from_raw_value(raw)
        }
    }

    /// Returns the number of elements in the array.
    pub fn len(&self) -> usize {
        self.array_data.length()
    }

    /// Returns `true` if the array is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get a pointer to the underlying ArrayData for FFI/VM use.
    ///
    /// This is useful for bridging to Tier 2 (DynamicValue) or Tier 3 (RawValue).
    pub fn as_raw_value(&self) -> RawValue {
        self.array_data.as_raw_value()
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
            array_data: ArrayData::from_raw_value(raw),
            _phantom: PhantomData,
        }
    }

    /// Returns an iterator over the array elements.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let arr = Array::new(&arena, &[1, 2, 3, 4, 5]);
    /// let sum: i64 = arr.iter().sum();
    /// assert_eq!(sum, 15);
    /// ```
    pub fn iter(&self) -> ArrayIter<'a, T> {
        unsafe {
            let start = self.array_data.as_ptr();
            let end = start.add(self.array_data.length());
            ArrayIter {
                current: start,
                end,
                _phantom: PhantomData,
            }
        }
    }
}

impl<'a, T: Bridge<'a>> Clone for Array<'a, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T: Bridge<'a>> Copy for Array<'a, T> {}

/// Iterator over typed Array elements.
///
/// Uses start/end pointer strategy for efficient iteration without bounds checks.
pub struct ArrayIter<'a, T: Bridge<'a>> {
    current: *const RawValue,
    end: *const RawValue,
    _phantom: PhantomData<(&'a (), T)>,
}

impl<'a, T: Bridge<'a>> Iterator for ArrayIter<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.end {
            return None;
        }

        let raw = unsafe { *self.current };
        self.current = unsafe { self.current.add(1) };

        Some(unsafe { T::from_raw_value(raw) })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = unsafe { self.end.offset_from(self.current) as usize };
        (remaining, Some(remaining))
    }
}

impl<'a, T: Bridge<'a>> ExactSizeIterator for ArrayIter<'a, T> {
    fn len(&self) -> usize {
        unsafe { self.end.offset_from(self.current) as usize }
    }
}

impl<'a, T: Bridge<'a>> IntoIterator for Array<'a, T> {
    type Item = T;
    type IntoIter = ArrayIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T: Bridge<'a>> IntoIterator for &'a Array<'a, T> {
    type Item = T;
    type IntoIter = ArrayIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

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
        let arena = Bump::new();
        let value: i64 = 42;
        let raw = i64::to_raw_value(&arena, value);
        let result = unsafe { i64::from_raw_value(raw) };
        assert_eq!(result, 42);
    }

    #[test]
    fn test_f64_roundtrip() {
        let arena = Bump::new();
        let value: f64 = 3.14159;
        let raw = f64::to_raw_value(&arena, value);
        let result = unsafe { f64::from_raw_value(raw) };
        assert_eq!(result, 3.14159);
    }

    #[test]
    fn test_bool_roundtrip() {
        let arena = Bump::new();
        let raw_true = bool::to_raw_value(&arena, true);
        let raw_false = bool::to_raw_value(&arena, false);
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

    #[test]
    fn test_str_bridge_type_from() {
        let arena = Bump::new();
        let type_mgr = TypeManager::new(&arena);

        // Test that Str implements Bridge correctly
        assert_eq!(<Str>::type_from(type_mgr), type_mgr.str());
    }

    #[test]
    fn test_bytes_bridge_type_from() {
        let arena = Bump::new();
        let type_mgr = TypeManager::new(&arena);

        // Test that &[u8] implements Bridge correctly
        assert_eq!(<&[u8]>::type_from(type_mgr), type_mgr.bytes());
    }

    #[test]
    fn test_str_from_raw_value() {
        use crate::values::raw::Slice;

        let arena = Bump::new();

        // Create a Slice with string data
        let data = b"hello";
        let slice = Slice::new(&arena, data);
        let raw = slice.as_raw_value();

        // Extract as Str
        let result = unsafe { <Str>::from_raw_value(raw) };
        assert_eq!(result.as_str(), "hello");
        assert_eq!(result, "hello"); // Test PartialEq<&str>
    }

    #[test]
    fn test_bytes_from_raw_value() {
        use crate::values::raw::Slice;

        let arena = Bump::new();

        // Create a Slice with byte data
        let data = b"\x00\x01\x02\xFF";
        let slice = Slice::new(&arena, data);
        let raw = slice.as_raw_value();

        // Extract as &[u8]
        let result = unsafe { <&[u8]>::from_raw_value(raw) };
        assert_eq!(result, &[0x00, 0x01, 0x02, 0xFF]);
    }

    #[test]
    fn test_array_of_str_type() {
        let arena = Bump::new();
        let type_mgr = TypeManager::new(&arena);

        // Test that Array<Str> implements Bridge correctly
        assert_eq!(
            Array::<Str>::type_from(type_mgr),
            type_mgr.array(type_mgr.str())
        );
    }

    #[test]
    fn test_array_of_bytes_type() {
        let arena = Bump::new();
        let type_mgr = TypeManager::new(&arena);

        // Test that Array<&[u8]> implements Bridge correctly
        assert_eq!(
            Array::<&[u8]>::type_from(type_mgr),
            type_mgr.array(type_mgr.bytes())
        );
    }

    #[test]
    fn test_nested_array_with_str_type() {
        let arena = Bump::new();
        let type_mgr = TypeManager::new(&arena);

        // Test that nested arrays with strings work
        assert_eq!(
            Array::<Array<Str>>::type_from(type_mgr),
            type_mgr.array(type_mgr.array(type_mgr.str()))
        );
    }

    #[test]
    fn test_array_str_with_from_strs() {
        let arena = Bump::new();

        // Create strings using various methods
        let strings = vec![
            format!("Long string {}: {}", 1, "x".repeat(100)),
            format!("Long string {}: {}", 2, "x".repeat(100)),
            format!("Long string {}: {}", 3, "x".repeat(100)),
        ];

        // Create Array<Str> using from_strs
        let str_array = Array::from_strs(&arena, strings);

        // Access strings from array
        assert_eq!(str_array.len(), 3);

        // Verify they're the long strings (using Deref)
        let first_str = str_array.get(0).unwrap();
        assert!(first_str.len() > 100);
        assert!(first_str.as_str().contains("Long string 1"));

        // Test PartialEq with &str
        let expected = format!("Long string 1: {}", "x".repeat(100));
        assert_eq!(first_str.as_str(), expected.as_str());
    }

    #[test]
    fn test_array_from_owned_strings() {
        let arena = Bump::new();

        // Create owned strings
        let strings = vec![
            format!("Hello {}", "world"),
            format!("Number: {}", 42),
            String::from("Rust"),
        ];

        // Create Array<Str> from owned Strings using from_strs
        // The arena will take ownership and allocate them
        let str_array = Array::from_strs(&arena, strings);

        assert_eq!(str_array.len(), 3);
        assert_eq!(str_array.get(0).unwrap().as_str(), "Hello world");
        assert_eq!(str_array.get(1).unwrap().as_str(), "Number: 42");
        assert_eq!(str_array.get(2).unwrap().as_str(), "Rust");
    }

    #[test]
    fn test_str_deref_and_equality() {
        let arena = Bump::new();

        let s1 = Str::from_str(&arena, "hello world");
        let s2 = Str::from_string(&arena, String::from("hello world"));

        // Test Deref
        assert_eq!(&*s1, "hello world");
        assert!(s1.starts_with("hello"));
        assert_eq!(s1.len(), 11);

        // Test PartialEq between Str instances - compare via as_str
        assert_eq!(s1.as_str(), s2.as_str());

        // Test as_str
        assert_eq!(s1.as_str(), "hello world");
        assert_eq!(s2.as_str(), "hello world");
    }

    #[test]
    fn test_array_iter_basic() {
        let arena = Bump::new();
        let arr = Array::new(&arena, &[1, 2, 3, 4, 5]);

        let values: Vec<i64> = arr.iter().collect();
        assert_eq!(values, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_array_iter_sum() {
        let arena = Bump::new();
        let arr = Array::new(&arena, &[1, 2, 3, 4, 5]);

        let sum: i64 = arr.iter().sum();
        assert_eq!(sum, 15);
    }

    #[test]
    fn test_array_iter_for_loop() {
        let arena = Bump::new();
        let arr = Array::new(&arena, &[10, 20, 30]);

        let mut sum = 0;
        for val in arr {
            // Tests IntoIterator
            sum += val;
        }
        assert_eq!(sum, 60);
    }

    #[test]
    fn test_array_iter_empty() {
        let arena = Bump::new();
        let arr = Array::<i64>::new(&arena, &[]);

        assert_eq!(arr.iter().count(), 0);
    }

    #[test]
    fn test_array_iter_with_str() {
        let arena = Bump::new();
        let arr = Array::from_strs(&arena, vec!["hello", "world", "rust"]);

        let strings: Vec<&str> = arr.iter().map(|s| s.as_str()).collect();
        assert_eq!(strings, vec!["hello", "world", "rust"]);
    }

    #[test]
    fn test_array_iter_exact_size() {
        let arena = Bump::new();
        let arr = Array::new(&arena, &[1, 2, 3, 4, 5]);

        let mut iter = arr.iter();
        assert_eq!(iter.len(), 5);

        iter.next();
        assert_eq!(iter.len(), 4);

        iter.next();
        assert_eq!(iter.len(), 3);
    }

    #[test]
    fn test_array_iter_map_filter() {
        let arena = Bump::new();
        let arr = Array::new(&arena, &[1, 2, 3, 4, 5, 6]);

        // Filter even numbers and double them
        let result: Vec<i64> = arr.iter().filter(|&x| x % 2 == 0).map(|x| x * 2).collect();

        assert_eq!(result, vec![4, 8, 12]);
    }

    #[test]
    fn test_array_iter_size_hint() {
        let arena = Bump::new();
        let arr = Array::new(&arena, &[1, 2, 3]);

        let mut iter = arr.iter();
        assert_eq!(iter.size_hint(), (3, Some(3)));

        iter.next();
        assert_eq!(iter.size_hint(), (2, Some(2)));
    }

    #[test]
    fn test_array_iter_nested() {
        let arena = Bump::new();

        // Create inner arrays
        let arr1 = Array::new(&arena, &[1, 2, 3]);
        let arr2 = Array::new(&arena, &[4, 5, 6]);
        let arr3 = Array::new(&arena, &[7, 8, 9]);

        // Create outer array of arrays
        let nested = Array::new(&arena, &[arr1, arr2, arr3]);

        // Iterate over outer array
        let mut sum = 0;
        for inner_arr in nested {
            // Iterate over each inner array
            for val in inner_arr {
                sum += val;
            }
        }

        assert_eq!(sum, 45); // 1+2+3+4+5+6+7+8+9 = 45
    }
}
