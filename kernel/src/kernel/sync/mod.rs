//! # Safety
//!
//! All `unsafe` blocks in this module are justified by the calling
//! functions which validate addresses, alignment, and invariants beforehand.
//!
//! Synchronization primitives: IrqSafeMutex, WaitQueue, hazard pointers, ring buffer.

#[cfg(target_os = "none")]
use crate::hal::HAL;
use alloc::sync::Arc;
use alloc::vec::Vec;
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
                #[cfg(target_os = "none")]
                HAL::irq_restore(flags);
                crate::klog_error!(
                    "IrqSafeMutex: probable deadlock detected after {} spins",
                    deadlock_spin_limit
                );
                crate::kernel::fatal_halt("IrqSafeMutex deadlock");
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
            // Failed to acquire â€” restore interrupt state
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
        crate::kernel::task::scheduling::suspend_current_task(self);
    }
}

pub struct WaitAggregator {
    queues: Vec<Arc<WaitQueue>>,
}

impl WaitAggregator {
    pub fn new() -> Self {
        Self {
            queues: Vec::new(),
        }
    }

    pub fn add(&mut self, queue: Arc<WaitQueue>) {
        self.queues.push(queue);
    }

    pub fn wait(&self) {
        crate::kernel::task::scheduling::suspend_current_task_multi(&self.queues);
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





// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_irq_safe_mutex_new() {
        let m = IrqSafeMutex::new(42u32);
        assert_eq!(*m.lock(), 42);
    }

    #[test_case]
    fn test_irq_safe_mutex_lock_drop() {
        let m = IrqSafeMutex::new(String::from("hello"));
        {
            let mut guard = m.lock();
            guard.push_str(" world");
        }
        assert_eq!(*m.lock(), "hello world");
    }

    #[test_case]
    fn test_irq_safe_mutex_try_lock() {
        let m = IrqSafeMutex::new(10u32);
        let guard = m.try_lock();
        assert!(guard.is_some());
        drop(guard);
        assert!(m.try_lock().is_some());
    }

    #[test_case]
    fn test_wait_queue_basic() {
        let wq = WaitQueue::new();
        assert!(wq.is_empty());
        wq.block_id(TaskId(42));
        assert_eq!(wq.len(), 1);
        assert!(!wq.is_empty());
        let woken = wq.wake_one();
        assert_eq!(woken, Some(TaskId(42)));
        assert!(wq.is_empty());
    }

    #[test_case]
    fn test_wait_queue_wake_mask() {
        let wq = WaitQueue::new();
        wq.block_id_with_mask(TaskId(1), 0x1);
        wq.block_id_with_mask(TaskId(2), 0x2);
        wq.block_id_with_mask(TaskId(3), 0x4);
        
        // Wake with mask 0x2 should wake Task ID 2.
        let woken = wq.wake_one_with_mask(0x2);
        assert_eq!(woken, Some(TaskId(2)));
        assert_eq!(wq.len(), 2);
        
        // Wake all with mask 0x5 should wake 1 and 3.
        let all = wq.wake_all_with_mask(0x5);
        assert_eq!(all.len(), 2);
        assert!(all.contains(&TaskId(1)));
        assert!(all.contains(&TaskId(3)));
        assert!(wq.is_empty());
    }

    #[test_case]
    fn test_wait_queue_requeue() {
        let src = WaitQueue::new();
        let dst = WaitQueue::new();
        src.block_id(TaskId(1));
        src.block_id(TaskId(2));
        src.block_id(TaskId(3));
        
        let moved = src.requeue_to(&dst, 2);
        assert_eq!(moved, 2);
        assert_eq!(src.len(), 1);
        assert_eq!(dst.len(), 2);
    }

    #[test_case]
    fn test_wait_queue_unblock() {
        let wq = WaitQueue::new();
        wq.block_id(TaskId(1));
        wq.block_id(TaskId(2));
        wq.unblock_id(TaskId(1));
        assert_eq!(wq.len(), 1);
        assert_eq!(wq.wake_one(), Some(TaskId(2)));
    }

    #[test_case]
    fn test_irq_safe_mutex_unsafe_get() {
        let m = IrqSafeMutex::new(99u32);
        // SAFETY: Single-threaded test context.
        unsafe {
            assert_eq!(*m.unsafe_get(), 99);
        }
    }

    #[test_case]
    fn test_wait_queue_wake_all() {
        let wq = WaitQueue::new();
        wq.block_id(TaskId(10));
        wq.block_id(TaskId(20));
        wq.block_id(TaskId(30));
        
        let all = wq.wake_all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&TaskId(10)));
        assert!(all.contains(&TaskId(20)));
        assert!(all.contains(&TaskId(30)));
        assert!(wq.is_empty());
    }
}
