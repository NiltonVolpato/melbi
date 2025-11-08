#![allow(dead_code)]

use super::function::FunctionData;
use bumpalo::Bump;

#[repr(C)]
pub union RawValue {
    pub int_value: i64,
    pub float_value: f64,
    pub bool_value: bool,
    pub boxed: *const RawValue, // TODO: Can I use NonNull here?
    pub array: *const ArrayDataRepr,
    pub record: *const RecordDataRepr,
    pub slice: *const Slice,
    pub function: *const FunctionData,
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
