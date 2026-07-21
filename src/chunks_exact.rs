use crate::{Slice, SliceRef, SoaRaw, Soars};
use core::{iter::FusedIterator, marker::PhantomData};

/// An iterator over a [`Slice`] in (non-overlapping) chunks of `chunk_size`
/// elements.
///
/// When the slice len is not evenly divided by the chunk size, the last up to
/// `chunk_size-1` elements will be omitted but can be retrieved from the
/// [`remainder`] function from the iterator.
///
/// This struct is created by the [`chunks_exact`] method.
///
/// [`remainder`]: ChunksExact::remainder
/// [`chunks_exact`]: Slice::chunks_exact
pub struct ChunksExact<'a, T>
where
    T: 'a + Soars,
{
    /// Base raw pointer (start of the divisible region).
    base: <T as Soars>::Raw,
    /// Index of the next chunk to yield from the front.
    fwd_index: usize,
    /// Exclusive upper bound for chunks from the back.
    back_index: usize,
    chunk_size: usize,
    remainder: SliceRef<'a, T>,
}

impl<'a, T> ChunksExact<'a, T>
where
    T: Soars,
{
    pub(crate) fn new(slice: &'a Slice<T>, chunk_size: usize) -> Self {
        let len = slice.len();
        let rem_len = len % chunk_size;
        let fst_len = len - rem_len;
        let remainder = slice.idx(fst_len..);
        // SAFETY: Lifetime of self is bound to the passed slice.
        let base = unsafe { slice.as_sized() }.raw;
        Self {
            base,
            fwd_index: 0,
            back_index: fst_len / chunk_size,
            chunk_size,
            remainder,
        }
    }

    /// Returns the remainder of the original slice that has not been yielded by
    /// the iterator.
    pub fn remainder(&self) -> &Slice<T> {
        self.remainder.as_ref()
    }
}

impl<'a, T> Iterator for ChunksExact<'a, T>
where
    T: Soars,
{
    type Item = SliceRef<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.fwd_index >= self.back_index {
            return None;
        }
        // SAFETY: fwd_index < back_index, so offset is within the original allocation.
        let raw = unsafe { self.base.offset(self.fwd_index * self.chunk_size) };
        self.fwd_index += 1;
        Some(SliceRef {
            slice: Slice::with_raw(raw),
            len: self.chunk_size,
            marker: PhantomData,
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.back_index - self.fwd_index;
        (n, Some(n))
    }
}

impl<'a, T> DoubleEndedIterator for ChunksExact<'a, T>
where
    T: Soars,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.fwd_index >= self.back_index {
            return None;
        }
        self.back_index -= 1;
        // SAFETY: back_index was > fwd_index before decrement, offset is valid.
        let raw = unsafe { self.base.offset(self.back_index * self.chunk_size) };
        Some(SliceRef {
            slice: Slice::with_raw(raw),
            len: self.chunk_size,
            marker: PhantomData,
        })
    }
}

impl<T> ExactSizeIterator for ChunksExact<'_, T>
where
    T: Soars,
{
    fn len(&self) -> usize {
        self.back_index - self.fwd_index
    }
}

impl<T> FusedIterator for ChunksExact<'_, T> where T: Soars {}
