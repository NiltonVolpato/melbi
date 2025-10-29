// Value prototype for Melbi core library.
use bumpalo::Bump;
use std::marker::PhantomData;

// ============================================================================
// Type system (minimal version)
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Type<'a> {
    Int,
    Array(&'a Type<'a>),
}

pub struct TypeManager<'a> {
    interned: &'a [&'a Type<'a>],
}

impl<'a> TypeManager<'a> {
    fn new(arena: &'a Bump) -> Self {
        // Pre-allocate all types we need to simplify.
        let int_ty: &'a Type<'a> = arena.alloc(Type::Int);
        let inner_array_ty: &'a Type<'a> = arena.alloc(Type::Array(int_ty));
        let outer_array_ty: &'a Type<'a> = arena.alloc(Type::Array(inner_array_ty));

        Self {
            interned: arena.alloc_slice_copy(&[int_ty, inner_array_ty, outer_array_ty]),
        }
    }

    fn int(&self) -> &'a Type<'a> {
        self.interned[0]
    }

    fn array(&self, elem_ty: &'a Type<'a>) -> &'a Type<'a> {
        if elem_ty == self.interned[0] {
            return self.interned[1];
        }
        if elem_ty == self.interned[1] {
            return self.interned[2];
        }
        panic!("Type not pre-interned");
    }
}

// ============================================================================
// Raw value representation (VM side)
// ============================================================================

#[repr(C)]
pub union RawValue {
    int_value: i64,
    float_value: f64,
    bool_value: bool,
    boxed: *const RawValue,
}

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
    pub fn new_in(arena: &Bump, length: usize) -> &mut ArrayData {
        let layout = Self::layout(length);

        unsafe {
            let ptr = arena.alloc_layout(layout).as_ptr() as *mut ArrayData;
            (*ptr).length = length;
            &mut *ptr
        }
    }

    pub fn new_with<'a>(arena: &'a Bump, values: &[RawValue]) -> &'a mut ArrayData {
        let arr = Self::new_in(arena, values.len());
        unsafe {
            std::ptr::copy_nonoverlapping(values.as_ptr(), arr.data.as_mut_ptr(), values.len());
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

    pub fn get(&self, index: usize) -> RawValue {
        debug_assert!(index < self.length, "Index out of bounds");
        unsafe { *self.data.as_ptr().add(index) }
    }

    pub fn set(&mut self, index: usize, value: RawValue) {
        debug_assert!(index < self.length, "Index out of bounds");
        unsafe {
            *self.data.as_mut_ptr().add(index) = value;
        }
    }
}

// ============================================================================
// Safe value wrapper (user-facing API)
// ============================================================================

pub struct Value<'a> {
    ty: &'a Type<'a>,
    raw: RawValue,
    type_mgr: &'a TypeManager<'a>,
}

impl<'a> Value<'a> {
    pub fn get<T: FromValue<'a>>(&self) -> Result<T, TypeError> {
        T::from_value(self.ty, self.raw, self.type_mgr)
    }
}

#[derive(Debug)]
pub enum TypeError {
    Mismatch,
    IndexOutOfBounds,
}

// ============================================================================
// FromValue trait
// ============================================================================

pub trait FromValue<'a>: Sized {
    fn melbi_type_descriptor(type_mgr: &TypeManager<'a>) -> &'a Type<'a>;

    fn from_value(
        ty: &'a Type<'a>,
        raw: RawValue,
        type_mgr: &'a TypeManager<'a>,
    ) -> Result<Self, TypeError>;
}

// i64 implementation
impl FromValue<'_> for i64 {
    fn melbi_type_descriptor<'a>(type_mgr: &TypeManager<'a>) -> &'a Type<'a> {
        type_mgr.int()
    }

    fn from_value(ty: &Type, raw: RawValue, type_mgr: &TypeManager) -> Result<Self, TypeError> {
        let expected = Self::melbi_type_descriptor(type_mgr);
        if !std::ptr::eq(ty, expected) {
            return Err(TypeError::Mismatch);
        }
        unsafe { Ok(raw.int_value) }
    }
}

// Array implementation
pub struct Array<'a, T> {
    ptr: *const ArrayData,
    elem_ty: &'a Type<'a>,
    type_mgr: &'a TypeManager<'a>,
    _phantom: PhantomData<(&'a (), T)>,
}

impl<'a, T: FromValue<'a>> FromValue<'a> for Array<'a, T> {
    fn melbi_type_descriptor(type_mgr: &TypeManager<'a>) -> &'a Type<'a> {
        let elem_ty = T::melbi_type_descriptor(type_mgr);
        type_mgr.array(elem_ty)
    }

    fn from_value(
        ty: &'a Type<'a>,
        raw: RawValue,
        type_mgr: &'a TypeManager<'a>,
    ) -> Result<Self, TypeError> {
        let expected = Self::melbi_type_descriptor(type_mgr);
        if !std::ptr::eq(ty, expected) {
            return Err(TypeError::Mismatch);
        }

        let Type::Array(elem_ty) = ty else {
            unreachable!()
        };

        unsafe {
            Ok(Array {
                ptr: raw.boxed as *const ArrayData,
                elem_ty,
                type_mgr,
                _phantom: PhantomData,
            })
        }
    }
}

impl<'a, T: FromValue<'a>> Array<'a, T> {
    pub fn get(&self, index: usize) -> Result<T, TypeError> {
        unsafe {
            let data = &*self.ptr;
            if index >= data.length() {
                return Err(TypeError::IndexOutOfBounds);
            }

            let raw = data.get(index);
            T::from_value(self.elem_ty, raw, self.type_mgr)
        }
    }

    pub fn len(&self) -> usize {
        unsafe { (*self.ptr).length }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// ============================================================================
// Example usage
// ============================================================================

fn main() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    // Pre-allocate all types we need
    let int_ty = type_mgr.int();
    let inner_array_ty = type_mgr.array(int_ty);
    let outer_array_ty = type_mgr.array(inner_array_ty);

    // Create inner array: [1, 2, 3]
    let inner = ArrayData::new_with(
        &arena,
        &[
            RawValue { int_value: 1 },
            RawValue { int_value: 2 },
            RawValue { int_value: 3 },
        ],
    ) as *const ArrayData;

    // Create outer array containing one inner array
    let outer = ArrayData::new_with(
        &arena,
        &[RawValue {
            boxed: inner as *const RawValue,
        }],
    ) as *const ArrayData;

    // Wrap in Value
    let value = Value {
        ty: outer_array_ty,
        raw: RawValue {
            boxed: outer as *const RawValue,
        },
        type_mgr: &type_mgr,
    };

    // Now use the safe API!
    let array: Array<'_, Array<'_, i64>> = value.get::<Array<Array<i64>>>().unwrap();
    println!("Outer array length: {}", array.len());

    let inner = array.get(0).unwrap();
    println!("Inner array length: {}", inner.len());

    let val = inner.get(0).unwrap();
    println!("Value at [0][0]: {}", val);

    assert_eq!(val, 1);
    println!("Success!");
}
