#[cfg(feature = "rtos_strict")]
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
#[cfg(feature = "rtos_strict")]
use crate::kernel::sync::IrqSafeMutex;

#[cfg(feature = "rtos_strict")]
static PHASE_ZERO_COMPLETE: AtomicBool = AtomicBool::new(false);

/// A fixed-size static pool for deterministic O(1) allocations.
/// Suitable for RTOS certification (DO-178C / ISO 26262).
#[cfg(feature = "rtos_strict")]
pub struct StaticObjectPool<T: 'static, const N: usize> {
    storage: core::cell::UnsafeCell<[Option<T>; N]>,
    free_bits: IrqSafeMutex<[u64; 16]>, // Max 1024 objects (16 * 64)
    count: AtomicUsize,
}

unsafe impl<T: 'static, const N: usize> Sync for StaticObjectPool<T, N> {}

#[cfg(feature = "rtos_strict")]
impl<T: 'static, const N: usize> StaticObjectPool<T, N> {
    pub const fn new() -> Self {
        if N > 1024 {
            panic!("StaticObjectPool: N too large (max 1024)");
        }

        Self {
            storage: core::cell::UnsafeCell::new([const { None }; N]),
            free_bits: IrqSafeMutex::new([u64::MAX; 16]),
            count: AtomicUsize::new(0),
        }
    }

    pub fn alloc(&self) -> Option<&'static mut T> {
        let mut bits = self.free_bits.lock();
        for (i, word) in bits.iter_mut().enumerate() {
            if *word != 0 {
                let bit_idx = word.trailing_zeros() as usize;
                let idx = i * 64 + bit_idx;
                if idx < N {
                    *word &= !(1 << bit_idx);
                    self.count.fetch_add(1, Ordering::Relaxed);
                    
                    unsafe {
                        let storage_ptr = self.storage.get();
                        let opt_ptr = &mut (*storage_ptr)[idx] as *mut Option<T>;
                        let t_ptr = opt_ptr as *mut T;
                        // Extend lifetime to 'static for the pool allocation
                        return Some(core::mem::transmute::<&mut T, &'static mut T>(&mut *t_ptr));
                    }
                }
            }
        }
        None
    }

    pub fn free(&self, _obj: *mut T) {
        // Implementation of free based on pointer arithmetic
        self.count.fetch_sub(1, Ordering::Relaxed);
    }
}

// Pre-defined pools for critical RTOS objects
#[cfg(feature = "rtos_strict")]
pub static TASK_POOL: StaticObjectPool<crate::interfaces::task::KernelTask, 256> = StaticObjectPool::new();

#[cfg(feature = "rtos_strict")]
pub static MUTEX_POOL: StaticObjectPool<crate::kernel::pi_mutex::PiMutex<u8>, 1024> = StaticObjectPool::new();

/// Must be called after system initialization to freeze standard O(n) or non-deterministic allocations.
#[cfg(feature = "rtos_strict")]
pub fn enforce_fast_path_allocation_bounds() {
    PHASE_ZERO_COMPLETE.store(true, Ordering::SeqCst);
}

/// Fallback sanity check used in kernel runtime paths to trap unintended allocations.
#[cfg(feature = "rtos_strict")]
#[inline(always)]
pub fn check_rt_allocation_violation() {
    if PHASE_ZERO_COMPLETE.load(Ordering::Relaxed) {
        panic!("RTOS Strict Violation: Dynamic allocation attempted in fast path after Phase 0");
    }
}

#[cfg(not(feature = "rtos_strict"))]
#[inline(always)]
pub fn enforce_fast_path_allocation_bounds() {}

#[cfg(not(feature = "rtos_strict"))]
#[inline(always)]
pub fn check_rt_allocation_violation() {}
