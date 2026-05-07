use crate::kernel::sync::WaitQueue;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use lazy_static::lazy_static;
use spin::Mutex;

use aethercore_common::{counter_inc, declare_counter_u64, telemetry};

declare_counter_u64!(FUTEX_WAIT_CALLS);
declare_counter_u64!(FUTEX_WAIT_ENQUEUED);
declare_counter_u64!(FUTEX_WAIT_MISMATCH);
declare_counter_u64!(FUTEX_WAKE_CALLS);
declare_counter_u64!(FUTEX_WAKE_WOKEN);

#[derive(Debug, Clone, Copy)]
pub struct FutexStats {
    pub wait_calls: u64,
    pub wait_enqueued: u64,
    pub wait_value_mismatch: u64,
    pub wake_calls: u64,
    pub wake_woken: u64,
    pub active_keys: usize,
    // Extended IPC-channel stats (for futex-as-channel telemetry)
    pub send_calls: u64,
    pub send_invalid_control: u64,
    pub receive_calls: u64,
    pub receive_hits: u64,
    pub receive_small_buffer: u64,
    pub wake_event_drops: u64,
}

pub fn stats() -> FutexStats {
    let keys = FUTEX_MAP.lock().len();
    FutexStats {
        wait_calls: telemetry::snapshot_u64(&FUTEX_WAIT_CALLS),
        wait_enqueued: telemetry::snapshot_u64(&FUTEX_WAIT_ENQUEUED),
        wait_value_mismatch: telemetry::snapshot_u64(&FUTEX_WAIT_MISMATCH),
        wake_calls: telemetry::snapshot_u64(&FUTEX_WAKE_CALLS),
        wake_woken: telemetry::snapshot_u64(&FUTEX_WAKE_WOKEN),
        active_keys: keys,
        send_calls: 0,
        send_invalid_control: 0,
        receive_calls: 0,
        receive_hits: 0,
        receive_small_buffer: 0,
        wake_event_drops: 0,
    }
}

pub fn take_stats() -> FutexStats {
    let keys = FUTEX_MAP.lock().len();
    FutexStats {
        wait_calls: telemetry::take_u64(&FUTEX_WAIT_CALLS),
        wait_enqueued: telemetry::take_u64(&FUTEX_WAIT_ENQUEUED),
        wait_value_mismatch: telemetry::take_u64(&FUTEX_WAIT_MISMATCH),
        wake_calls: telemetry::take_u64(&FUTEX_WAKE_CALLS),
        wake_woken: telemetry::take_u64(&FUTEX_WAKE_WOKEN),
        active_keys: keys,
        send_calls: 0,
        send_invalid_control: 0,
        receive_calls: 0,
        receive_hits: 0,
        receive_small_buffer: 0,
        wake_event_drops: 0,
    }
}

/// Result of a futex wait operation, used by the syscall layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FutexWaitResult {
    /// Task was enqueued and subsequently woken.
    Enqueued,
    /// The value at the address differed from `expected`; no wait occurred.
    ValueMismatch,
}

struct FutexEntry {
    queue: WaitQueue,
}

lazy_static! {
    static ref FUTEX_MAP: Mutex<BTreeMap<u64, Arc<FutexEntry>>> = Mutex::new(BTreeMap::new());
}

pub struct Futex;

pub static FUTEX_MANAGER: Futex = Futex;

pub fn global() -> &'static Futex {
    &FUTEX_MANAGER
}

impl Futex {
    /// Wait on a futex key with an optional bitmask.
    pub fn wait_bitset(&self, key: u64, observed: u32, expected: u32, mask: u32) -> FutexWaitResult {
        counter_inc!(FUTEX_WAIT_CALLS);

        if observed != expected {
            counter_inc!(FUTEX_WAIT_MISMATCH);
            return FutexWaitResult::ValueMismatch;
        }

        let entry = {
            let mut map = FUTEX_MAP.lock();
            map.entry(key)
                .or_insert_with(|| {
                    Arc::new(FutexEntry {
                        queue: WaitQueue::new(),
                    })
                })
                .clone()
        };

        super::common::suspend_on_with_mask(&entry.queue, mask);
        counter_inc!(FUTEX_WAIT_ENQUEUED);
        FutexWaitResult::Enqueued
    }

    pub fn wait(&self, key: u64, observed: u32, expected: u32) -> FutexWaitResult {
        self.wait_bitset(key, observed, expected, 0xFFFF_FFFF)
    }

    /// Wake up to `max_wake` threads waiting on the key that match the bitmask.
    pub fn wake_bitset(&self, key: u64, max_wake: usize, mask: u32) -> usize {
        counter_inc!(FUTEX_WAKE_CALLS);
        if max_wake == 0 {
            return 0;
        }

        let entry = {
            let map = FUTEX_MAP.lock();
            map.get(&key).cloned()
        };

        if let Some(e) = entry {
            let woken = super::common::wake_tasks_with_mask(&e.queue, mask, max_wake);
            
            for _ in 0..woken {
                counter_inc!(FUTEX_WAKE_WOKEN);
            }

            // Cleanup if no more waiters
            if e.queue.is_empty() {
                let mut map = FUTEX_MAP.lock();
                if let Some(curr) = map.get(&key) {
                    if curr.queue.is_empty() {
                        map.remove(&key);
                    }
                }
            }
            woken
        } else {
            0
        }
    }

    pub fn wake(&self, key: u64, max_wake: usize) -> usize {
        self.wake_bitset(key, max_wake, 0xFFFF_FFFF)
    }

    /// Move up to `max_requeue` tasks from `src_key` to `dst_key`.
    pub fn requeue(&self, src_key: u64, dst_key: u64, max_wake: usize, max_requeue: usize) -> usize {
        // 1. Wake some tasks from src
        let woken = self.wake(src_key, max_wake);

        // 2. Requeue remaining tasks to dst
        let mut map = FUTEX_MAP.lock();
        
        let src_entry = map.get(&src_key).cloned();
        if let Some(src) = src_entry {
            let dst = map.entry(dst_key)
                .or_insert_with(|| Arc::new(FutexEntry { queue: WaitQueue::new() }))
                .clone();
            
            src.queue.requeue_to(&dst.queue, max_requeue);

            if src.queue.is_empty() {
                map.remove(&src_key);
            }
        }

        woken
    }
}


// POSIX/Linux Syscall glue follows in the dispatcher layers

#[cfg(test)]
mod tests {
    use super::{stats, take_stats, FutexWaitResult, FUTEX_MANAGER};

    #[test_case]
    fn futex_stats_take_resets_counters() {
        let futex = &FUTEX_MANAGER;

        let result = futex.wait(42, 1, 2);
        assert_eq!(result, FutexWaitResult::ValueMismatch);

        let before = stats();
        assert_eq!(before.wait_calls, 1);
        assert_eq!(before.wait_value_mismatch, 1);
        assert_eq!(before.wait_enqueued, 0);

        let taken = take_stats();
        assert_eq!(taken.wait_calls, 1);
        assert_eq!(taken.wait_value_mismatch, 1);

        let after = stats();
        assert_eq!(after.wait_calls, 0);
        assert_eq!(after.wait_value_mismatch, 0);
    }
}
