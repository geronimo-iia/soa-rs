use crate::{
    AsMutSlice, AsSlice, IntoIter, Iter, IterMut, Slice, SliceMut, SliceRef, SoaClone, SoaRaw,
    Soars, Vec, drain::Drain, iter_raw::IterRaw,
};
use core::{
    borrow::{Borrow, BorrowMut},
    cmp::Ordering,
    fmt::{self, Debug, Formatter},
    hash::{Hash, Hasher},
    marker::PhantomData,
    mem::{ManuallyDrop, needs_drop, size_of},
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

/// A growable array type that stores the values for each field of `T`
/// contiguously.
///
/// The design for SoA aligns closely with [`Vec`]:
/// - Overallocates capacity to provide O(1) amortized insertion
/// - Does not allocate until elements are added
/// - Never deallocates memory unless explicitly requested
/// - Uses `usize::MAX` as the capacity for zero-sized types
///
/// See the top-level [`soa_rs`] docs for usage examples.
///
/// [`soa_rs`]: crate
pub struct Soa<T>
where
    T: Soars,
{
    pub(crate) cap: usize,
    pub(crate) slice: Slice<T, ()>,
    pub(crate) len: usize,
}

impl<T> Soa<T>
where
    T: Soars,
{
    /// The capacity of the initial allocation. This is an optimization to avoid
    /// excessive reallocation for small array sizes.
    const SMALL_CAPACITY: usize = 4;

    /// Constructs a new, empty `Soa<T>`.
    ///
    /// The container will not allocate until elements are pushed onto it.
    ///
    /// # Examples
    /// ```
    /// # use soa_rs::{Soa, Soars};
    /// # #[derive(Soars, Copy, Clone)]
    /// # #[soa_derive(Debug, PartialEq)]
    /// # struct Foo;
    /// let mut soa = Soa::<Foo>::new();
    /// ```
    pub fn new() -> Self {
        Self {
            cap: if size_of::<T>() == 0 { usize::MAX } else { 0 },
            slice: Slice::empty(),
            len: 0,
        }
    }

    /// Construct a new, empty `Soa<T>` with at least the specified capacity.
    ///
    /// The container will be able to hold `capacity` elements without
    /// reallocating. If the `capacity` is 0, the container will not allocate.
    /// Note that although the returned vector has the minimum capacity
    /// specified, the vector will have a zero length. The capacity will be as
    /// specified unless `T` is zero-sized, in which case the capacity will be
    /// `usize::MAX`.
    ///
    /// # Examples
    /// ```
    /// # use soa_rs::{Soa, Soars};
    /// #[derive(Soars)]
    /// # #[soa_derive(Debug, PartialEq)]
    /// struct Foo(u8, u8);
    ///
    /// let mut soa = Soa::<Foo>::with_capacity(10);
    /// assert_eq!(soa.len(), 0);
    /// assert_eq!(soa.capacity(), 10);
    ///
    /// // These pushes do not reallocate...
    /// for i in 0..10 {
    ///     soa.push(Foo(i, i));
    /// }
    /// assert_eq!(soa.len(), 10);
    /// assert_eq!(soa.capacity(), 10);
    ///
    /// // ...but this one does
    /// soa.push(Foo(11, 11));
    /// assert_eq!(soa.len(), 11);
    /// assert_eq!(soa.capacity(), 20);
    ///
    /// #[derive(Soars, Copy, Clone)]
    /// # #[soa_derive(Debug, PartialEq)]
    /// struct Bar;
    ///
    /// // A SOA of a zero-sized type always over-allocates
    /// let soa = Soa::<Bar>::with_capacity(10);
    /// assert_eq!(soa.capacity(), usize::MAX);
    /// ```
    pub fn with_capacity(capacity: usize) -> Self {
        match capacity {
            0 => Self::new(),
            capacity => {
                if size_of::<T>() == 0 {
                    Self {
                        cap: usize::MAX,
                        slice: Slice::empty(),
                        len: 0,
                    }
                } else {
                    Self {
                        cap: capacity,
                        // SAFETY:
                        // - T is nonzero sized
                        // - capacity is nonzero
                        slice: Slice::with_raw(unsafe { T::Raw::alloc(capacity) }),
                        len: 0,
                    }
                }
            }
        }
    }

    /// Constructs a new `Soa<T>` with the given first element.
    ///
    /// This is mainly useful to get around type inference limitations in some
    /// situations, namely macros. Type inference can struggle sometimes due to
    /// dereferencing to an associated type of `T`, which causes Rust to get
    /// confused about whether, for example, `push`ing and element should coerce
    /// `self` to the argument's type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use soa_rs::{Soa, Soars, soa};
    /// # #[derive(Soars, Debug, PartialEq)]
    /// # #[soa_derive(Debug, PartialEq)]
    /// # struct Foo(usize);
    /// let soa = Soa::with(Foo(10));
    /// assert_eq!(soa, soa![Foo(10)]);
    /// ```
    pub fn with(element: T) -> Self {
        let mut out = Self::new();
        out.push(element);
        out
    }

    /// Returns the total number of elements the container can hold without
    /// reallocating.
    ///
    /// # Examples
    ///
    /// ```
    /// # use soa_rs::{Soa, Soars};
    /// # #[derive(Soars)]
    /// # #[soa_derive(Debug, PartialEq)]
    /// # struct Foo(usize);
    /// let mut soa = Soa::<Foo>::new();
    /// for i in 0..42 {
    ///     assert!(soa.capacity() >= i);
    ///     soa.push(Foo(i));
    /// }
    /// ```
    pub fn capacity(&self) -> usize {
        self.cap
    }

    /// Decomposes a `Soa<T>` into its raw components.
    ///
    /// Returns the raw pointer to the underlying data, the length of the vector (in
    /// elements), and the allocated capacity of the data (in elements). These
    /// are the same arguments in the same order as the arguments to
    /// [`Soa::from_raw_parts`].
    ///
    /// After calling this function, the caller is responsible for the memory
    /// previously managed by the `Soa`. The only way to do this is to convert the
    /// raw pointer, length, and capacity back into a Vec with the
    /// [`Soa::from_raw_parts`] function, allowing the destructor to perform the cleanup.
    ///
    /// # Examples
    ///
    /// ```
    /// # use soa_rs::{Soa, Soars, soa};
    /// # #[derive(Soars, Debug, PartialEq)]
    /// # #[soa_derive(Debug, PartialEq)]
    /// # struct Foo(usize);
    /// let soa = soa![Foo(1), Foo(2)];
    /// let (ptr, len, cap) = soa.into_raw_parts();
    /// let rebuilt = unsafe { Soa::<Foo>::from_raw_parts(ptr, len, cap) };
    /// assert_eq!(rebuilt, soa![Foo(1), Foo(2)]);
    /// ```
    pub fn into_raw_parts(self) -> (NonNull<u8>, usize, usize) {
        let me = ManuallyDrop::new(self);
        (me.raw().into_parts(), me.len, me.cap)
    }

    /// Creates a `Soa<T>` from a pointer, a length, and a capacity.
    ///
    /// # Safety
    ///
    /// This is highly unsafe due to the number of invariants that aren't
    /// checked. Given that many of these invariants are private implementation
    /// details of [`SoaRaw`], it is better not to uphold them manually. Rather,
    /// it only valid to call this method with the output of a previous call to
    /// [`Soa::into_raw_parts`].
    pub unsafe fn from_raw_parts(ptr: NonNull<u8>, length: usize, capacity: usize) -> Self {
        let raw = unsafe { T::Raw::from_parts(ptr, capacity) };
        Self {
            cap: capacity,
            slice: Slice::with_raw(raw),
            len: length,
        }
    }

    /// Appends an element to the back of a collection.
    ///
    /// # Examples
    ///
    /// ```
    /// # use soa_rs::{Soa, Soars, soa};
    /// # #[derive(Soars, Debug, PartialEq)]
    /// # #[soa_derive(Debug, PartialEq)]
    /// # struct Foo(usize);
    /// let mut soa = soa![Foo(1), Foo(2)];
    /// soa.push(Foo(3));
    /// assert_eq!(soa, soa![Foo(1), Foo(2), Foo(3)]);
    /// ```
    pub fn push(&mut self, element: T) {
        self.maybe_grow();
        // SAFETY: After maybe_grow, the allocated capacity is greater than len
        unsafe {
            self.raw().offset(self.len).set(element);
        }
        self.len += 1;
    }

    /// Removes the last element from a vector and returns it, or [`None`] if it
    /// is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// # use soa_rs::{Soa, Soars, soa};
    /// # #[derive(Soars, Debug, PartialEq)]
    /// # #[soa_derive(Debug, PartialEq)]
    /// # struct Foo(usize);
    /// let mut soa = soa![Foo(1), Foo(2), Foo(3)];
    /// assert_eq!(soa.pop(), Some(Foo(3)));
    /// assert_eq!(soa, soa![Foo(1), Foo(2)]);
    /// ```
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            // SAFETY: len points to at least one initialized item
            Some(unsafe { self.raw().offset(self.len).get() })
        }
    }

    /// Inserts an element at position `index`, shifting all elements after it
    /// to the right.
    ///
    /// # Panics
    ///
    /// Panics if `index > len`
    ///
    /// # Examples
    ///
    /// ```
    /// # use soa_rs::{Soa, Soars, soa};
    /// # #[derive(Soars, Debug, PartialEq)]
    /// # #[soa_derive(Debug, PartialEq)]
    /// # struct Foo(usize);
    /// let mut soa = soa![Foo(1), Foo(2), Foo(3)];
    /// soa.insert(1, Foo(4));
    /// assert_eq!(soa, soa![Foo(1), Foo(4), Foo(2), Foo(3)]);
    /// soa.insert(4, Foo(5));
    /// assert_eq!(soa, soa![Foo(1), Foo(4), Foo(2), Foo(3), Foo(5)]);
    /// ```
    pub fn insert(&mut self, index: usize, element: T) {
        assert!(index <= self.len, "index out of bounds");
        self.maybe_grow();
        // SAFETY: After the bounds check and maybe_grow, index is an
        // initialized item and index+1 is allocated
        unsafe {
            let ith = self.raw().offset(index);
            ith.copy_to(ith.offset(1), self.len - index);
            ith.set(element);
        }
        self.len += 1;
    }

    /// Removes and returns the element at position index within the vector,
    /// shifting all elements after it to the left.
    ///
    /// # Examples
    ///
    /// ```
    /// # use soa_rs::{Soa, Soars, soa};
    /// # #[derive(Soars, Debug, PartialEq)]
    /// # #[soa_derive(Debug, PartialEq)]
    /// # struct Foo(usize);
    /// let mut soa = soa![Foo(1), Foo(2), Foo(3)];
    /// assert_eq!(soa.remove(1), Foo(2));
    /// assert_eq!(soa, soa![Foo(1), Foo(3)])
    /// ```
    pub fn remove(&mut self, index: usize) -> T {
        assert!(index < self.len, "index out of bounds");
        self.len -= 1;
        // SAFETY: After the bounds check, we know ith item is initialized
        let ith = unsafe { self.raw().offset(index) };
        let out = unsafe { ith.get() };
        // SAFETY: There are len-index initialized elements to shift back
        unsafe {
            ith.offset(1).copy_to(ith, self.len - index);
        }
        out
    }

    /// Reserves capacity for at least additional more elements to be inserted
    /// in the given `Soa<T>`. The collection may reserve more space to
    /// speculatively avoid frequent reallocations. After calling reserve,
    /// capacity will be greater than or equal to `self.len() + additional`.
    /// Does nothing if capacity is already sufficient.
    ///
    /// # Examples
    ///
    /// ```
    /// # use soa_rs::{Soa, Soars, soa};
    /// # #[derive(Soars, Debug, PartialEq)]
    /// # #[soa_derive(Debug, PartialEq)]
    /// # struct Foo(usize);
    /// let mut soa = soa![Foo(1)];
    /// soa.reserve(10);
    /// assert!(soa.capacity() >= 11);
    /// ```
    pub fn reserve(&mut self, additional: usize) {
        let new_len = self.len + additional;
        if new_len > self.cap {
            let new_cap = new_len
                // Ensure exponential growth
                .max(self.cap * 2)
                .max(Self::SMALL_CAPACITY);
            self.grow(new_cap);
        }
    }

    /// Reserves the minimum capacity for at least additional more elements to
    /// be inserted in the given `Soa<T>`. Unlike [`Soa::reserve`], this will
    /// not deliberately over-allocate to speculatively avoid frequent
    /// allocations. After calling `reserve_exact`, capacity will be equal to
    /// self.len() + additional, or else `usize::MAX` if `T` is zero-sized. Does
    /// nothing if the capacity is already sufficient.
    ///
    /// # Examples
    ///
    /// ```
    /// # use soa_rs::{Soa, Soars, soa};
    /// # #[derive(Soars, Debug, PartialEq)]
    /// # #[soa_derive(Debug, PartialEq)]
    /// # struct Foo(usize);
    /// let mut soa = soa![Foo(1)];
    /// soa.reserve_exact(10);
    /// assert!(soa.capacity() == 11);
    /// ```
    pub fn reserve_exact(&mut self, additional: usize) {
        let new_len = additional + self.len;
        if new_len > self.cap {
            self.grow(new_len);
        }
    }

    /// Shrinks the capacity of the container as much as possible.
    ///
    /// # Examples
    ///
    /// ```
    /// # use soa_rs::{Soa, Soars, soa};
    /// # #[derive(Soars, Debug, PartialEq)]
    /// # #[soa_derive(Debug, PartialEq)]
    /// # struct Foo(usize);
    /// let mut soa = Soa::<Foo>::with_capacity(10);
    /// soa.extend([Foo(1), Foo(2), Foo(3)]);
    /// assert_eq!(soa.capacity(), 10);
    /// soa.shrink_to_fit();
    /// assert_eq!(soa.capacity(), 3);
    /// ```
    pub fn shrink_to_fit(&mut self) {
        self.shrink(self.len);
    }

    /// Shrinks the capacity of the vector with a lower bound.
    ///
    /// The capacity will remain at least as large as both the length and the
    /// supplied value. If the current capacity is less than the lower limit,
    /// this is a no-op.
    ///
    /// # Examples
    ///
    /// ```
    /// # use soa_rs::{Soa, Soars, soa};
    /// # #[derive(Soars, Debug, PartialEq)]
    /// # #[soa_derive(Debug, PartialEq)]
    /// # struct Foo(usize);
    /// let mut soa = Soa::<Foo>::with_capacity(10);
    /// soa.extend([Foo(1), Foo(2), Foo(3)]);
    /// assert_eq!(soa.capacity(), 10);
    /// soa.shrink_to(4);
    /// assert_eq!(soa.capacity(), 4);
    /// soa.shrink_to(0);
    /// assert_eq!(soa.capacity(), 3);
    pub fn shrink_to(&mut self, min_capacity: usize) {
        let new_cap = self.len.max(min_capacity);
        if new_cap < self.cap {
            self.shrink(new_cap);
        }
    }

    /// Shortens the vector, keeping the first len elements and dropping the rest.
    ///
    /// If len is greater or equal to the vector’s current length, this has no
    /// effect. Note that this method has no effect on the allocated capacity of
    /// the vector.
    ///
    /// # Examples
    ///
    /// Truncating a five-element SOA to two elements:
    /// ```
    /// # use soa_rs::{Soa, Soars, soa};
    /// # #[derive(Soars, Debug, PartialEq)]
    /// # #[soa_derive(Debug, PartialEq)]
    /// # struct Foo(usize);
    /// let mut soa = soa![Foo(1), Foo(2), Foo(3), Foo(4), Foo(5)];
    /// soa.truncate(2);
    /// assert_eq!(soa, soa![Foo(1), Foo(2)]);
    /// ```
    ///
    /// No truncation occurs when `len` is greater than the SOA's current
    /// length:
    /// ```
    /// # use soa_rs::{Soa, Soars, soa};
    /// # #[derive(Soars, Debug, PartialEq)]
    /// # #[soa_derive(Debug, PartialEq)]
    /// # struct Foo(usize);
    /// let mut soa = soa![Foo(1), Foo(2), Foo(3)];
    /// soa.truncate(8);
    /// assert_eq!(soa, soa![Foo(1), Foo(2), Foo(3)]);
    /// ```
    ///
    /// Truncating with `len == 0` is equivalent to [`Soa::clear`].
    /// ```
    /// # use soa_rs::{Soa, Soars, soa};
    /// # #[derive(Soars, Debug, PartialEq)]
    /// # #[soa_derive(Debug, PartialEq)]
    /// # struct Foo(usize);
    /// let mut soa = soa![Foo(1), Foo(2), Foo(3)];
    /// soa.truncate(0);
    /// assert_eq!(soa, soa![]);
    /// ```
    pub fn truncate(&mut self, len: usize) {
        if len >= self.len {
            return;
        }
        if needs_drop::<T>() {
            for i in len..self.len {
                // SAFETY: i < self.len, element is initialized; moved out then dropped.
                drop(unsafe { self.raw().offset(i).get() });
            }
        }
        self.len = len;
    }

    /// Removes an element from the vector and returns it.
    ///
    /// The removed element is replaced by the last element of the vector. This
    /// does not preserve ordering, but is O(1). If you need to preserve the
    /// element order, use remove instead.
    ///
    /// # Panics
    ///
    /// Panics if index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// # use soa_rs::{Soa, Soars, soa};
    /// # #[derive(Soars, Debug, PartialEq)]
    /// # #[soa_derive(Debug, PartialEq)]
    /// # struct Foo(usize);
    /// let mut soa = soa![Foo(0), Foo(1), Foo(2), Foo(3)];
    ///
    /// assert_eq!(soa.swap_remove(1), Foo(1));
    /// assert_eq!(soa, soa![Foo(0), Foo(3), Foo(2)]);
    ///
    /// assert_eq!(soa.swap_remove(0), Foo(0));
    /// assert_eq!(soa, soa![Foo(2), Foo(3)])
    /// ```
    pub fn swap_remove(&mut self, index: usize) -> T {
        if index >= self.len {
            panic!("index out of bounds")
        }
        self.len -= 1;
        // SAFETY: index and len-1 are initialized elements
        let to_remove = unsafe { self.raw().offset(index) };
        let last = unsafe { self.raw().offset(self.len) };
        let out = unsafe { to_remove.get() };
        unsafe {
            last.copy_to(to_remove, 1);
        }
        out
    }

    /// Moves all the elements of other into self, leaving other empty.
    ///
    /// # Examples
    ///
    /// ```
    /// # use soa_rs::{Soa, Soars, soa};
    /// # #[derive(Soars, Debug, PartialEq)]
    /// # #[soa_derive(Debug, PartialEq)]
    /// # struct Foo(usize);
    /// let mut soa1  = soa![Foo(1), Foo(2), Foo(3)];
    /// let mut soa2 = soa![Foo(4), Foo(5), Foo(6)];
    /// soa1.append(&mut soa2);
    /// assert_eq!(soa1, soa![Foo(1), Foo(2), Foo(3), Foo(4), Foo(5), Foo(6)]);
    /// assert_eq!(soa2, soa![]);
    /// ```
    pub fn append(&mut self, other: &mut Self) {
        if other.is_empty() {
            return;
        }
        self.reserve(other.len);
        // SAFETY:
        // - self has capacity for self.len + other.len elements after reserve.
        // - other.raw() points to other.len initialized elements.
        // - dst = self.raw().offset(self.len) is within the allocated region.
        // - src and dst are separate allocations; no overlap.
        unsafe {
            other.raw().copy_to(self.raw().offset(self.len), other.len);
        }
        self.len += other.len;
        // Elements moved to self; set other.len = 0 to prevent double-drop.
        // other retains its allocation (mirrors Vec::append behaviour).
        other.len = 0;
    }

    /// Clears the vector, removing all values.
    ///
    /// Note that this method has no effect on the allocated capacity of the
    /// vector.
    ///
    /// # Examples
    ///
    /// ```
    /// # use soa_rs::{Soa, Soars, soa};
    /// # #[derive(Soars, Debug, PartialEq)]
    /// # #[soa_derive(Debug, PartialEq)]
    /// # struct Foo(usize);
    /// let mut soa = soa![Foo(1), Foo(2)];
    /// soa.clear();
    /// assert!(soa.is_empty());
    /// ```
    pub fn clear(&mut self) {
        while self.pop().is_some() {}
    }

    /// Retains only the elements specified by the predicate.
    ///
    /// Removes all elements `e` for which `f(e)` returns `false`. This method
    /// operates in place, visiting each element exactly once in the original
    /// order, and preserves the order of the retained elements.
    ///
    /// # Examples
    ///
    /// ```
    /// # use soa_rs::{Soa, Soars, soa};
    /// # #[derive(Soars, Debug, PartialEq)]
    /// # #[soa_derive(Debug, PartialEq)]
    /// # struct Foo(u8);
    /// let mut soa: Soa<_> = [Foo(0), Foo(1), Foo(2), Foo(3)].into();
    /// soa.retain(|r| r.0 % 2 == 0);
    /// assert_eq!(soa, soa![Foo(0), Foo(2)]);
    /// ```
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(T::Ref<'_>) -> bool,
    {
        let mut write = 0usize;
        let mut read = 0usize;
        while read < self.len {
            // SAFETY: read < self.len, element is initialized.
            let keep = unsafe { f(self.raw().offset(read).get_ref()) };
            if keep {
                if write != read {
                    // SAFETY: write < read, both < self.len; ranges non-overlapping.
                    unsafe {
                        self.raw().offset(read).copy_to(self.raw().offset(write), 1);
                    }
                }
                write += 1;
            } else {
                // SAFETY: read < self.len, element is initialized; move out and drop.
                drop(unsafe { self.raw().offset(read).get() });
            }
            read += 1;
        }
        self.len = write;
    }

    /// Retains only the elements specified by the predicate, passing mutable
    /// references to it.
    ///
    /// Like [`retain`], but allows the predicate to also mutate kept elements.
    ///
    /// # Examples
    ///
    /// ```
    /// # use soa_rs::{Soa, Soars, soa};
    /// # #[derive(Soars, Debug, PartialEq)]
    /// # #[soa_derive(Debug, PartialEq)]
    /// # struct Foo(u8);
    /// let mut soa: Soa<_> = [Foo(1), Foo(2), Foo(3), Foo(4)].into();
    /// soa.retain_mut(|mut r| { *r.0 *= 10; *r.0 < 30 });
    /// assert_eq!(soa, soa![Foo(10), Foo(20)]);
    /// ```
    ///
    /// [`retain`]: Soa::retain
    pub fn retain_mut<F>(&mut self, mut f: F)
    where
        F: FnMut(T::RefMut<'_>) -> bool,
    {
        let mut write = 0usize;
        let mut read = 0usize;
        while read < self.len {
            // SAFETY: read < self.len, element is initialized.
            let keep = unsafe { f(self.raw().offset(read).get_mut()) };
            if keep {
                if write != read {
                    // SAFETY: write < read, both < self.len; ranges non-overlapping.
                    unsafe {
                        self.raw().offset(read).copy_to(self.raw().offset(write), 1);
                    }
                }
                write += 1;
            } else {
                drop(unsafe { self.raw().offset(read).get() });
            }
            read += 1;
        }
        self.len = write;
    }

    /// Removes all but the first of consecutive elements for which
    /// `same_bucket` returns `true`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use soa_rs::{Soa, Soars, soa};
    /// # #[derive(Soars, Debug, PartialEq)]
    /// # #[soa_derive(Debug, PartialEq)]
    /// # struct Foo(u8);
    /// let mut soa: Soa<_> = [Foo(1), Foo(2), Foo(2), Foo(3)].into();
    /// soa.dedup_by(|a, b| a == b);
    /// assert_eq!(soa, soa![Foo(1), Foo(2), Foo(3)]);
    /// ```
    pub fn dedup_by<F>(&mut self, mut same_bucket: F)
    where
        F: FnMut(T::Ref<'_>, T::Ref<'_>) -> bool,
    {
        if self.len <= 1 {
            return;
        }
        let mut write = 1usize;
        for read in 1..self.len {
            // SAFETY: read < self.len; write - 1 < write <= read < self.len.
            let is_dup = unsafe {
                same_bucket(
                    self.raw().offset(read).get_ref(),
                    self.raw().offset(write - 1).get_ref(),
                )
            };
            if is_dup {
                // SAFETY: read < self.len, element is initialized.
                drop(unsafe { self.raw().offset(read).get() });
            } else {
                if write != read {
                    // SAFETY: write < read < self.len; non-overlapping.
                    unsafe {
                        self.raw().offset(read).copy_to(self.raw().offset(write), 1);
                    }
                }
                write += 1;
            }
        }
        self.len = write;
    }

    /// Removes consecutive repeated elements.
    ///
    /// If the vector is not sorted, this removes only consecutive duplicates.
    /// Use [`sort`] first if you want to deduplicate globally.
    ///
    /// # Examples
    ///
    /// ```
    /// # use soa_rs::{Soa, Soars, soa};
    /// # #[derive(Soars, Debug, PartialEq)]
    /// # #[soa_derive(Debug, PartialEq)]
    /// # struct Foo(u8);
    /// let mut soa: Soa<_> = [Foo(1), Foo(1), Foo(2), Foo(3), Foo(3)].into();
    /// soa.dedup();
    /// assert_eq!(soa, soa![Foo(1), Foo(2), Foo(3)]);
    /// ```
    ///
    /// [`sort`]: Slice::sort
    pub fn dedup(&mut self)
    where
        for<'a> T::Ref<'a>: PartialEq,
    {
        self.dedup_by(|a, b| {
            // SAFETY: Both `a` and `b` reference live, initialized data that
            // remains valid for the duration of this comparison. Extending one
            // ref to match the other's lifetime is sound here.
            let b: T::Ref<'_> = unsafe { core::mem::transmute(b) };
            a == b
        });
    }

    /// Removes consecutive elements that map to the same key.
    ///
    /// # Examples
    ///
    /// ```
    /// # use soa_rs::{Soa, Soars, soa};
    /// # #[derive(Soars, Debug, PartialEq)]
    /// # #[soa_derive(Debug, PartialEq)]
    /// # struct Foo(u8);
    /// let mut soa: Soa<_> = [Foo(10), Foo(11), Foo(20), Foo(30), Foo(31)].into();
    /// soa.dedup_by_key(|r| *r.0 / 10);
    /// assert_eq!(soa, soa![Foo(10), Foo(20), Foo(30)]);
    /// ```
    pub fn dedup_by_key<K, F>(&mut self, mut key: F)
    where
        K: PartialEq,
        F: FnMut(T::Ref<'_>) -> K,
    {
        self.dedup_by(|a, b| {
            // SAFETY: `a` and `b` are both live for the call; transmuting to a
            // single lifetime is sound since we only read through them here.
            let b: T::Ref<'_> = unsafe { core::mem::transmute(b) };
            key(a) == key(b)
        });
    }

    /// Removes and yields elements from the given range, shifting the tail left.
    ///
    /// The returned iterator yields the removed elements in order. If the
    /// iterator is dropped before being fully consumed, the remaining elements
    /// of the range are dropped and the tail is shifted.
    ///
    /// # Panics
    ///
    /// Panics if the range start is greater than the end, or if the range end
    /// is greater than the length.
    ///
    /// # Examples
    ///
    /// ```
    /// # use soa_rs::{Soa, Soars, soa};
    /// # #[derive(Soars, Debug, PartialEq)]
    /// # #[soa_derive(Debug, PartialEq)]
    /// # struct Foo(usize);
    /// let mut soa: Soa<_> = [Foo(1), Foo(2), Foo(3), Foo(4), Foo(5)].into();
    /// let drained: Vec<_> = soa.drain(1..4).collect();
    /// assert_eq!(drained, vec![Foo(2), Foo(3), Foo(4)]);
    /// assert_eq!(soa, soa![Foo(1), Foo(5)]);
    /// ```
    pub fn drain<R>(&mut self, range: R) -> Drain<'_, T>
    where
        R: core::ops::RangeBounds<usize>,
    {
        use core::ops::Bound;
        let start = match range.start_bound() {
            Bound::Included(&s) => s,
            Bound::Excluded(&s) => s + 1,
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(&e) => e + 1,
            Bound::Excluded(&e) => e,
            Bound::Unbounded => self.len,
        };
        assert!(start <= end, "drain range start > end");
        assert!(end <= self.len, "drain range end out of bounds");

        let old_len = self.len;
        // Set len to `start` so that if Drain is leaked, the Soa exposes only
        // the prefix [0..start] and the tail [end..old_len] is leaked too.
        self.len = start;

        Drain {
            soa: core::ptr::NonNull::from(&mut *self),
            range_start: start,
            range_end: end,
            tail_start: end,
            tail_len: old_len - end,
            read: start,
            _marker: core::marker::PhantomData,
        }
    }

    /// Splits the collection into two at the given index.
    ///
    /// Returns a newly allocated `Soa` containing the elements in the range
    /// `[at, len)`. After the call, the original will contain elements `[0, at)`.
    ///
    /// # Panics
    ///
    /// Panics if `at > len`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use soa_rs::{Soa, Soars, soa};
    /// # #[derive(Soars, Debug, PartialEq)]
    /// # #[soa_derive(Debug, PartialEq)]
    /// # struct Foo(usize);
    /// let mut soa: Soa<_> = [Foo(1), Foo(2), Foo(3), Foo(4)].into();
    /// let tail = soa.split_off(2);
    /// assert_eq!(soa, soa![Foo(1), Foo(2)]);
    /// assert_eq!(tail, soa![Foo(3), Foo(4)]);
    /// ```
    pub fn split_off(&mut self, at: usize) -> Self {
        assert!(at <= self.len, "index out of bounds");
        let tail_len = self.len - at;
        let mut other = Self::with_capacity(tail_len);
        if tail_len > 0 {
            // SAFETY:
            // - self.raw().offset(at) points to `tail_len` initialized elements.
            // - other.raw() has capacity for `tail_len` elements.
            // - The two allocations do not overlap.
            unsafe {
                self.raw()
                    .offset(at)
                    .copy_to(other.raw().offset(0), tail_len);
            }
            other.len = tail_len;
        }
        self.len = at;
        other
    }

    /// Resizes the `Soa` in-place so that `len` equals `new_len`.
    ///
    /// If `new_len` is greater than `len`, the `Soa` is extended by calling
    /// `f` for each new element. If `new_len` is less than `len`, the `Soa`
    /// is truncated.
    ///
    /// # Examples
    ///
    /// ```
    /// # use soa_rs::{Soa, Soars, soa};
    /// # #[derive(Soars, Debug, PartialEq)]
    /// # #[soa_derive(Debug, PartialEq)]
    /// # struct Foo(u8);
    /// let mut soa: Soa<Foo> = Soa::new();
    /// let mut n = 0u8;
    /// soa.resize_with(4, || { let v = Foo(n); n += 1; v });
    /// assert_eq!(soa, soa![Foo(0), Foo(1), Foo(2), Foo(3)]);
    /// ```
    pub fn resize_with<F>(&mut self, new_len: usize, mut f: F)
    where
        F: FnMut() -> T,
    {
        match new_len.cmp(&self.len) {
            core::cmp::Ordering::Greater => {
                self.reserve(new_len - self.len);
                while self.len < new_len {
                    self.push(f());
                }
            }
            core::cmp::Ordering::Less => {
                self.truncate(new_len);
            }
            core::cmp::Ordering::Equal => {}
        }
    }

    /// Resizes the `Soa` in-place so that `len` equals `new_len`, using `value`
    /// to fill new elements.
    ///
    /// # Examples
    ///
    /// ```
    /// # use soa_rs::{Soa, Soars, soa};
    /// # #[derive(Soars, Debug, PartialEq, Clone)]
    /// # #[soa_derive(Debug, PartialEq)]
    /// # struct Foo(u8);
    /// let mut soa: Soa<_> = [Foo(1), Foo(2)].into();
    /// soa.resize(5, Foo(0));
    /// assert_eq!(soa, soa![Foo(1), Foo(2), Foo(0), Foo(0), Foo(0)]);
    /// ```
    pub fn resize(&mut self, new_len: usize, value: T)
    where
        T: Clone,
    {
        self.resize_with(new_len, || value.clone());
    }

    /// Returns a `Vec<usize>` of indices `0..self.len()` sorted by `compare`.
    ///
    /// Data in the `Soa` is never moved. Iterate over the result with
    /// `soa.idx(i)` to visit elements in sorted order.
    ///
    /// # Examples
    ///
    /// ```
    /// # use soa_rs::{soa, Soars};
    /// # #[derive(Soars, Debug, PartialEq, Copy, Clone)]
    /// # #[soa_derive(Debug, PartialEq)]
    /// # struct Foo(u8);
    /// let soa = soa![Foo(3), Foo(1), Foo(2)];
    /// let order = soa.sort_indices_by(|a, b| a.0.cmp(b.0));
    /// let sorted: Vec<u8> = order.iter().map(|&i| *soa.idx(i).0).collect();
    /// assert_eq!(sorted, [1, 2, 3]);
    /// ```
    pub fn sort_indices_by<F>(&self, mut compare: F) -> crate::Vec<usize>
    where
        F: FnMut(T::Ref<'_>, T::Ref<'_>) -> core::cmp::Ordering,
    {
        let mut indices: crate::Vec<usize> = (0..self.len).collect();
        indices.sort_by(|&a, &b| {
            // SAFETY: a and b are in 0..self.len
            let ra = unsafe { self.raw().offset(a).get_ref() };
            let rb = unsafe { self.raw().offset(b).get_ref() };
            compare(ra, rb)
        });
        indices
    }

    /// Returns a `Vec<usize>` of indices `0..self.len()` sorted by `key`.
    ///
    /// Data in the `Soa` is never moved. Iterate over the result with
    /// `soa.idx(i)` to visit elements in sorted order.
    ///
    /// # Examples
    ///
    /// ```
    /// # use soa_rs::{soa, Soars};
    /// # #[derive(Soars, Debug, PartialEq, Copy, Clone)]
    /// # #[soa_derive(Debug, PartialEq)]
    /// # struct Foo(u8);
    /// let soa = soa![Foo(3), Foo(1), Foo(2)];
    /// let order = soa.sort_indices_by_key(|r| *r.0);
    /// let sorted: Vec<u8> = order.iter().map(|&i| *soa.idx(i).0).collect();
    /// assert_eq!(sorted, [1, 2, 3]);
    /// ```
    pub fn sort_indices_by_key<K, F>(&self, mut key: F) -> crate::Vec<usize>
    where
        K: Ord,
        F: FnMut(T::Ref<'_>) -> K,
    {
        let mut indices: crate::Vec<usize> = (0..self.len).collect();
        indices.sort_by_key(|&i| {
            // SAFETY: i is in 0..self.len
            let r = unsafe { self.raw().offset(i).get_ref() };
            key(r)
        });
        indices
    }

    /// Grows the allocated capacity if `len == cap`.
    fn maybe_grow(&mut self) {
        if self.len < self.cap {
            return;
        }
        let new_cap = match self.cap {
            0 => Self::SMALL_CAPACITY,
            old_cap => old_cap * 2,
        };
        self.grow(new_cap);
    }

    // Shrinks the allocated capacity.
    fn shrink(&mut self, new_cap: usize) {
        debug_assert!(new_cap <= self.cap);
        if self.cap == 0 || new_cap == self.cap || size_of::<T>() == 0 {
            return;
        }

        if new_cap == 0 {
            debug_assert!(self.cap > 0);
            // SAFETY: We asserted the preconditions
            unsafe {
                self.raw().dealloc(self.cap);
            }
            self.raw = T::Raw::dangling();
        } else {
            debug_assert!(new_cap < self.cap);
            debug_assert!(self.len <= new_cap);
            // SAFETY: We asserted the preconditions
            unsafe {
                self.raw = self.raw().realloc_shrink(self.cap, new_cap, self.len);
            }
        }

        self.cap = new_cap;
    }

    /// Grows the allocated capacity.
    fn grow(&mut self, new_cap: usize) {
        debug_assert!(size_of::<T>() > 0);
        debug_assert!(new_cap > self.cap);

        if self.cap == 0 {
            debug_assert!(new_cap > 0);
            // SAFETY: We asserted the preconditions
            self.raw = unsafe { T::Raw::alloc(new_cap) };
        } else {
            debug_assert!(self.len <= self.cap);
            // SAFETY: We asserted the preconditions
            unsafe {
                self.raw = self.raw().realloc_grow(self.cap, new_cap, self.len);
            }
        }

        self.cap = new_cap;
    }
}

impl<T> Drop for Soa<T>
where
    T: Soars,
{
    fn drop(&mut self) {
        if needs_drop::<T>() {
            while self.pop().is_some() {}
        }

        if size_of::<T>() > 0 && self.cap > 0 {
            // SAFETY: We asserted the preconditions
            unsafe {
                self.raw().dealloc(self.cap);
            }
        }
    }
}

impl<T> IntoIterator for Soa<T>
where
    T: Soars,
{
    type Item = T;

    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        let soa = ManuallyDrop::new(self);
        IntoIter {
            iter_raw: IterRaw {
                slice: soa.slice,
                len: soa.len,
                adapter: PhantomData,
            },
            ptr: soa.raw().into_parts(),
            cap: soa.cap,
        }
    }
}

impl<'a, T> IntoIterator for &'a Soa<T>
where
    T: Soars,
{
    type Item = T::Ref<'a>;

    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.deref().into_iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Soa<T>
where
    T: Soars,
{
    type Item = T::RefMut<'a>;

    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.deref_mut().into_iter()
    }
}

impl<T> Clone for Soa<T>
where
    T: SoaClone,
{
    fn clone(&self) -> Self {
        self.iter().map(SoaClone::soa_clone).collect()
    }

    fn clone_from(&mut self, source: &Self) {
        self.clear();
        self.extend(source.iter().map(SoaClone::soa_clone));
    }
}

impl<T> Extend<T> for Soa<T>
where
    T: Soars,
{
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        let iter = iter.into_iter();
        let (lower, _) = iter.size_hint();
        self.reserve(lower);
        for item in iter {
            self.push(item);
        }
    }
}

impl<T> FromIterator<T> for Soa<T>
where
    T: Soars,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (hint_min, hint_max) = iter.size_hint();
        let cap = hint_max.unwrap_or(hint_min);
        let mut out = Self::with_capacity(cap);
        for item in iter {
            out.push(item);
        }
        out
    }
}

impl<T, const N: usize> From<[T; N]> for Soa<T>
where
    T: Soars,
{
    /// Allocate a `Soa<T>` and move `value`'s items into it.
    fn from(value: [T; N]) -> Self {
        value.into_iter().collect()
    }
}

impl<T, const N: usize> From<&[T; N]> for Soa<T>
where
    T: Soars + Clone,
{
    /// Allocate a `Soa<T>` and fill it by cloning `value`'s items.
    fn from(value: &[T; N]) -> Self {
        value.as_ref().into()
    }
}

impl<T, const N: usize> From<&mut [T; N]> for Soa<T>
where
    T: Soars + Clone,
{
    /// Allocate a `Soa<T>` and fill it by cloning `value`'s items.
    fn from(value: &mut [T; N]) -> Self {
        value.as_ref().into()
    }
}

impl<T> From<&[T]> for Soa<T>
where
    T: Soars + Clone,
{
    /// Allocate a `Soa<T>` and fill it by cloning `value`'s items.
    fn from(value: &[T]) -> Self {
        value.iter().cloned().collect()
    }
}

impl<T> From<&mut [T]> for Soa<T>
where
    T: Soars + Clone,
{
    /// Allocate a `Soa<T>` and fill it by cloning `value`'s items.
    fn from(value: &mut [T]) -> Self {
        value.as_ref().into()
    }
}

impl<T> From<Soa<T>> for Vec<T>
where
    T: Soars,
{
    /// Allocate a `Vec<T>` and fill it by moving the contents of `value`.
    fn from(value: Soa<T>) -> Self {
        value.into_iter().collect()
    }
}

impl<T> Debug for Soa<T>
where
    T: Soars,
    for<'a> T::Ref<'a>: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.as_slice().fmt(f)
    }
}

impl<T> PartialOrd for Soa<T>
where
    T: Soars,
    for<'a> T::Ref<'a>: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.as_slice().partial_cmp(&other.as_slice())
    }
}

impl<T> Ord for Soa<T>
where
    T: Soars,
    for<'a> T::Ref<'a>: Ord,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_slice().cmp(&other.as_slice())
    }
}

impl<T> Hash for Soa<T>
where
    T: Soars,
    for<'a> T::Ref<'a>: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state)
    }
}

impl<T> Default for Soa<T>
where
    T: Soars,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> AsRef<Slice<T>> for Soa<T>
where
    T: Soars,
{
    fn as_ref(&self) -> &Slice<T> {
        // SAFETY:
        // - len is valid for the slice
        // - The lifetime is bound to self
        unsafe { self.slice.as_unsized(self.len) }
    }
}

impl<T> AsMut<Slice<T>> for Soa<T>
where
    T: Soars,
{
    fn as_mut(&mut self) -> &mut Slice<T> {
        // SAFETY:
        // - len is valid for the slice
        // - The lifetime is bound to self
        unsafe { self.slice.as_unsized_mut(self.len) }
    }
}

impl<T> AsRef<Self> for Soa<T>
where
    T: Soars,
{
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<T> AsMut<Self> for Soa<T>
where
    T: Soars,
{
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

impl<T> Deref for Soa<T>
where
    T: Soars,
{
    type Target = Slice<T>;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T> DerefMut for Soa<T>
where
    T: Soars,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

impl<T> Borrow<Slice<T>> for Soa<T>
where
    T: Soars,
{
    fn borrow(&self) -> &Slice<T> {
        self.as_ref()
    }
}

impl<T> BorrowMut<Slice<T>> for Soa<T>
where
    T: Soars,
{
    fn borrow_mut(&mut self) -> &mut Slice<T> {
        self.as_mut()
    }
}

impl<T, R> PartialEq<R> for Soa<T>
where
    T: Soars,
    R: AsSlice<Item = T> + ?Sized,
    for<'a> T::Ref<'a>: PartialEq,
{
    fn eq(&self, other: &R) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<T> Eq for Soa<T>
where
    T: Soars,
    for<'a> T::Ref<'a>: Eq,
{
}

impl<T> AsSlice for Soa<T>
where
    T: Soars,
{
    type Item = T;

    fn as_slice(&self) -> SliceRef<'_, Self::Item> {
        // SAFETY:
        // - len is valid for this slice
        // - The returned lifetime is bound to self
        unsafe { SliceRef::from_slice(self.slice, self.len) }
    }
}

impl<T> AsMutSlice for Soa<T>
where
    T: Soars,
{
    fn as_mut_slice(&mut self) -> crate::SliceMut<'_, Self::Item> {
        // SAFETY:
        // - len is valid for this slice
        // - The returned lifetime is bound to self
        unsafe { SliceMut::from_slice(self.slice, self.len) }
    }
}
