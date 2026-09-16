//! # Safety
//!
//! All `unsafe` blocks in this module are justified by the calling
//! functions which validate addresses, alignment, and invariants beforehand.
//!
/// Priority-Inheritance Mutex (PI Mutex)
///
/// Solves the priority-inversion problem that arises when a high-priority task
/// is blocked waiting for a lock held by a low-priority task: the owner's
/// effective priority is temporarily raised to the highest priority among all
/// waiters, ensuring the owner is scheduled promptly and releases the lock.
///
/// # Protocol
///
/// 1. `lock()` â€” acquire the mutex:
///    - Fast path: lock is free â†’ set owner, record base priority, done.
///    - Slow path: lock is busy â†’ record this caller's priority as a waiter.
///      If waiter priority > owner's current effective priority, boost the
///      owner. Spin until the lock is free (or yield if budget allows).
///
/// 2. `unlock()` â€” release the mutex (called by the guard's `Drop`):
///    - Clear owner.
///    - Restore the task's effective priority to its saved base value.
///    - Release the spinlock so a waiter can proceed.
///
/// # Limitations
///
/// * Only one level of priority boost is tracked (highest single waiter).
/// * Priority restoration is immediate (non-chained); full transitive PI would
///   require a waiter graph, which is left for future work.
/// * The mutex is spin-based â€” it does not yield the CPU on contention.
///   This is appropriate for short critical sections in a kernel context.
use alloc::sync::Arc;
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};

use crate::interfaces::task::TaskId;
use crate::kernel::task::get_task;

// â”€â”€ sentinel values â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Stored in `owner_id` when no task owns the mutex.
const NO_OWNER: u64 = u64::MAX;
/// Stored in `max_waiter_prio` when no waiters are present.
const NO_WAITER: u8 = 0;

// â”€â”€ PI mutex core â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// A Priority-Inheritance Mutex protecting a value of type `T`.
pub struct PiMutex<T> {
    /// Spinlock â€” holds the actual per-CPU exclusive section.
    lock: AtomicBool,
    /// Task ID of the current owner (NO_OWNER when free).
    owner_id: AtomicU64,
    /// Base priority of the owner at the time it acquired the lock.
    owner_base_prio: AtomicU8,
    /// Highest priority among all current waiters (0 = none).
    max_waiter_prio: AtomicU8,
    /// Protected data.
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for PiMutex<T> {}
unsafe impl<T: Send> Send for PiMutex<T> {}

impl<T> PiMutex<T> {
    /// Create a new, unlocked PI mutex.
    pub const fn new(data: T) -> Self {
        Self {
            lock: AtomicBool::new(false),
            owner_id: AtomicU64::new(NO_OWNER),
            owner_base_prio: AtomicU8::new(0),
            max_waiter_prio: AtomicU8::new(NO_WAITER),
            data: UnsafeCell::new(data),
        }
    }

    /// Acquire the lock, performing priority inheritance if needed.
    ///
    /// `caller_tid`  â€” the TaskId of the calling task (used to look up its
    ///                 priority and to record it as a waiter).
    /// `caller_prio` â€” the base priority of the calling task (avoids needing
    ///                 to lock the task registry on the fast path).
    pub fn lock(&self, caller_tid: TaskId, caller_prio: u8) -> PiMutexGuard<'_, T> {
        // Fast path â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
        if self
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            // We are now the owner.
            self.owner_id.store(caller_tid.0 as u64, Ordering::Relaxed);
            self.owner_base_prio.store(caller_prio, Ordering::Relaxed);
            self.max_waiter_prio.store(NO_WAITER, Ordering::Relaxed);
            return PiMutexGuard {
                mutex: self,
                owner_tid: caller_tid,
            };
        }

        // Slow path â€” contention â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
        //
        // Register ourselves as a waiter.  If our priority is higher than the
        // owner's current effective priority, boost the owner.
        self.register_waiter_and_boost(caller_prio);

        // Spin until the lock is free.
        let spin_limit: usize = 4_000_000;
        let mut spins = 0usize;
        loop {
            if self
                .lock
                .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                break;
            }
            spins += 1;
            PI_CONTENTION_SPINS.fetch_add(1, Ordering::Relaxed);
            if spins >= spin_limit {
                // Re-boost just in case the owner's priority drifted down.
                self.register_waiter_and_boost(caller_prio);
                spins = 0;
            }
            core::hint::spin_loop();
        }

        // We acquired the lock.  Unregister ourselves as a waiter â€” if we were
        // the sole high-priority waiter the max_waiter_prio naturally drops to
        // whatever the next highest waiter was (approximated here by leaving it
        // unchanged; the owner's priority will be restored on unlock anyway).
        self.owner_id.store(caller_tid.0 as u64, Ordering::Relaxed);
        self.owner_base_prio.store(caller_prio, Ordering::Relaxed);

        PiMutexGuard {
            mutex: self,
            owner_tid: caller_tid,
        }
    }

    /// Record `waiter_prio` and boost the current owner if needed.
    fn register_waiter_and_boost(&self, waiter_prio: u8) {
        // Update max_waiter_prio (take maximum atomically via CAS loop).
        let mut cur = self.max_waiter_prio.load(Ordering::Relaxed);
        while waiter_prio > cur {
            match self.max_waiter_prio.compare_exchange_weak(
                cur,
                waiter_prio,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(v) => cur = v,
            }
        }

        // If the owner's effective priority is less than ours, boost it.
        let owner_raw = self.owner_id.load(Ordering::Relaxed);
        if owner_raw == NO_OWNER {
            return;
        }
        let owner_tid = TaskId(owner_raw as usize);
        if let Some(task_arc) = get_task(owner_tid) {
            let mut t = task_arc.lock();
            if t.priority < waiter_prio {
                t.priority = waiter_prio;
                PI_BOOSTS_TOTAL.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Internal unlock â€” called by the guard's `Drop`.
    fn unlock(&self, owner_tid: TaskId) {
        // Restore the owner's original base priority before we release.
        let base = self.owner_base_prio.load(Ordering::Relaxed);
        if let Some(task_arc) = get_task(owner_tid) {
            let mut t = task_arc.lock();
            // Only restore if we are still the ones who boosted it.
            if t.priority > base {
                t.priority = base;
                PI_RESTORES_TOTAL.fetch_add(1, Ordering::Relaxed);
            }
        }
        // Clear owner metadata.
        self.owner_id.store(NO_OWNER, Ordering::Relaxed);
        self.max_waiter_prio.store(NO_WAITER, Ordering::Relaxed);
        // Release the spinlock.  This must be the last store.
        self.lock.store(false, Ordering::Release);
    }

    /// Returns the TaskId of the current owner, or `None` if unowned.
    pub fn owner(&self) -> Option<TaskId> {
        let v = self.owner_id.load(Ordering::Relaxed);
        if v == NO_OWNER {
            None
        } else {
            Some(TaskId(v as usize))
        }
    }

    /// Returns the highest registered waiter priority (0 means no waiters).
    pub fn max_waiter_priority(&self) -> u8 {
        self.max_waiter_prio.load(Ordering::Relaxed)
    }

    /// Manual unlock for POSIX compatibility (unsafe because it bypasses RAII).
    pub unsafe fn unlock_from_posix(&self, owner_tid: TaskId) {
        // Restore the owner's original base priority before we release.
        let base = self.owner_base_prio.load(Ordering::Relaxed);
        if let Some(task_arc) = get_task(owner_tid) {
            let mut t = task_arc.lock();
            // Only restore if we are still the ones who boosted it.
            if t.priority > base {
                t.priority = base;
                PI_RESTORES_TOTAL.fetch_add(1, Ordering::Relaxed);
            }
        }
        // Clear owner metadata.
        self.owner_id.store(NO_OWNER, Ordering::Relaxed);
        self.max_waiter_prio.store(NO_WAITER, Ordering::Relaxed);
        // Release the spinlock.
        self.lock.store(false, Ordering::Release);
    }
}

// â”€â”€ RAII guard â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// RAII guard that releases the PI mutex on drop.
pub struct PiMutexGuard<'a, T> {
    mutex: &'a PiMutex<T>,
    owner_tid: TaskId,
}

impl<'a, T> Deref for PiMutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<'a, T> DerefMut for PiMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<'a, T> Drop for PiMutexGuard<'a, T> {
    fn drop(&mut self) {
        self.mutex.unlock(self.owner_tid);
    }
}

// â”€â”€ Arc-wrapped convenience â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// A shareable PI mutex â€” wraps `PiMutex<T>` in an `Arc` for easy cloning
/// across task contexts.
pub type SharedPiMutex<T> = Arc<PiMutex<T>>;

// â”€â”€ Global PI mutex statistics â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

use core::sync::atomic::AtomicUsize;

static PI_BOOSTS_TOTAL: AtomicUsize = AtomicUsize::new(0);
static PI_RESTORES_TOTAL: AtomicUsize = AtomicUsize::new(0);
static PI_CONTENTION_SPINS: AtomicUsize = AtomicUsize::new(0);

/// Summary of PI mutex activity (for diagnostics / telemetry).
#[derive(Debug, Clone, Copy)]
pub struct PiStats {
    pub boosts: usize,
    pub restores: usize,
    pub spins: usize,
}

pub fn pi_stats() -> PiStats {
    PiStats {
        boosts: PI_BOOSTS_TOTAL.load(Ordering::Relaxed),
        restores: PI_RESTORES_TOTAL.load(Ordering::Relaxed),
        spins: PI_CONTENTION_SPINS.load(Ordering::Relaxed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interfaces::KernelTask;
    use crate::kernel::task::register_task;

    #[test_case]
    fn test_pi_mutex_basic_lock_unlock() {
        let mutex = PiMutex::new(42);
        let tid = TaskId(999);
        let guard = mutex.lock(tid, 10);
        assert_eq!(*guard, 42);
        assert_eq!(mutex.owner(), Some(tid));
        drop(guard);
        assert_eq!(mutex.owner(), None);
    }

    #[test_case]
    fn test_pi_mutex_priority_inheritance_and_telemetry() {
        std::eprintln!("TEST START");
        let mutex = PiMutex::new(100);
        let low_task = KernelTask::new(TaskId(1001), 10, 0, 0, 0, 0, 0);
        let high_task = KernelTask::new(TaskId(1002), 50, 0, 0, 0, 0, 0);
        
        std::eprintln!("REGISTER TASK LOW");
        register_task(low_task);
        std::eprintln!("REGISTER TASK HIGH");
        register_task(high_task);

        let initial_stats = pi_stats();

        // 1. Low priority task acquires the lock
        std::eprintln!("LOCK MUTEX");
        let guard = mutex.lock(TaskId(1001), 10);
        assert_eq!(mutex.owner(), Some(TaskId(1001)));

        // 2. High priority task tries to acquire and contention happens
        std::eprintln!("BOOST OWNER");
        mutex.register_waiter_and_boost(50);

        // Low priority task should be boosted to 50
        std::eprintln!("CHECK BOOSTED PRIO");
        if let Some(t) = get_task(TaskId(1001)) {
            assert_eq!(t.lock().priority, 50);
        } else {
            panic!("Task not found");
        }

        // Check telemetry
        let stats_after_boost = pi_stats();
        assert_eq!(stats_after_boost.boosts, initial_stats.boosts + 1);

        // 3. Low priority task releases the lock
        std::eprintln!("DROP GUARD");
        drop(guard);

        // Low priority task's priority should be restored to 10
        std::eprintln!("CHECK RESTORED PRIO");
        if let Some(t) = get_task(TaskId(1001)) {
            assert_eq!(t.lock().priority, 10);
        } else {
            panic!("Task not found");
        }

        // Check telemetry
        let stats_after_restore = pi_stats();
        assert_eq!(stats_after_restore.restores, initial_stats.restores + 1);
        std::eprintln!("TEST END");
    }
}

