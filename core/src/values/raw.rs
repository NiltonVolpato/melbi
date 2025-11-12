#![allow(dead_code)]

use bumpalo::Bump;

#[repr(C)]
pub union RawValue {
    pub int_value: i64,
    pub float_value: f64,
    pub bool_value: bool,
    pub boxed: *const RawValue, // TODO: Can I use NonNull here?
    pub array: *const ArrayDataRepr,
    pub record: *const RecordDataRepr,
    pub map: *const MapDataRepr,
    pub slice: *const Slice,
    pub function: *const (), // Thin pointer to arena-allocated fat pointer
}

impl Copy for RawValue {}
impl Clone for RawValue {
    fn clone(&self) -> Self {
        *self
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

    // XXX: this should be called as_data_ptr() but maybe as_slice() would replace this better.
    pub(crate) fn as_ptr(&self) -> *const RawValue {
        let (_, data_offset) = Self::layout(self.length());
        unsafe { (self.ptr as *const u8).add(data_offset) as *const RawValue }
        // core::ptr::addr_of!(self._data).cast::<RawValue>()
    }

    pub unsafe fn get(&self, index: usize) -> RawValue {
        debug_assert!(index < self.length(), "Index out of bounds");
        unsafe { *self.as_ptr().add(index) }
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
pub struct MapDataRepr {
    _length: usize, // Number of key-value pairs
    _data: [RawValue; 0],
}

#[derive(Clone, Copy)]
pub struct MapData<'a> {
    ptr: *const MapDataRepr,
    _marker: core::marker::PhantomData<&'a ()>,
}

impl<'a> MapData<'a> {
    fn new_uninitialized_in(arena: &'a Bump, length: usize) -> (*mut MapDataRepr, *mut RawValue) {
        // We need space for length * 2 RawValues (key-value pairs)
        let (layout, data_offset) = Self::layout(length);

        unsafe {
            let ptr = arena.alloc_layout(layout).as_ptr();
            core::ptr::write::<usize>(ptr as *mut usize, length);
            let data = ptr.add(data_offset) as *mut RawValue;
            let map_data_ptr = ptr as *mut MapDataRepr;
            (map_data_ptr, data)
        }
    }

    /// Create a new map from sorted key-value pairs.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - `pairs` contains alternating key and value RawValues [k0, v0, k1, v1, ...]
    /// - Keys are sorted in ascending order according to Value::cmp
    /// - `pairs.len()` is even
    pub fn new_with_sorted(arena: &'a Bump, pairs: &[RawValue]) -> MapData<'a> {
        debug_assert!(
            pairs.len() % 2 == 0,
            "Pairs array must have even length (key-value pairs)"
        );

        let length = pairs.len() / 2;
        let (map, data_ptr) = Self::new_uninitialized_in(arena, length);

        for (i, &val) in pairs.iter().enumerate() {
            unsafe { core::ptr::write(data_ptr.add(i), val) };
        }

        MapData {
            ptr: map,
            _marker: core::marker::PhantomData,
        }
    }

    fn layout(n: usize) -> (core::alloc::Layout, usize) {
        let map_data_layout = core::alloc::Layout::new::<usize>();
        // Store n key-value pairs = 2n RawValues
        let elements_layout = core::alloc::Layout::array::<RawValue>(n * 2).unwrap();
        let (layout, data_offset) = map_data_layout.extend(elements_layout).unwrap();
        (layout.pad_to_align(), data_offset)
    }

    /// Returns the number of key-value pairs in the map.
    pub fn length(&self) -> usize {
        unsafe { (*self.ptr)._length }
    }

    pub(crate) fn as_ptr(&self) -> *const RawValue {
        let (_, data_offset) = Self::layout(self.length());
        unsafe { (self.ptr as *const u8).add(data_offset) as *const RawValue }
    }

    /// Get the key at the given index.
    ///
    /// # Safety
    ///
    /// The caller must ensure index < length().
    pub unsafe fn get_key(&self, index: usize) -> RawValue {
        debug_assert!(index < self.length(), "Index out of bounds");
        unsafe { *self.as_ptr().add(index * 2) }
    }

    /// Get the value at the given index.
    ///
    /// # Safety
    ///
    /// The caller must ensure index < length().
    pub unsafe fn get_value(&self, index: usize) -> RawValue {
        debug_assert!(index < self.length(), "Index out of bounds");
        unsafe { *self.as_ptr().add(index * 2 + 1) }
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
