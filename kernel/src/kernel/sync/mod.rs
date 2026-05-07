#[cfg(target_os = "none")]
use crate::hal::HAL;
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};

pub mod ring_buffer;

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

impl<T: core::fmt::Debug> core::fmt::Debug for IrqSafeMutex<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.try_lock() {
            Some(guard) => f.debug_struct("IrqSafeMutex").field("data", &*guard).finish(),
            None => f.debug_struct("IrqSafeMutex").field("data", &"<locked>").finish(),
        }
    }
}

use crate::interfaces::task::TaskId;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

pub struct WaitQueue {
    waiters: IrqSafeMutex<VecDeque<(TaskId, u32)>>,
}

impl core::fmt::Debug for WaitQueue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("WaitQueue").finish()
    }
}

impl WaitQueue {
    pub const fn new() -> Self {
        Self {
            waiters: IrqSafeMutex::new(VecDeque::new()),
        }
    }

    /// Block a task by its ID with an optional bitmask.
    pub fn block_id_with_mask(&self, tid: TaskId, mask: u32) {
        self.waiters.lock().push_back((tid, mask));
    }

    pub fn block_id(&self, tid: TaskId) {
        self.block_id_with_mask(tid, 0xFFFF_FFFF);
    }

    /// Wake one task ID that matches the bitmask.
    pub fn wake_one_with_mask(&self, mask: u32) -> Option<TaskId> {
        let mut q = self.waiters.lock();
        if let Some(pos) = q.iter().position(|&(_, m)| (m & mask) != 0) {
            return Some(q.remove(pos).unwrap().0);
        }
        None
    }

    pub fn wake_one(&self) -> Option<TaskId> {
        self.wake_one_with_mask(0xFFFF_FFFF)
    }

    /// Wake all task IDs that match the bitmask.
    pub fn wake_all_with_mask(&self, mask: u32) -> Vec<TaskId> {
        let mut q = self.waiters.lock();
        let mut out = Vec::new();
        let mut i = 0;
        while i < q.len() {
            if (q[i].1 & mask) != 0 {
                out.push(q.remove(i).unwrap().0);
            } else {
                i += 1;
            }
        }
        out
    }

    pub fn wake_all(&self) -> Vec<TaskId> {
        self.wake_all_with_mask(0xFFFF_FFFF)
    }

    pub fn len(&self) -> usize {
        self.waiters.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.waiters.lock().is_empty()
    }

    pub fn unblock_id(&self, tid: TaskId) {
        let mut q = self.waiters.lock();
        if let Some(pos) = q.iter().position(|&(t, _)| t == tid) {
            q.remove(pos);
        }
    }

    /// Move tasks from this queue to another queue (requeue).
    pub fn requeue_to(&self, other: &WaitQueue, max_count: usize) -> usize {
        let mut src = self.waiters.lock();
        let mut dst = other.waiters.lock();
        let mut count = 0;
        while count < max_count {
            if let Some(entry) = src.pop_front() {
                dst.push_back(entry);
                count += 1;
            } else {
                break;
            }
        }
        count
    }

    /// Block the current task on this queue.
    pub fn wait(&self) {
        let tid = crate::modules::posix::process::gettid();
        if tid == 0 {
            return;
        }
        self.block_id(crate::interfaces::TaskId(tid));
        crate::kernel::rt_preemption::request_forced_reschedule();
    }
}

pub struct PerCpu<T> {
    data: [T; crate::generated_consts::KERNEL_MAX_CPUS],
}

impl<T> PerCpu<T> {
    pub const fn new(data: [T; crate::generated_consts::KERNEL_MAX_CPUS]) -> Self {
        Self { data }
    }

    pub fn get(&self) -> &T {
        let id = crate::kernel::cpu_local::CpuLocal::id();
        &self.data[id]
    }

    pub fn get_mut(&mut self) -> &mut T {
        let id = crate::kernel::cpu_local::CpuLocal::id();
        &mut self.data[id]
    }
}

