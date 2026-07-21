use crate::{Soa, SoaRaw, Soars};
use core::iter::FusedIterator;
use core::ptr::NonNull;

/// A draining iterator for [`Soa`].
///
/// This struct is created by [`Soa::drain`]. See its documentation for more.
///
/// [`Soa`]: crate::Soa
pub struct Drain<'a, T: Soars> {
    /// Back-pointer to the parent `Soa`. Length was set to `range_start` when
    /// this was created; we restore it in `Drop`.
    pub(crate) soa: NonNull<Soa<T>>,
    /// First index of the drained range.
    pub(crate) range_start: usize,
    /// One-past-last index of the drained range.
    pub(crate) range_end: usize,
    /// First index of the tail (elements after the drained range).
    pub(crate) tail_start: usize,
    /// Number of elements in the tail.
    pub(crate) tail_len: usize,
    /// Current read cursor within the range.
    pub(crate) read: usize,
    pub(crate) _marker: core::marker::PhantomData<&'a mut Soa<T>>,
}

impl<T: Soars> Iterator for Drain<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.read >= self.range_end {
            return None;
        }
        // SAFETY: `read` is within the drained range, which holds initialized elements.
        let item = unsafe { self.soa.as_ref().raw().offset(self.read).get() };
        self.read += 1;
        Some(item)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.range_end - self.read;
        (remaining, Some(remaining))
    }
}

impl<T: Soars> ExactSizeIterator for Drain<'_, T> {}
impl<T: Soars> FusedIterator for Drain<'_, T> {}

impl<T: Soars> Drop for Drain<'_, T> {
    fn drop(&mut self) {
        // Drop any elements in [read..range_end] not yet yielded.
        for i in self.read..self.range_end {
            // SAFETY: elements in [read..range_end] are still initialized.
            drop(unsafe { self.soa.as_ref().raw().offset(i).get() });
        }

        // Shift tail [tail_start..tail_start+tail_len] left to close the gap.
        let soa = unsafe { self.soa.as_mut() };
        if self.tail_len > 0 {
            // SAFETY:
            // - tail_start = range_end <= old_len, tail elements are initialized.
            // - range_start < tail_start, so dst is before src; no overlap for
            //   a forward copy of tail_len elements.
            unsafe {
                soa.raw()
                    .offset(self.tail_start)
                    .copy_to(soa.raw().offset(self.range_start), self.tail_len);
            }
        }

        soa.len = self.range_start + self.tail_len;
    }
}
