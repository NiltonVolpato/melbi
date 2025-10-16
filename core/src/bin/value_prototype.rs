// test_value.rs
use bumpalo::Bump;
use std::marker::PhantomData;
use std::mem;

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
        // Pre-allocate all types we need
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
    boxed: *const u8,
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
    // followed by [RawValue; length]
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
            if index >= data.length {
                return Err(TypeError::IndexOutOfBounds);
            }

            let elements =
                (self.ptr as *const u8).add(mem::size_of::<ArrayData>()) as *const RawValue;
            let raw = *elements.add(index);

            T::from_value(self.elem_ty, raw, self.type_mgr)
        }
    }

    pub fn len(&self) -> usize {
        unsafe { (*self.ptr).length }
    }
}

// ============================================================================
// Helper to create arrays
// ============================================================================

fn create_array(arena: &Bump, values: &[RawValue]) -> *const ArrayData {
    let layout = std::alloc::Layout::from_size_align(
        mem::size_of::<ArrayData>() + mem::size_of::<RawValue>() * values.len(),
        mem::align_of::<ArrayData>(),
    )
    .unwrap();

    unsafe {
        let ptr = arena.alloc_layout(layout).as_ptr() as *mut ArrayData;
        (*ptr).length = values.len();

        let elements = (ptr as *mut u8).add(mem::size_of::<ArrayData>()) as *mut RawValue;
        for (i, &val) in values.iter().enumerate() {
            *elements.add(i) = val;
        }

        ptr
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
    let inner = create_array(
        &arena,
        &[
            RawValue { int_value: 1 },
            RawValue { int_value: 2 },
            RawValue { int_value: 3 },
        ],
    );

    // Create outer array containing one inner array
    let outer = create_array(
        &arena,
        &[RawValue {
            boxed: inner as *const u8,
        }],
    );

    // Wrap in Value
    let value = Value {
        ty: outer_array_ty,
        raw: RawValue {
            boxed: outer as *const u8,
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
