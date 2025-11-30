#![allow(unsafe_code)]

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::fmt;

#[repr(C)]
pub union RawValue {
    int: i64,
    bool: bool,
    array: *const Vec<RawValue>,
}
impl Copy for RawValue {}
impl Clone for RawValue {
    fn clone(&self) -> Self {
        *self
    }
}
impl fmt::Debug for RawValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:p}", unsafe { self.array })
    }
}

impl RawValue {
    pub fn new_int(value: i64) -> Self {
        RawValue { int: value }
    }

    pub fn new_bool(value: bool) -> Self {
        RawValue { bool: value }
    }

    pub fn new_array(values: &[RawValue]) -> Self {
        // Just an example. In real life I don't want to leak memory.
        let boxed: Box<Vec<RawValue>> = Box::new(Vec::from(values));
        let array = Box::into_raw(boxed); // XXX
        RawValue { array }
    }

    pub fn as_int_unchecked(&self) -> i64 {
        unsafe { self.int }
    }

    pub fn as_bool_unchecked(&self) -> bool {
        unsafe { self.bool }
    }

    pub fn as_array_unchecked(&self) -> &[RawValue] {
        unsafe { &*self.array }
    }
}
