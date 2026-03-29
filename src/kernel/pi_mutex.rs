/// Priority-Inheritance Mutex (PI Mutex)
///
/// Solves the priority-inversion problem that arises when a high-priority task
/// is blocked waiting for a lock held by a low-priority task: the owner's
/// effective priority is temporarily raised to the highest priority among all
/// waiters, ensuring the owner is scheduled promptly and releases the lock.
///
/// # Protocol
///
/// 1. `lock()` — acquire the mutex:
///    - Fast path: lock is free → set owner, record base priority, done.
///    - Slow path: lock is busy → record this caller's priority as a waiter.
///      If waiter priority > owner's current effective priority, boost the
///      owner. Spin until the lock is free (or yield if budget allows).
///
/// 2. `unlock()` — release the mutex (called by the guard's `Drop`):
///    - Clear owner.
///    - Restore the task's effective priority to its saved base value.
///    - Release the spinlock so a waiter can proceed.
///
/// # Limitations
///
/// * Only one level of priority boost is tracked (highest single waiter).
/// * Priority restoration is immediate (non-chained); full transitive PI would
///   require a waiter graph, which is left for future work.
/// * The mutex is spin-based — it does not yield the CPU on contention.
///   This is appropriate for short critical sections in a kernel context.
use alloc::sync::Arc;
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};

use crate::interfaces::task::TaskId;
use crate::kernel::task::get_task;

// ── sentinel values ───────────────────────────────────────────────────────────

/// Stored in `owner_id` when no task owns the mutex.
const NO_OWNER: u64 = u64::MAX;
/// Stored in `max_waiter_prio` when no waiters are present.
const NO_WAITER: u8 = 0;

// ── PI mutex core ─────────────────────────────────────────────────────────────

/// A Priority-Inheritance Mutex protecting a value of type `T`.
pub struct PiMutex<T> {
    /// Spinlock — holds the actual per-CPU exclusive section.
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
    /// `caller_tid`  — the TaskId of the calling task (used to look up its
    ///                 priority and to record it as a waiter).
    /// `caller_prio` — the base priority of the calling task (avoids needing
    ///                 to lock the task registry on the fast path).
    pub fn lock(&self, caller_tid: TaskId, caller_prio: u8) -> PiMutexGuard<'_, T> {
        // Fast path ────────────────────────────────────────────────────────────
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

        // Slow path — contention ───────────────────────────────────────────────
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
            if spins >= spin_limit {
                // Re-boost just in case the owner's priority drifted down.
                self.register_waiter_and_boost(caller_prio);
                spins = 0;
            }
            core::hint::spin_loop();
        }

        // We acquired the lock.  Unregister ourselves as a waiter — if we were
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
            }
        }
    }

    /// Internal unlock — called by the guard's `Drop`.
    fn unlock(&self, owner_tid: TaskId) {
        // Restore the owner's original base priority before we release.
        let base = self.owner_base_prio.load(Ordering::Relaxed);
        if let Some(task_arc) = get_task(owner_tid) {
            let mut t = task_arc.lock();
            // Only restore if we are still the ones who boosted it.
            if t.priority > base {
                t.priority = base;
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
}

// ── RAII guard ────────────────────────────────────────────────────────────────

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

// ── Arc-wrapped convenience ───────────────────────────────────────────────────

/// A shareable PI mutex — wraps `PiMutex<T>` in an `Arc` for easy cloning
/// across task contexts.
pub type SharedPiMutex<T> = Arc<PiMutex<T>>;

// ── Global PI mutex statistics ────────────────────────────────────────────────

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
