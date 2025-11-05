//! TeenyVec: A 16-byte small vector optimized for inline storage.
//!
//! TeenyVec provides a compact vector type that:
//! - Is exactly 16 bytes (2 registers on x86-64/arm64)
//! - Stores up to 15 bytes inline without heap allocation
//! - Grows to heap seamlessly when needed
//! - Uses odd/even discriminant for stack/heap detection

#![allow(dead_code)]

use alloc::alloc::{Layout, alloc};
use core::{
    mem::ManuallyDrop,
    ptr::{self, NonNull},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TeenyVecKind {
    Heap,
    Stack,
}

// Option 1:
// * cap_lo is always even when allocated on the heap
// * so we make len always odd when allocated on the stack (2 * actual_length + 1)
//
// struct Heap  { cap_lo: u8, cap_hi: u8, len: u16, data: NonNull<u8> }
// struct Stack { len: u8,    data: [u8; 15]                          }
//
// Option 2:
// * same applies: `cap` is always even in the heap, make `len` always odd in the stack.
//
// struct Heap  { cap: u16, len: u16, data: NonNull<u8> }
// struct Stack { len: u16, data: [u8; 14]              }
//
// Option 3:
// * remove capacity and define it as the next power of 2 after the length
// * heap doesn't need to use the first u16 in this case
// * so when `len == 0` data is on the heap, otherwise `len - 1` is the actual length of stack data.
//
// struct Heap  { _: u16,   len: u16, data: NonNull<u8> }
// struct Stack { len: u16, data: [u8; 14]              }

struct Heap {
    cap_lo: u8, // always even
    cap_hi: u8,
    len: u16,
    data: NonNull<u8>,
}

struct Stack {
    len: u8, // always odd
    data: [u8; 15],
}

union TeenyVecRepr {
    heap: ManuallyDrop<Heap>,
    stack: ManuallyDrop<Stack>,
}

pub struct TeenyVec {
    repr: TeenyVecRepr,
}

impl TeenyVec {
    pub fn new() -> Self {
        Self {
            repr: TeenyVecRepr {
                stack: ManuallyDrop::new(Stack {
                    len: 1,
                    data: [0; 15],
                }),
            },
        }
    }

    #[inline(always)]
    fn kind(&self) -> TeenyVecKind {
        if unsafe { self.repr.stack.len } % 2 == 1 {
            TeenyVecKind::Stack
        } else {
            TeenyVecKind::Heap
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        match self.kind() {
            TeenyVecKind::Stack => (unsafe { (self.repr.stack.len - 1) / 2 }) as usize,
            TeenyVecKind::Heap => (unsafe { self.repr.heap.len }) as usize,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn cap(&self) -> usize {
        match self.kind() {
            TeenyVecKind::Stack => 15usize,
            TeenyVecKind::Heap => {
                let cap_lo = unsafe { self.repr.heap.cap_lo };
                let cap_hi = unsafe { self.repr.heap.cap_hi };
                ((cap_hi as usize) << 8) | (cap_lo as usize)
            }
        }
    }

    pub fn push(&mut self, value: u8) {
        if self.len() == self.cap() {
            self.reserve_one_unchecked();
        }
        unsafe {
            match self.kind() {
                TeenyVecKind::Stack => {
                    let stack = &mut self.repr.stack;
                    ptr::write(stack.data.as_mut_ptr().add(self.len()), value);
                }
                TeenyVecKind::Heap => {
                    let heap = &mut self.repr.heap;
                    ptr::write(heap.data.add(self.len()).as_ptr(), value);
                }
            }
        }
        self.inc_len();
    }

    #[cold]
    fn reserve_one_unchecked(&mut self) {
        debug_assert_eq!(self.len(), self.cap());
        let new_cap = (self.len() as u16)
            .checked_add(1)
            .and_then(u16::checked_next_power_of_two)
            .expect("capacity overflow");
        self.grow(new_cap as usize);
    }

    fn set_cap(&mut self, new_cap: usize) {
        let new_cap: u16 = new_cap.try_into().expect("capacity overflow");
        assert!(new_cap % 2 == 0);
        let heap = unsafe { &mut self.repr.heap };
        heap.cap_lo = (new_cap & 0xff) as u8;
        heap.cap_hi = (new_cap >> 8) as u8;
    }

    #[inline(always)]
    fn set_len(&mut self, new_len: usize) {
        match self.kind() {
            TeenyVecKind::Stack => {
                let stack = unsafe { &mut self.repr.stack };
                stack.len = (2 * new_len + 1).try_into().expect("capacity overflow");
            }
            TeenyVecKind::Heap => {
                let heap = unsafe { &mut self.repr.heap };
                heap.len = new_len.try_into().expect("capacity overflow");
            }
        }
    }

    #[inline(always)]
    fn inc_len(&mut self) {
        match self.kind() {
            TeenyVecKind::Stack => {
                let stack = unsafe { &mut self.repr.stack };
                stack.len += 2;
            }
            TeenyVecKind::Heap => {
                let heap = unsafe { &mut self.repr.heap };
                heap.len += 1;
            }
        }
    }

    pub fn grow(&mut self, mut new_cap: usize) {
        unsafe {
            assert!(new_cap >= self.len());
            let kind = self.kind();
            if kind == TeenyVecKind::Stack {
                if new_cap < 32 {
                    new_cap = 32;
                }
                let src = &self.repr.stack.data[..self.len()];
                let ptr = alloc(Layout::array::<u8>(new_cap).unwrap());
                ptr::copy_nonoverlapping(src.as_ptr(), ptr, self.len());
                self.repr.heap = ManuallyDrop::new(Heap {
                    data: NonNull::new_unchecked(ptr),
                    cap_lo: (new_cap & 0xff) as u8,
                    cap_hi: (new_cap >> 8) as u8,
                    len: self.len() as u16,
                });
                assert!(self.kind() == TeenyVecKind::Heap);
                return;
            }
            // Heap to larger heap
            let old_cap = self.cap();
            let old_ptr = self.repr.heap.data.as_ptr();
            let old_len = self.len();

            let new_ptr = alloc(Layout::array::<u8>(new_cap).unwrap());
            ptr::copy_nonoverlapping(old_ptr, new_ptr, old_len);

            // Free old allocation
            alloc::alloc::dealloc(old_ptr, Layout::array::<u8>(old_cap).unwrap());

            // Update to new allocation
            let heap = &mut self.repr.heap;
            heap.data = NonNull::new_unchecked(new_ptr);
            self.set_cap(new_cap);
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        match self.kind() {
            TeenyVecKind::Stack => {
                let stack = unsafe { &self.repr.stack };
                &stack.data[..self.len()]
            }
            TeenyVecKind::Heap => unsafe {
                let heap = &self.repr.heap;
                let ptr = heap.data.as_ptr();
                alloc::slice::from_raw_parts(ptr, self.len())
            },
        }
    }
}

impl Clone for TeenyVec {
    fn clone(&self) -> Self {
        match self.kind() {
            TeenyVecKind::Stack => {
                let this = unsafe { &self.repr.stack };
                Self {
                    repr: TeenyVecRepr {
                        stack: ManuallyDrop::new(Stack {
                            len: this.len,
                            data: this.data,
                        }),
                    },
                }
            }
            TeenyVecKind::Heap => {
                let this = unsafe { &self.repr.heap };
                let data = unsafe {
                    let data = alloc(Layout::array::<u8>(self.cap()).unwrap());
                    ptr::copy_nonoverlapping(this.data.as_ptr(), data, self.len());
                    data
                };
                Self {
                    repr: TeenyVecRepr {
                        heap: ManuallyDrop::new(Heap {
                            cap_lo: this.cap_lo,
                            cap_hi: this.cap_hi,
                            len: this.len,
                            data: unsafe { NonNull::new_unchecked(data) },
                        }),
                    },
                }
            }
        }
    }
}

impl Drop for TeenyVec {
    fn drop(&mut self) {
        if self.kind() == TeenyVecKind::Heap {
            unsafe {
                let heap = &self.repr.heap;
                let ptr = heap.data.as_ptr();
                let cap = self.cap();
                alloc::alloc::dealloc(ptr, Layout::array::<u8>(cap).unwrap());
            }
        }
        // Stack variant has no heap allocation, nothing to clean up
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grow() {
        let mut vec = TeenyVec::new();
        vec.push(1);
        vec.push(2);
        vec.push(3);
        assert_eq!(vec.len(), 3);
        assert_eq!(vec.cap(), 15);
        assert_eq!(vec.as_slice(), &[1, 2, 3]);
        for i in 4..15 {
            vec.push(i);
        }
        assert_eq!(vec.len(), 14);
        assert_eq!(vec.cap(), 15);
        assert_eq!(
            vec.as_slice(),
            &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14]
        );
        for i in 15..18 {
            vec.push(i);
        }
        assert_eq!(vec.len(), 17);
        assert_eq!(vec.cap(), 32);
        assert_eq!(
            vec.as_slice(),
            &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17]
        );
    }

    #[test]
    fn test_heap_to_heap_grow() {
        let mut vec = TeenyVec::new();
        // Fill to trigger stack->heap
        for i in 0..16 {
            vec.push(i);
        }
        assert_eq!(vec.len(), 16);
        assert_eq!(vec.cap(), 32);

        // Now trigger heap->heap growth
        for i in 16..33 {
            vec.push(i);
        }
        assert_eq!(vec.len(), 33);
        assert_eq!(vec.cap(), 64);

        // Verify all data is intact
        for i in 0..33 {
            assert_eq!(vec.as_slice()[i as usize], i);
        }
    }

    #[test]
    fn test_drop_stack() {
        // Just create and drop a stack variant
        let mut vec = TeenyVec::new();
        vec.push(1);
        vec.push(2);
        assert_eq!(vec.len(), 2);
        // Drop happens here, should not leak
    }

    #[test]
    fn test_drop_heap() {
        // Create and drop a heap variant
        let mut vec = TeenyVec::new();
        for i in 0..20 {
            vec.push(i);
        }
        assert_eq!(vec.len(), 20);
        assert_eq!(vec.cap(), 32);
        // Drop happens here, should free heap memory
    }

    #[test]
    fn test_inline_capacity() {
        let mut vec = TeenyVec::new();
        // Push exactly 15 items (max inline)
        for i in 0..15 {
            vec.push(i);
        }
        assert_eq!(vec.len(), 15);
        assert_eq!(vec.cap(), 15);
        assert_eq!(vec.kind(), TeenyVecKind::Stack);

        // Next push should trigger heap
        vec.push(15);
        assert_eq!(vec.len(), 16);
        assert_eq!(vec.cap(), 32);
        assert_eq!(vec.kind(), TeenyVecKind::Heap);
    }

    #[test]
    fn test_clone_stack() {
        let mut vec = TeenyVec::new();
        for i in 0..14 {
            vec.push(i);
        }

        assert_eq!(vec.len(), 14);
        let cloned = vec.clone();
        assert_eq!(vec.as_slice(), cloned.as_slice());
    }

    #[test]
    fn test_clone_heap() {
        let mut vec = TeenyVec::new();
        for i in 0..100 {
            vec.push(i);
        }

        assert_eq!(vec.len(), 100);
        let cloned = vec.clone();
        assert_eq!(vec.as_slice(), cloned.as_slice());
    }
}
