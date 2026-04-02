use crate::kernel::sync::WaitQueue;
use crate::kernel::task::wake_tasks;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use lazy_static::lazy_static;
use spin::Mutex;
use super::common::suspend_on;

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

static GLOBAL_FUTEX: Futex = Futex;

#[inline(always)]
pub fn global() -> &'static Futex {
    &GLOBAL_FUTEX
}

impl Futex {
    /// Wait on a futex key.
    /// Returns `FutexWaitResult` so the syscall layer can distinguish outcomes.
    pub fn wait(&self, key: u64, observed: u32, expected: u32) -> FutexWaitResult {
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

        suspend_on(&entry.queue);
        counter_inc!(FUTEX_WAIT_ENQUEUED);
        FutexWaitResult::Enqueued
    }

    /// Wake up to `max_wake` threads waiting on the key.
    pub fn wake(&self, key: u64, max_wake: usize) -> usize {
        counter_inc!(FUTEX_WAKE_CALLS);
        if max_wake == 0 {
            return 0;
        }

        let entry = {
            let map = FUTEX_MAP.lock();
            map.get(&key).cloned()
        };

        if let Some(e) = entry {
            let mut woken_tids = alloc::vec::Vec::with_capacity(max_wake);
            for _ in 0..max_wake {
                if let Some(tid) = e.queue.wake_one() {
                    woken_tids.push(tid);
                } else {
                    break;
                }
            }

            let count = woken_tids.len();
            if count > 0 {
                for _ in 0..count {
                    counter_inc!(FUTEX_WAKE_WOKEN);
                }
                wake_tasks(woken_tids);
            }

            // Cleanup if no more waiters
            if e.queue.is_empty() {
                let mut map = FUTEX_MAP.lock();
                // Check again to avoid race
                if let Some(curr) = map.get(&key) {
                    if curr.queue.is_empty() {
                        map.remove(&key);
                    }
                }
            }
            count
        } else {
            0
        }
    }
}

// POSIX/Linux Syscall glue follows in the dispatcher layers

#[cfg(test)]
mod tests {
    use super::{global, stats, take_stats, FutexWaitResult};

    #[test]
    fn futex_stats_take_resets_counters() {
        let futex = global();

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
