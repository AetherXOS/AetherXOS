use core::sync::atomic::{AtomicBool, Ordering};

/// Adaptive Spinlock: Spins for a threshold, then yields.
/// This prevents CPU waste on long-held locks while maintaining low latency.
pub struct AdaptiveLock {
    locked: AtomicBool,
}

impl AdaptiveLock {
    pub const fn new() -> Self {
        Self { locked: AtomicBool::new(false) }
    }

    pub fn lock(&self) {
        let mut count = 0;
        loop {
            if !self.locked.swap(true, Ordering::Acquire) {
                return; // Acquired
            }
            
            // Adaptive spinning
            count += 1;
            if count > 1000 {
                crate::hal::HAL::cpu_relax();
                if count > 10000 {
                    // Yield if spinning too long
                    crate::kernel::task::scheduling::suspend_current_task_with_mask(
                        &crate::kernel::sync::WaitQueue::new(), 0 // Yield-only
                    );
                    count = 0;
                }
            }
        }
    }

    pub fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }
}
