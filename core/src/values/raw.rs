#![allow(dead_code)]

use bumpalo::Bump;
use static_assertions::assert_eq_size;

#[repr(C)]
pub union RawValue {
    pub int_value: i64,
    pub float_value: f64,
    pub bool_value: bool,
    pub boxed: *const RawValue, // TODO: Can I use NonNull here?
    pub array: *const ArrayData,
    pub slice: *const Slice,
}
assert_eq_size!(RawValue, *const RawValue);

impl Copy for RawValue {}
impl Clone for RawValue {
    fn clone(&self) -> Self {
        *self
    }
}

#[repr(C)]
pub struct ArrayData {
    length: usize,
    data: [RawValue; 0],
}

impl ArrayData {
    pub fn new_uninitialized_in<'a>(arena: &'a Bump, length: usize) -> &'a mut ArrayData {
        let layout = Self::layout(length);

        unsafe {
            let ptr = arena.alloc_layout(layout).as_ptr() as *mut ArrayData;
            (*ptr).length = length;
            &mut *ptr
        }
    }

    pub fn new_with<'a>(arena: &'a Bump, values: &[RawValue]) -> &'a ArrayData {
        let arr = Self::new_uninitialized_in(arena, values.len());
        for (i, &val) in values.iter().enumerate() {
            unsafe { arr.set(i, val) };
        }
        arr
    }

    fn layout(n: usize) -> std::alloc::Layout {
        let array_data_layout = std::alloc::Layout::new::<usize>();
        let elements_layout = std::alloc::Layout::array::<RawValue>(n).unwrap();
        let (layout, _data_offset) = array_data_layout.extend(elements_layout).unwrap();
        layout.pad_to_align()
    }

    pub fn length(&self) -> usize {
        self.length
    }

    pub unsafe fn get(&self, index: usize) -> RawValue {
        debug_assert!(index < self.length, "Index out of bounds");
        unsafe { *self.data.as_ptr().add(index) }
    }

    pub unsafe fn set(&mut self, index: usize, value: RawValue) {
        debug_assert!(index < self.length, "Index out of bounds");
        unsafe {
            *self.data.as_mut_ptr().add(index) = value;
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
        unsafe { std::slice::from_raw_parts(self.data, self.length) }
    }
}
