//! A fixed-capacity, inline container with ownership-preserving operations.

const CAPACITY: usize = 32;

/// Owns up to 32 elements without allocating a separate storage buffer.
///
/// The occupied prefix contains only `Some(T)`; every remaining slot is `None`.
/// Element types may allocate internally, but this container does not. Its
/// physical stack or heap placement depends on where the value is stored.
/// No `Copy`, `Clone`, or `Default` bound is required for elements.
///
/// ```
/// use rust_homework_8::MyVec;
///
/// let mut values = MyVec::new();
/// values.push(String::from("first")).unwrap();
/// values.push(String::from("second")).unwrap();
/// assert_eq!(values.get(0).map(String::as_str), Some("first"));
/// assert_eq!(values.pop().as_deref(), Some("second"));
/// ```
#[derive(Debug)]
pub struct MyVec<T> {
    data: [Option<T>; CAPACITY],
    len: usize,
}

impl<T> MyVec<T> {
    /// Creates an empty container without constructing any `T` values.
    pub fn new() -> Self {
        Self {
            data: std::array::from_fn(|_| None),
            len: 0,
        }
    }

    /// Takes ownership of `value` and appends it if a slot is available.
    ///
    /// # Errors
    ///
    /// Returns the original `value` unchanged when all 32 slots are occupied.
    /// On that error, no element is dropped and the container is unchanged.
    pub fn push(&mut self, value: T) -> Result<(), T> {
        if self.len == CAPACITY {
            return Err(value);
        }
        self.data[self.len] = Some(value);
        self.len += 1;
        Ok(())
    }

    /// Removes the last element, transferring its ownership to the caller.
    /// Returns `None` when empty, without modifying the container.
    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }
        self.len -= 1;
        self.data[self.len].take()
    }

    /// Returns the number of occupied slots, between 0 and 32 inclusive.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns whether the container holds no elements.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Borrows an element, or returns `None` for any index outside `0..len()`.
    pub fn get(&self, idx: usize) -> Option<&T> {
        if idx < self.len {
            self.data[idx].as_ref()
        } else {
            None
        }
    }

    /// Exclusively borrows an element, or returns `None` outside `0..len()`.
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut T> {
        if idx < self.len {
            self.data[idx].as_mut()
        } else {
            None
        }
    }

    /// Iterates over borrowed elements in insertion order, without cloning.
    ///
    /// The container cannot be mutated while returned references are in use.
    ///
    /// ```compile_fail
    /// use rust_homework_8::MyVec;
    ///
    /// let mut values = MyVec::new();
    /// values.push(String::from("first")).unwrap();
    /// let first = values.iter().next().unwrap();
    /// values.clear();
    /// println!("{first}");
    /// ```
    pub fn iter(&self) -> MyVecIter<'_, T> {
        MyVecIter { vec: self, pos: 0 }
    }

    /// Removes and drops all elements, starting with the last one.
    ///
    /// If an element destructor unwinds, the container retains a valid occupied
    /// prefix of the remaining elements; clearing is not transactional. The
    /// element has been removed before its destructor is called. No guarantee
    /// of continued execution is made for aborts or a second unwinding panic.
    pub fn clear(&mut self) {
        while let Some(value) = self.pop() {
            drop(value);
        }
    }
}

/// Borrows elements in insertion order, just like [`MyVec::iter`].
///
/// Converting `&MyVec<T>` to an iterator neither consumes the container nor
/// clones elements. Returned references cannot outlive the shared borrow.
impl<'a, T> IntoIterator for &'a MyVec<T> {
    type Item = &'a T;
    type IntoIter = MyVecIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T> Default for MyVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Borrows a `MyVec` and tracks the next occupied position.
///
/// Iteration does not consume elements. Once exhausted, `next()` continues to
/// return `None`. The iterator allocates no storage.
#[derive(Debug)]
pub struct MyVecIter<'a, T> {
    vec: &'a MyVec<T>,
    pos: usize,
}

impl<'a, T> Iterator for MyVecIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.vec.get(self.pos)?;
        self.pos += 1;
        Some(value)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.vec.len() - self.pos;
        (remaining, Some(remaining))
    }
}
