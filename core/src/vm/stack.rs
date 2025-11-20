#![allow(dead_code)]
use crate::Vec;
use alloc::fmt;

/// A stack data structure with maximum size enforcement in debug mode.
///
/// This stack is used by the VM for value storage during execution. The maximum
/// size is only enforced in debug builds to catch stack overflow bugs during
/// development, while maintaining zero overhead in release builds.
///
/// # Examples
///
/// ```ignore
/// use melbi_core::vm::Stack;
///
/// let mut stack = Stack::new(100);
/// stack.push(42);
/// stack.push(17);
/// assert_eq!(stack.pop(), Some(17));
/// assert_eq!(stack.peek(), Some(&42));
/// assert_eq!(stack.len(), 1);
/// ```
pub struct Stack<T> {
    /// The underlying storage for stack elements.
    items: Vec<T>,
    /// Maximum allowed stack size (enforced in debug mode only).
    max_size: usize,
}

impl<T> Stack<T> {
    /// Creates a new stack with the specified maximum size.
    ///
    /// The stack will pre-allocate a reasonable amount of space to avoid
    /// frequent reallocations during normal operation.
    ///
    /// # Arguments
    ///
    /// * `max_size` - Maximum number of elements allowed on the stack.
    ///                This is only enforced in debug builds.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use melbi_core::vm::Stack;
    ///
    /// let stack: Stack<i32> = Stack::new(1000);
    /// assert_eq!(stack.capacity(), 1000);
    /// assert_eq!(stack.len(), 0);
    /// ```
    pub fn new(max_size: usize) -> Self {
        // Pre-allocate a reasonable amount (min of max_size or 256)
        // to avoid frequent reallocations during normal execution
        let initial_capacity = max_size.min(256);

        Self {
            items: Vec::with_capacity(initial_capacity),
            max_size,
        }
    }

    /// Pushes a value onto the stack.
    ///
    /// In debug builds, this will panic if pushing would exceed the maximum
    /// stack size. In release builds, no bounds checking is performed for
    /// maximum performance.
    ///
    /// # Panics
    ///
    /// Panics in debug mode if the stack is already at maximum capacity.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use melbi_core::vm::Stack;
    ///
    /// let mut stack = Stack::new(100);
    /// stack.push(42);
    /// stack.push(17);
    /// assert_eq!(stack.len(), 2);
    /// ```
    #[inline]
    pub fn push(&mut self, value: T) {
        debug_assert!(
            self.items.len() < self.max_size,
            "Stack overflow: attempted to push beyond maximum size of {}",
            self.max_size
        );
        self.items.push(value);
    }

    /// Removes and returns the top value from the stack.
    ///
    /// Returns `None` if the stack is empty.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use melbi_core::vm::Stack;
    ///
    /// let mut stack = Stack::new(100);
    /// stack.push(42);
    /// assert_eq!(stack.pop(), Some(42));
    /// assert_eq!(stack.pop(), None);
    /// ```
    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        // TODO: Change this to return T. Add debug_assert! for underflow.
        self.items.pop()
    }

    /// Returns a reference to the top value without removing it.
    ///
    /// Returns `None` if the stack is empty.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use melbi_core::vm::Stack;
    ///
    /// let mut stack = Stack::new(100);
    /// stack.push(42);
    /// assert_eq!(stack.peek(), Some(&42));
    /// assert_eq!(stack.len(), 1); // Value is still on the stack
    /// ```
    #[inline]
    pub fn peek(&self) -> Option<&T> {
        self.items.last()
    }

    /// Returns a mutable reference to the top value without removing it.
    ///
    /// Returns `None` if the stack is empty.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use melbi_core::vm::Stack;
    ///
    /// let mut stack = Stack::new(100);
    /// stack.push(42);
    /// if let Some(top) = stack.peek_mut() {
    ///     *top = 100;
    /// }
    /// assert_eq!(stack.pop(), Some(100));
    /// ```
    #[inline]
    pub fn peek_mut(&mut self) -> Option<&mut T> {
        self.items.last_mut()
    }

    /// Returns the current number of elements on the stack.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use melbi_core::vm::Stack;
    ///
    /// let mut stack = Stack::new(100);
    /// assert_eq!(stack.len(), 0);
    /// stack.push(1);
    /// stack.push(2);
    /// assert_eq!(stack.len(), 2);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns `true` if the stack contains no elements.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use melbi_core::vm::Stack;
    ///
    /// let mut stack = Stack::new(100);
    /// assert!(stack.is_empty());
    /// stack.push(42);
    /// assert!(!stack.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns the maximum capacity of the stack.
    ///
    /// Note that this limit is only enforced in debug builds.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use melbi_core::vm::Stack;
    ///
    /// let stack: Stack<i32> = Stack::new(500);
    /// assert_eq!(stack.capacity(), 500);
    /// ```
    #[inline]
    pub fn capacity(&self) -> usize {
        self.max_size
    }

    /// Clears the stack, removing all values.
    ///
    /// This does not deallocate the underlying storage.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use melbi_core::vm::Stack;
    ///
    /// let mut stack = Stack::new(100);
    /// stack.push(1);
    /// stack.push(2);
    /// stack.clear();
    /// assert_eq!(stack.len(), 0);
    /// assert!(stack.is_empty());
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Removes the top `n` elements from the stack.
    ///
    /// If `n` is greater than the current stack size, all elements are removed.
    /// This is useful for cleaning up after operations that consume multiple
    /// stack values (like function calls or array construction).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use melbi_core::vm::Stack;
    ///
    /// let mut stack = Stack::new(100);
    /// stack.push(10);
    /// stack.push(20);
    /// stack.push(30);
    /// stack.push(40);
    ///
    /// // Remove top 2 elements
    /// stack.pop_n(2);
    /// assert_eq!(stack.len(), 2);
    /// assert_eq!(stack.peek(), Some(&20));
    ///
    /// // Remove more than remaining
    /// stack.pop_n(10);
    /// assert_eq!(stack.len(), 0);
    /// ```
    #[inline]
    pub fn pop_n(&mut self, n: usize) {
        let new_len = self.len().saturating_sub(n);
        self.items.truncate(new_len);
    }

    /// Returns an iterator over references to the stack elements.
    ///
    /// The iterator yields elements from bottom to top.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use melbi_core::vm::Stack;
    ///
    /// let mut stack = Stack::new(100);
    /// stack.push(1);
    /// stack.push(2);
    /// stack.push(3);
    ///
    /// let items: Vec<_> = stack.iter().copied().collect();
    /// assert_eq!(items, vec![1, 2, 3]);
    /// ```
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter()
    }

    /// Returns a slice of the top `n` elements on the stack.
    ///
    /// The slice is ordered from bottom to top (so `slice[0]` is the oldest
    /// of the n elements, and `slice[n-1]` is the top of the stack).
    ///
    /// Returns `None` if there are fewer than `n` elements on the stack.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use melbi_core::vm::Stack;
    ///
    /// let mut stack = Stack::new(100);
    /// stack.push(10);
    /// stack.push(20);
    /// stack.push(30);
    /// stack.push(40);
    ///
    /// // Get top 2 elements
    /// let top_2 = stack.top_n(2).unwrap();
    /// assert_eq!(top_2, &[30, 40]); // [second from top, top]
    ///
    /// // Get top 3 elements
    /// let top_3 = stack.top_n(3).unwrap();
    /// assert_eq!(top_3, &[20, 30, 40]);
    ///
    /// // Not enough elements
    /// assert_eq!(stack.top_n(10), None);
    /// ```
    #[inline]
    pub fn top_n(&self, n: usize) -> Option<&[T]> {
        let len = self.items.len();
        if n > len {
            None
        } else {
            Some(&self.items[len - n..])
        }
    }

    /// Returns a mutable slice of the top `n` elements on the stack.
    ///
    /// The slice is ordered from bottom to top (so `slice[0]` is the oldest
    /// of the n elements, and `slice[n-1]` is the top of the stack).
    ///
    /// Returns `None` if there are fewer than `n` elements on the stack.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use melbi_core::vm::stack::Stack;
    ///
    /// let mut stack = Stack::new(100);
    /// stack.push(10);
    /// stack.push(20);
    /// stack.push(30);
    ///
    /// // Modify top 2 elements
    /// if let Some(top_2) = stack.top_n_mut(2) {
    ///     top_2[0] = 99; // Second from top
    ///     top_2[1] = 88; // Top
    /// }
    ///
    /// assert_eq!(stack.pop(), Some(88));
    /// assert_eq!(stack.pop(), Some(99));
    /// ```
    #[inline]
    pub fn top_n_mut(&mut self, n: usize) -> Option<&mut [T]> {
        let len = self.items.len();
        if n > len {
            None
        } else {
            Some(&mut self.items[len - n..])
        }
    }
}

impl<T: Clone> Stack<T> {
    /// Returns a reference to the element at the specified distance from the top.
    ///
    /// `offset = 0` returns the top element (same as `peek()`).
    /// `offset = 1` returns the element below the top, etc.
    ///
    /// Returns `None` if the offset is out of bounds.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use melbi_core::vm::Stack;
    ///
    /// let mut stack = Stack::new(100);
    /// stack.push(10);
    /// stack.push(20);
    /// stack.push(30);
    ///
    /// assert_eq!(stack.peek_at(0), Some(&30)); // Top
    /// assert_eq!(stack.peek_at(1), Some(&20)); // Below top
    /// assert_eq!(stack.peek_at(2), Some(&10)); // Bottom
    /// assert_eq!(stack.peek_at(3), None);      // Out of bounds
    /// ```
    #[inline]
    pub fn peek_at(&self, offset: usize) -> Option<&T> {
        let len = self.items.len();
        if offset >= len {
            None
        } else {
            Some(&self.items[len - 1 - offset])
        }
    }

    /// Duplicates the top element of the stack.
    ///
    /// Returns `true` if successful, `false` if the stack is empty.
    ///
    /// # Panics
    ///
    /// Panics in debug mode if duplicating would exceed the maximum stack size.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use melbi_core::vm::Stack;
    ///
    /// let mut stack = Stack::new(100);
    /// stack.push(42);
    /// assert!(stack.dup());
    /// assert_eq!(stack.pop(), Some(42));
    /// assert_eq!(stack.pop(), Some(42));
    /// ```
    #[inline]
    pub fn dup(&mut self) -> bool {
        if let Some(value) = self.peek().cloned() {
            self.push(value);
            true
        } else {
            false
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for Stack<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Stack")
            .field("items", &self.items)
            .field("len", &self.items.len())
            .field("capacity", &self.max_size)
            .finish()
    }
}

impl<T> core::ops::Index<usize> for Stack<T> {
    type Output = T;

    /// Indexes the stack from the top.
    ///
    /// `stack[0]` returns the top element, `stack[1]` returns the element below it, etc.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use melbi_core::vm::Stack;
    ///
    /// let mut stack = Stack::new(100);
    /// stack.push(10);
    /// stack.push(20);
    /// stack.push(30);
    ///
    /// assert_eq!(stack[0], 30); // Top
    /// assert_eq!(stack[1], 20); // Below top
    /// assert_eq!(stack[2], 10); // Bottom
    /// ```
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        let len = self.items.len();
        assert!(
            index < len,
            "Stack index out of bounds: index {} but stack has {} elements",
            index,
            len
        );
        &self.items[len - 1 - index]
    }
}

impl<T> core::ops::IndexMut<usize> for Stack<T> {
    /// Indexes the stack from the top (mutable).
    ///
    /// `stack[0]` returns the top element, `stack[1]` returns the element below it, etc.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use melbi_core::vm::Stack;
    ///
    /// let mut stack = Stack::new(100);
    /// stack.push(10);
    /// stack.push(20);
    /// stack.push(30);
    ///
    /// stack[0] = 99; // Modify top
    /// assert_eq!(stack[0], 99);
    /// ```
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let len = self.items.len();
        assert!(
            index < len,
            "Stack index out of bounds: index {} but stack has {} elements",
            index,
            len
        );
        &mut self.items[len - 1 - index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_stack() {
        let stack: Stack<i32> = Stack::new(100);
        assert_eq!(stack.len(), 0);
        assert_eq!(stack.capacity(), 100);
        assert!(stack.is_empty());
    }

    #[test]
    fn test_push_pop() {
        let mut stack = Stack::new(100);
        stack.push(1);
        stack.push(2);
        stack.push(3);

        assert_eq!(stack.len(), 3);
        assert_eq!(stack.pop(), Some(3));
        assert_eq!(stack.pop(), Some(2));
        assert_eq!(stack.pop(), Some(1));
        assert_eq!(stack.pop(), None);
    }

    #[test]
    fn test_peek() {
        let mut stack = Stack::new(100);
        assert_eq!(stack.peek(), None);

        stack.push(42);
        assert_eq!(stack.peek(), Some(&42));
        assert_eq!(stack.len(), 1); // Peek doesn't remove

        stack.push(17);
        assert_eq!(stack.peek(), Some(&17));

        // Clean up
        stack.clear();
    }

    #[test]
    fn test_peek_mut() {
        let mut stack = Stack::new(100);
        stack.push(42);

        if let Some(top) = stack.peek_mut() {
            *top = 100;
        }

        assert_eq!(stack.pop(), Some(100));
    }

    #[test]
    fn test_peek_at() {
        let mut stack = Stack::new(100);
        stack.push(10);
        stack.push(20);
        stack.push(30);

        assert_eq!(stack.peek_at(0), Some(&30));
        assert_eq!(stack.peek_at(1), Some(&20));
        assert_eq!(stack.peek_at(2), Some(&10));
        assert_eq!(stack.peek_at(3), None);

        // Clean up
        stack.clear();
    }

    #[test]
    fn test_dup() {
        let mut stack = Stack::new(100);

        // Dup on empty stack
        assert!(!stack.dup());

        // Dup with value
        stack.push(42);
        assert!(stack.dup());
        assert_eq!(stack.len(), 2);
        assert_eq!(stack.pop(), Some(42));
        assert_eq!(stack.pop(), Some(42));
    }

    #[test]
    fn test_clear() {
        let mut stack = Stack::new(100);
        stack.push(1);
        stack.push(2);
        stack.push(3);

        stack.clear();
        assert_eq!(stack.len(), 0);
        assert!(stack.is_empty());
    }

    #[test]
    fn test_iter() {
        let mut stack = Stack::new(100);
        stack.push(1);
        stack.push(2);
        stack.push(3);

        let items: Vec<_> = stack.iter().copied().collect();
        assert_eq!(items, vec![1, 2, 3]);

        // Clean up
        stack.clear();
    }

    #[test]
    #[should_panic(expected = "Stack overflow")]
    #[cfg(debug_assertions)]
    fn test_overflow_debug() {
        let mut stack = Stack::new(2);
        stack.push(1);
        stack.push(2);
        stack.push(3); // Should panic in debug mode
    }

    #[test]
    fn test_large_stack() {
        let mut stack = Stack::new(10000);
        for i in 0..1000 {
            stack.push(i);
        }

        assert_eq!(stack.len(), 1000);

        for i in (0..1000).rev() {
            assert_eq!(stack.pop(), Some(i));
        }
    }
}
