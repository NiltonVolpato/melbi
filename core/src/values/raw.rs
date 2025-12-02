#![allow(dead_code)]
#![allow(unsafe_code)]

use core::{fmt, ptr::NonNull};

use bumpalo::Bump;

#[repr(C)]
pub union RawValue {
    // TODO: make all fields private.
    int_value: i64,
    float_value: f64,
    bool_value: bool,
    ptr: *const (),
    pub boxed: *const RawValue,
    array: *const ArrayDataRepr,
    record: *const RecordDataRepr,
    map: *const MapDataRepr,
    pub slice: *const Slice,
    option: Option<NonNull<RawValue>>,
    function_old: *const (), // Thin pointer to arena-allocated fat pointer
    function: NonNull<FunctionPtrRepr>,
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
static_assertions::assert_eq_size!(RawValue, usize);

impl Copy for RawValue {}
impl Clone for RawValue {
    fn clone(&self) -> Self {
        *self
    }
}

impl RawValue {
    /// Create an Option value at the raw level.
    ///
    /// - `None`: Represented as a null pointer (boxed = null)
    /// - `Some(value)`: Value is allocated in the arena and boxed pointer stored
    ///
    /// This encapsulates the memory layout of Option values, ensuring a single
    /// source of truth. If the representation changes, only this function needs updating.
    #[inline]
    pub fn make_optional(arena: &Bump, value: Option<RawValue>) -> RawValue {
        match value {
            None => RawValue {
                boxed: core::ptr::null(),
            },
            Some(val) => {
                let boxed = arena.alloc(val);
                RawValue {
                    boxed: boxed as *const RawValue,
                }
            }
        }
    }

    #[inline(always)]
    pub fn make_bool(value: bool) -> RawValue {
        RawValue { bool_value: value }
    }

    #[inline(always)]
    pub fn make_int(value: i64) -> RawValue {
        RawValue { int_value: value }
    }

    #[inline(always)]
    pub fn make_float(value: f64) -> RawValue {
        RawValue { float_value: value }
    }

    #[inline(always)]
    pub fn as_optional_unchecked(&self) -> Option<RawValue> {
        if unsafe { self.boxed.is_null() } {
            None
        } else {
            Some(unsafe { *self.boxed })
        }
    }

    #[inline(always)]
    pub fn as_int_unchecked(self) -> i64 {
        unsafe { self.int_value }
    }

    #[inline(always)]
    pub fn as_float_unchecked(self) -> f64 {
        unsafe { self.float_value }
    }

    #[inline(always)]
    pub fn as_bool_unchecked(self) -> bool {
        unsafe { self.bool_value }
    }

    #[inline(always)]
    pub fn as_bytes_unchecked<'a>(self) -> &'a [u8] {
        unsafe { (*self.slice).as_slice() }
    }

    #[inline(always)]
    pub fn as_str_unchecked<'a>(self) -> &'a str {
        unsafe { core::str::from_utf8_unchecked(self.as_bytes_unchecked()) }
    }

    /// Create a function value using a single allocation containing both the fat pointer
    /// and the function object.
    ///
    /// The memory layout is:
    ///
    /// ```text
    /// [*const dyn Function (16 bytes)][T object (sizeof<T> bytes)]
    ///  ^                               ^
    ///  |                               |
    ///  thin pointer stored             fat pointer's data points here
    ///  in RawValue.function
    /// ```
    ///
    /// # Arguments
    /// * `arena` - Arena to allocate the combined storage
    /// * `func` - The function value to store (will be moved into the allocation)
    ///
    /// # Returns
    /// A RawValue representing the allocated function.
    //    pub fn make_function<'a, 'b, F: super::Function<'a, 'b> + 'b>(
    pub fn make_function<'a, 'b, F: super::Function<'a, 'b> + 'b>(
        arena: &'b Bump,
        func: F,
    ) -> RawValue {
        let (layout, value_offset) = {
            let ptr_layout = core::alloc::Layout::new::<*const dyn super::Function<'a, 'b>>();
            let value_layout = core::alloc::Layout::new::<F>();
            let (layout, value_offset) = ptr_layout.extend(value_layout).unwrap();
            (layout.pad_to_align(), value_offset)
        };
        let storage = arena.alloc_layout(layout);

        // Initialize the allocation:
        // 1. Write the function object T at offset `value_offset`
        // 2. Write the fat pointer at the beginning, pointing to the T object
        unsafe {
            let func_ptr = storage.add(value_offset).as_ptr().cast::<F>();
            core::ptr::write(func_ptr, func);

            // Create fat pointer: Rust constructs vtable when casting T* to dyn Function*
            let fat_ptr: *const dyn super::Function<'a, 'b> = func_ptr;
            core::ptr::write(
                storage.as_ptr() as *mut *const dyn super::Function<'a, 'b>,
                fat_ptr,
            );
        };

        RawValue {
            function_old: storage.as_ptr() as *const (),
        }
    }

    /// Extract a function trait object reference from this RawValue.
    ///
    /// # Safety
    ///
    /// The caller must ensure this RawValue was created with `make_function`
    /// and contains a valid function pointer.
    #[inline(always)]
    pub fn as_function_unchecked<'a, 'b>(self) -> &'a dyn super::Function<'b, 'a> {
        let storage_ptr = unsafe { self.function_old as *const *const dyn super::Function<'b, 'a> };
        unsafe { &**storage_ptr }
    }

    /// Returns an id associated with this RawValue.
    ///
    /// For boxed values, if `id(a) == id(b)` then `a == b`.
    #[inline(always)]
    pub fn id(&self) -> usize {
        unsafe { self.ptr as usize }
    }
}

impl fmt::Debug for RawValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:p}", unsafe { self.boxed })
    }
}

#[repr(C)]
pub struct ArrayDataRepr {
    _length: usize,
    _data: [RawValue; 0],
}

#[derive(Clone, Copy)]
pub struct ArrayData<'a> {
    ptr: *const ArrayDataRepr,
    _marker: core::marker::PhantomData<&'a ()>,
}

impl<'a> ArrayData<'a> {
    fn new_uninitialized_in(arena: &'a Bump, length: usize) -> (*mut ArrayDataRepr, *mut RawValue) {
        let (layout, data_offset) = Self::layout(length);

        unsafe {
            let ptr = arena.alloc_layout(layout).as_ptr();
            core::ptr::write::<usize>(ptr as *mut usize, length);
            let data = ptr.add(data_offset) as *mut RawValue;
            let array_data_ptr = ptr as *mut ArrayDataRepr;
            (array_data_ptr, data)
        }
    }

    pub fn new_with(arena: &'a Bump, values: &[RawValue]) -> ArrayData<'a> {
        let (arr, data_ptr) = Self::new_uninitialized_in(arena, values.len());
        for (i, &val) in values.iter().enumerate() {
            unsafe { core::ptr::write(data_ptr.add(i), val) };
        }
        ArrayData {
            ptr: arr,
            _marker: core::marker::PhantomData,
        }
    }

    fn layout(n: usize) -> (core::alloc::Layout, usize) {
        let array_data_layout = core::alloc::Layout::new::<usize>();
        let elements_layout = core::alloc::Layout::array::<RawValue>(n).unwrap();
        let (layout, data_offset) = array_data_layout.extend(elements_layout).unwrap();
        (layout.pad_to_align(), data_offset)
    }

    pub fn length(&self) -> usize {
        unsafe { (*self.ptr)._length }
        // unsafe { *(self.ptr as *const ArrayDataRepr as *const usize) }
    }

    /// Returns a pointer to the first element of the `data` array.
    pub fn as_data_ptr(&self) -> *const RawValue {
        let (_, data_offset) = Self::layout(self.length());
        unsafe { (self.ptr as *const u8).add(data_offset) as *const RawValue }
        // core::ptr::addr_of!(self._data).cast::<RawValue>()
    }

    pub unsafe fn get_unchecked(&self, index: usize) -> RawValue {
        debug_assert!(index < self.length(), "Index out of bounds");
        unsafe { *self.as_data_ptr().add(index) }
    }

    pub(crate) fn as_raw_value(&self) -> RawValue {
        RawValue { array: self.ptr }
    }

    pub(crate) fn from_raw_value(raw: RawValue) -> Self {
        ArrayData {
            ptr: unsafe { raw.array },
            _marker: core::marker::PhantomData,
        }
    }
}

#[repr(C)]
pub struct RecordDataRepr {
    _length: usize,
    _data: [RawValue; 0],
}

#[derive(Clone, Copy)]
pub struct RecordData<'a> {
    ptr: *const RecordDataRepr,
    _marker: core::marker::PhantomData<&'a ()>,
}

impl<'a> RecordData<'a> {
    fn new_uninitialized_in(
        arena: &'a Bump,
        length: usize,
    ) -> (*mut RecordDataRepr, *mut RawValue) {
        let (layout, data_offset) = Self::layout(length);

        unsafe {
            let ptr = arena.alloc_layout(layout).as_ptr();
            core::ptr::write::<usize>(ptr as *mut usize, length);
            let data = ptr.add(data_offset) as *mut RawValue;
            let record_data_ptr = ptr as *mut RecordDataRepr;
            (record_data_ptr, data)
        }
    }

    pub fn new_with(arena: &'a Bump, values: &[RawValue]) -> RecordData<'a> {
        let (rec, data_ptr) = Self::new_uninitialized_in(arena, values.len());
        for (i, &val) in values.iter().enumerate() {
            unsafe { core::ptr::write(data_ptr.add(i), val) };
        }
        RecordData {
            ptr: rec,
            _marker: core::marker::PhantomData,
        }
    }

    fn layout(n: usize) -> (core::alloc::Layout, usize) {
        let record_data_layout = core::alloc::Layout::new::<usize>();
        let elements_layout = core::alloc::Layout::array::<RawValue>(n).unwrap();
        let (layout, data_offset) = record_data_layout.extend(elements_layout).unwrap();
        (layout.pad_to_align(), data_offset)
    }

    pub fn length(&self) -> usize {
        unsafe { (*self.ptr)._length }
    }

    pub(self) fn as_ptr(&self) -> *const RawValue {
        let (_, data_offset) = Self::layout(self.length());
        unsafe { (self.ptr as *const u8).add(data_offset) as *const RawValue }
    }

    pub unsafe fn get(&self, index: usize) -> RawValue {
        debug_assert!(index < self.length(), "Index out of bounds");
        unsafe { *self.as_ptr().add(index) }
    }

    pub(crate) fn as_raw_value(&self) -> RawValue {
        RawValue { record: self.ptr }
    }

    pub(crate) fn from_raw_value(raw: RawValue) -> Self {
        RecordData {
            ptr: unsafe { raw.record },
            _marker: core::marker::PhantomData,
        }
    }
}

#[repr(C)]
pub struct Slice {
    data: *const u8,
    length: usize,
}

impl Slice {
    pub fn new<'a>(arena: &'a Bump, value: &[u8]) -> &'a Self {
        arena.alloc(Slice {
            data: value.as_ptr(),
            length: value.len(),
        })
    }

    pub fn length(&self) -> usize {
        self.length
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.data, self.length) }
    }

    pub(crate) fn as_raw_value(&self) -> RawValue {
        RawValue {
            slice: self as *const Slice,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MapEntry {
    pub key: RawValue,
    pub value: RawValue,
}

#[repr(C)]
pub struct MapDataRepr {
    _length: usize, // Number of key-value pairs
    _data: [MapEntry; 0],
}

#[derive(Clone, Copy)]
pub struct MapData<'a> {
    ptr: *const MapDataRepr,
    _marker: core::marker::PhantomData<&'a ()>,
}

impl<'a> MapData<'a> {
    fn new_uninitialized_in(arena: &'a Bump, length: usize) -> (*mut MapDataRepr, *mut MapEntry) {
        let (layout, data_offset) = Self::layout(length);

        unsafe {
            let ptr = arena.alloc_layout(layout).as_ptr();
            core::ptr::write::<usize>(ptr as *mut usize, length);
            let data = ptr.add(data_offset) as *mut MapEntry;
            let map_data_ptr = ptr as *mut MapDataRepr;
            (map_data_ptr, data)
        }
    }

    /// Create a new map from sorted key-value pairs.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - Keys are sorted in ascending order according to Value::cmp
    pub fn new_with_sorted(arena: &'a Bump, entries: &[MapEntry]) -> MapData<'a> {
        let length = entries.len();
        let (map, data_ptr) = Self::new_uninitialized_in(arena, length);

        for (i, &entry) in entries.iter().enumerate() {
            unsafe { core::ptr::write(data_ptr.add(i), entry) };
        }

        MapData {
            ptr: map,
            _marker: core::marker::PhantomData,
        }
    }

    fn layout(n: usize) -> (core::alloc::Layout, usize) {
        let map_data_layout = core::alloc::Layout::new::<usize>();
        let elements_layout = core::alloc::Layout::array::<MapEntry>(n).unwrap();
        let (layout, data_offset) = map_data_layout.extend(elements_layout).unwrap();
        (layout.pad_to_align(), data_offset)
    }

    /// Returns the number of key-value pairs in the map.
    pub fn length(&self) -> usize {
        unsafe { (*self.ptr)._length }
    }

    pub(crate) fn as_ptr(&self) -> *const MapEntry {
        let (_, data_offset) = Self::layout(self.length());
        unsafe { (self.ptr as *const u8).add(data_offset) as *const MapEntry }
    }

    /// Get the key at the given index.
    ///
    /// # Safety
    ///
    /// The caller must ensure index < length().
    pub unsafe fn get_key(&self, index: usize) -> RawValue {
        debug_assert!(index < self.length(), "Index out of bounds");
        unsafe { (*self.as_ptr().add(index)).key }
    }

    /// Get the value at the given index.
    ///
    /// # Safety
    ///
    /// The caller must ensure index < length().
    pub unsafe fn get_value(&self, index: usize) -> RawValue {
        debug_assert!(index < self.length(), "Index out of bounds");
        unsafe { (*self.as_ptr().add(index)).value }
    }

    pub(crate) fn as_raw_value(&self) -> RawValue {
        RawValue { map: self.ptr }
    }

    pub(crate) fn from_raw_value(raw: RawValue) -> Self {
        MapData {
            ptr: unsafe { raw.map },
            _marker: core::marker::PhantomData,
        }
    }
}

#[repr(C)]
struct FunctionPtrRepr {
    dyn_ptr: NonNull<dyn for<'a, 'b> super::Function<'a, 'b>>,
}

#[repr(C)]
struct FunctionRepr<'a, 'b, F: super::Function<'a, 'b>> {
    dyn_ptr: *const dyn super::Function<'a, 'b>,
    func: F,
}
