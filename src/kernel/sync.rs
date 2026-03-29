#[cfg(target_os = "none")]
use crate::hal::HAL;
#[cfg(target_os = "none")]
use crate::interfaces::HardwareAbstraction;
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};

/// A Mutex that disables interrupts while locked.
/// Essential for preventing deadlocks in interrupt handlers.
///
/// If a thread holds a regular Mutex and an interrupt fires,
/// and the interrupt handler tries to acquire the same Mutex,
/// the system deadlocks. This struct prevents that by disabling IRQs.
pub struct IrqSafeMutex<T> {
    lock: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for IrqSafeMutex<T> {}
unsafe impl<T: Send> Send for IrqSafeMutex<T> {}

impl<T> IrqSafeMutex<T> {
    pub const fn new(data: T) -> Self {
        Self {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    pub fn lock(&self) -> IrqSafeMutexGuard<'_, T> {
        // 1. Disable Interrupts and Save Flags
        #[cfg(target_os = "none")]
        let flags = HAL::irq_save();
        #[cfg(not(target_os = "none"))]
        let _flags = 0usize;
        let deadlock_spin_limit = crate::config::KernelConfig::irqsafe_mutex_deadlock_spin_limit();

        // 2. Spin loop with bounded iteration to detect deadlocks
        let mut spin_count: usize = 0;
        while self
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            spin_count += 1;
            if spin_count >= deadlock_spin_limit {
                // Probable deadlock: same CPU trying to re-lock, or cross-CPU
                // contention exceeding reasonable bounds. Restore IRQs and panic.
                #[cfg(target_os = "none")]
                HAL::irq_restore(flags);
                panic!(
                    "IrqSafeMutex: probable deadlock detected after {} spins",
                    deadlock_spin_limit
                );
            }
            core::hint::spin_loop();
        }

        IrqSafeMutexGuard {
            mutex: self,
            #[cfg(target_os = "none")]
            saved_flags: flags,
        }
    }

    /// Attempt to acquire the lock without blocking.
    /// Returns `None` if the lock is already held (no interrupt state change in that case).
    pub fn try_lock(&self) -> Option<IrqSafeMutexGuard<'_, T>> {
        #[cfg(target_os = "none")]
        let flags = HAL::irq_save();
        #[cfg(not(target_os = "none"))]
        let _flags = 0usize;

        if self
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(IrqSafeMutexGuard {
                mutex: self,
                #[cfg(target_os = "none")]
                saved_flags: flags,
            })
        } else {
            // Failed to acquire — restore interrupt state
            #[cfg(target_os = "none")]
            HAL::irq_restore(flags);
            None
        }
    }

    /// Borrow the protected value without taking the spin lock.
    ///
    /// # Safety
    /// The caller must guarantee exclusive access to the protected value for the
    /// entire lifetime of the returned borrow. This is intended only for
    /// bootstrap paths where the object is not yet published to any concurrent
    /// runtime structure.
    pub unsafe fn bootstrap_borrow_mut(&self) -> &mut T {
        unsafe { &mut *self.data.get() }
    }
}

pub struct IrqSafeMutexGuard<'a, T> {
    mutex: &'a IrqSafeMutex<T>,
    #[cfg(target_os = "none")]
    saved_flags: usize,
}

impl<'a, T> Deref for IrqSafeMutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<'a, T> DerefMut for IrqSafeMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<'a, T> Drop for IrqSafeMutexGuard<'a, T> {
    fn drop(&mut self) {
        // 1. Unlock
        self.mutex.lock.store(false, Ordering::Release);

        // 2. Restore Interrupts (IF flag)
        #[cfg(target_os = "none")]
        HAL::irq_restore(self.saved_flags);
    }
}

use crate::interfaces::task::TaskId;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

pub struct WaitQueue {
    waiters: IrqSafeMutex<VecDeque<TaskId>>,
}

impl WaitQueue {
    pub const fn new() -> Self {
        Self {
            waiters: IrqSafeMutex::new(VecDeque::new()),
        }
    }

    /// Block a task by its ID.
    pub fn block_id(&self, tid: TaskId) {
        self.waiters.lock().push_back(tid);
    }

    /// Wake one task ID.
    pub fn wake_one(&self) -> Option<TaskId> {
        self.waiters.lock().pop_front()
    }

    /// Wake all task IDs.
    pub fn wake_all(&self) -> Vec<TaskId> {
        let mut waiters = self.waiters.lock();
        let mut out = Vec::new();
        while let Some(tid) = waiters.pop_front() {
            out.push(tid);
        }
        out
    }

    pub fn len(&self) -> usize {
        self.waiters.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.waiters.lock().is_empty()
    }

    /// Remove a specific task ID from the wait queue (used when blocking is undone).
    pub fn unblock_id(&self, tid: TaskId) {
        let mut q = self.waiters.lock();
        if let Some(pos) = q.iter().position(|&t| t == tid) {
            q.remove(pos);
        }
    }
}
