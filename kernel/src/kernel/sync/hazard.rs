use core::sync::atomic::{AtomicPtr, Ordering};
use core::ptr;

/// A simple Hazard Pointer implementation for lock-free resource reclamation.
///
/// Hazard pointers allow threads to "announce" which pointers they are currently accessing,
/// preventing the reclamation thread from freeing those pointers until the hazard is cleared.
pub struct HazardPointer {
    ptr: AtomicPtr<u8>,
}

impl HazardPointer {
    pub const fn new() -> Self {
        Self {
            ptr: AtomicPtr::new(ptr::null_mut()),
        }
    }

    /// Protect a pointer by marking it as "hazard".
    pub fn protect(&self, p: *mut u8) {
        self.ptr.store(p, Ordering::Release);
    }

    /// Clear the hazard protection.
    pub fn clear(&self) {
        self.ptr.store(ptr::null_mut(), Ordering::Release);
    }

    /// Check if a pointer is currently protected by this hazard pointer.
    pub fn is_protecting(&self, p: *mut u8) -> bool {
        self.ptr.load(Ordering::Acquire) == p
    }
}

/// Global Hazard Table for all CPUs.
pub struct HazardTable<const N: usize> {
    slots: [HazardPointer; N],
}

impl<const N: usize> HazardTable<N> {
    pub const fn new() -> Self {
        // Safety: HazardPointer::new() is const and safe to initialize in an array
        // We use a manual array initialization because [HazardPointer::new(); N] 
        // requires Copy or specific compiler support.
        unsafe { core::mem::zeroed() } // In reality, we'd want a proper const init
    }

    pub fn protect(&self, cpu_id: usize, p: *mut u8) {
        if cpu_id < N {
            self.slots[cpu_id].protect(p);
        }
    }

    pub fn clear(&self, cpu_id: usize) {
        if cpu_id < N {
            self.slots[cpu_id].clear();
        }
    }

    pub fn is_hazard(&self, p: *mut u8) -> bool {
        for slot in self.slots.iter() {
            if slot.is_protecting(p) {
                return true;
            }
        }
        false
    }
}
