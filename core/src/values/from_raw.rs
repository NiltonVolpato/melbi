use core::marker::PhantomData;

use crate::{
    types::{Type, manager::TypeManager},
    values::raw::{ArrayData, RawValue},
};

#[derive(Debug)]
pub enum TypeError {
    Mismatch,
    IndexOutOfBounds,
}

impl core::fmt::Display for TypeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl core::error::Error for TypeError {}

pub trait FromRawValue<'a>: Sized {
    fn type_descr(type_mgr: &'a TypeManager<'a>) -> &'a Type<'a>;

    fn from_raw(
        type_mgr: &'a TypeManager<'a>,
        ty: &'a Type<'a>,
        raw: RawValue,
    ) -> Result<Self, TypeError>;
}

// i64 implementation
impl<'a> FromRawValue<'a> for i64 {
    fn type_descr(type_mgr: &'a TypeManager<'a>) -> &'a Type<'a> {
        type_mgr.int()
    }

    fn from_raw(
        type_mgr: &'a TypeManager<'a>,
        ty: &'a Type<'a>,
        raw: RawValue,
    ) -> Result<Self, TypeError> {
        let expected = Self::type_descr(type_mgr);
        if !core::ptr::eq(ty, expected) {
            return Err(TypeError::Mismatch);
        }
        unsafe { Ok(raw.int_value) }
    }
}

// f64 implementation
impl<'a> FromRawValue<'a> for f64 {
    fn type_descr(type_mgr: &'a TypeManager<'a>) -> &'a Type<'a> {
        type_mgr.float()
    }

    fn from_raw(
        type_mgr: &'a TypeManager<'a>,
        ty: &'a Type<'a>,
        raw: RawValue,
    ) -> Result<Self, TypeError> {
        let expected = Self::type_descr(type_mgr);
        if !core::ptr::eq(ty, expected) {
            return Err(TypeError::Mismatch);
        }
        unsafe { Ok(raw.float_value) }
    }
}

// bool implementation
impl<'a> FromRawValue<'a> for bool {
    fn type_descr(type_mgr: &'a TypeManager<'a>) -> &'a Type<'a> {
        type_mgr.bool()
    }

    fn from_raw(
        type_mgr: &'a TypeManager<'a>,
        ty: &'a Type<'a>,
        raw: RawValue,
    ) -> Result<Self, TypeError> {
        let expected = Self::type_descr(type_mgr);
        if !core::ptr::eq(ty, expected) {
            return Err(TypeError::Mismatch);
        }
        unsafe { Ok(raw.bool_value) }
    }
}

// &str implementation
impl<'a> FromRawValue<'a> for &'a str {
    fn type_descr(type_mgr: &'a TypeManager<'a>) -> &'a Type<'a> {
        type_mgr.str()
    }

    fn from_raw(
        type_mgr: &'a TypeManager<'a>,
        ty: &'a Type<'a>,
        raw: crate::values::raw::RawValue,
    ) -> Result<Self, TypeError> {
        let expected = Self::type_descr(type_mgr);
        if !core::ptr::eq(ty, expected) {
            return Err(TypeError::Mismatch);
        }
        // SAFETY: slice points to a valid Slice representing a UTF-8 string
        unsafe {
            let slice = &*(raw.slice);
            let bytes = slice.as_slice();
            debug_assert!(
                core::str::from_utf8(bytes).is_ok(),
                "invalid UTF-8 in string value"
            );
            Ok(core::str::from_utf8_unchecked(bytes))
        }
    }
}

// &[u8] implementation
impl<'a> FromRawValue<'a> for &'a [u8] {
    fn type_descr(type_mgr: &'a TypeManager<'a>) -> &'a Type<'a> {
        type_mgr.bytes()
    }

    fn from_raw(
        type_mgr: &'a TypeManager<'a>,
        ty: &'a Type<'a>,
        raw: crate::values::raw::RawValue,
    ) -> Result<Self, TypeError> {
        let expected = Self::type_descr(type_mgr);
        if !core::ptr::eq(ty, expected) {
            return Err(TypeError::Mismatch);
        }
        // SAFETY: slice points to a valid Slice representing a byte array
        unsafe {
            let slice = &*(raw.slice);
            Ok(slice.as_slice())
        }
    }
}

// Array implementation
pub struct Array<'a, T> {
    elem_ty: &'a Type<'a>,
    ptr: *const ArrayData,
    _phantom: PhantomData<(&'a (), T)>,
}

impl<'a, T: FromRawValue<'a>> FromRawValue<'a> for Array<'a, T> {
    fn type_descr(type_mgr: &'a TypeManager<'a>) -> &'a Type<'a> {
        let elem_ty = T::type_descr(type_mgr);
        type_mgr.array(elem_ty)
    }

    fn from_raw(
        type_mgr: &'a TypeManager<'a>,
        ty: &'a Type<'a>,
        raw: RawValue,
    ) -> Result<Self, TypeError> {
        let expected = Self::type_descr(type_mgr);
        if !core::ptr::eq(ty, expected) {
            return Err(TypeError::Mismatch);
        }

        let Type::Array(elem_ty) = ty else {
            unreachable!()
        };

        unsafe {
            Ok(Array {
                elem_ty,
                ptr: raw.array,
                _phantom: PhantomData,
            })
        }
    }
}

impl<'a, T: FromRawValue<'a>> Array<'a, T> {
    pub fn get(&self, type_mgr: &'a TypeManager<'a>, index: usize) -> Result<T, TypeError> {
        unsafe {
            let data = &*self.ptr;
            if index >= data.length() {
                return Err(TypeError::IndexOutOfBounds);
            }

            let raw = data.get(index);
            T::from_raw(type_mgr, self.elem_ty, raw)
        }
    }

    pub fn len(&self) -> usize {
        unsafe { (*self.ptr).length() }
    }

    pub fn as_raw_value(&self) -> RawValue {
        RawValue { array: self.ptr }
    }
}
