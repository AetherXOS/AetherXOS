use alloc::boxed::Box;
use core::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

/// RCU (Read-Copy-Update) with Epoch-Based Reclamation.
///
/// This implementation tracks active readers via an atomic counter.
/// Writers swap the data pointer but **must not** free the old allocation
/// until all readers that could have observed it have exited.
///
/// The `update()` method returns the old `Box<T>` to the caller, but
/// the caller **must** ensure no `RcuGuard` for the previous value is
/// still alive before dropping it. In practice this means:
///   - Deferring the drop to a quiescent-state callback, or
///   - Checking `active_readers() == 0` before dropping.
///
/// For kernel-internal usage where read sections are short and bounded
/// (e.g., config lookups), the reader-count approach is sufficient.

pub struct Rcu<T> {
    inner: AtomicPtr<T>,
    /// Number of RcuGuard instances currently alive.
    readers: AtomicUsize,
}

impl<T> Rcu<T> {
    pub fn new(data: T) -> Self {
        let boxed = Box::new(data);
        Self {
            inner: AtomicPtr::new(Box::into_raw(boxed)),
            readers: AtomicUsize::new(0),
        }
    }

    /// How many read guards are currently live.
    #[inline]
    pub fn active_readers(&self) -> usize {
        self.readers.load(Ordering::Acquire)
    }

    /// Reader: Returns a Guard that dereferences to `&T`.
    ///
    /// The guard increments the reader count on creation and decrements
    /// it on drop, preventing the writer from reclaiming data while any
    /// guard is alive.
    pub fn read(&self) -> RcuGuard<'_, T> {
        self.readers.fetch_add(1, Ordering::AcqRel);
        // Acquire ensures we see the latest pointer after the reader
        // count has been published.
        let ptr = self.inner.load(Ordering::Acquire);
        unsafe {
            RcuGuard {
                data: &*ptr,
                rcu: self,
            }
        }
    }

    /// Writer: Swaps the data pointer and returns the old `Box<T>`.
    ///
    /// **Safety contract**: the caller must not drop the returned box
    /// until `active_readers() == 0`, or until a grace period has
    /// elapsed ensuring all prior read-side critical sections completed.
    pub fn update(&self, new_data: T) -> Box<T> {
        let new_ptr = Box::into_raw(Box::new(new_data));
        let old_ptr = self.inner.swap(new_ptr, Ordering::AcqRel);
        unsafe { Box::from_raw(old_ptr) }
    }
}

pub struct RcuGuard<'a, T> {
    data: &'a T,
    rcu: &'a Rcu<T>,
}

impl<'a, T> core::ops::Deref for RcuGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.data
    }
}

impl<'a, T> Drop for RcuGuard<'a, T> {
    fn drop(&mut self) {
        self.rcu.readers.fetch_sub(1, Ordering::AcqRel);
    }
}

impl<T> Drop for Rcu<T> {
    fn drop(&mut self) {
        let ptr = self.inner.load(Ordering::Relaxed);
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}
