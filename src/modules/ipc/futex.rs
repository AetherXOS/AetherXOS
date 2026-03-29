use crate::kernel::sync::WaitQueue;
use crate::kernel::task::{suspend_current_task, wake_tasks};
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicU64, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

static FUTEX_WAIT_CALLS: AtomicU64 = AtomicU64::new(0);
static FUTEX_WAIT_ENQUEUED: AtomicU64 = AtomicU64::new(0);
static FUTEX_WAIT_MISMATCH: AtomicU64 = AtomicU64::new(0);
static FUTEX_WAKE_CALLS: AtomicU64 = AtomicU64::new(0);
static FUTEX_WAKE_WOKEN: AtomicU64 = AtomicU64::new(0);

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
        wait_calls: FUTEX_WAIT_CALLS.load(Ordering::Relaxed),
        wait_enqueued: FUTEX_WAIT_ENQUEUED.load(Ordering::Relaxed),
        wait_value_mismatch: FUTEX_WAIT_MISMATCH.load(Ordering::Relaxed),
        wake_calls: FUTEX_WAKE_CALLS.load(Ordering::Relaxed),
        wake_woken: FUTEX_WAKE_WOKEN.load(Ordering::Relaxed),
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
        FUTEX_WAIT_CALLS.fetch_add(1, Ordering::Relaxed);

        if observed != expected {
            FUTEX_WAIT_MISMATCH.fetch_add(1, Ordering::Relaxed);
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

        suspend_current_task(&entry.queue);
        FUTEX_WAIT_ENQUEUED.fetch_add(1, Ordering::Relaxed);
        FutexWaitResult::Enqueued
    }

    /// Wake up to `max_wake` threads waiting on the key.
    pub fn wake(&self, key: u64, max_wake: usize) -> usize {
        FUTEX_WAKE_CALLS.fetch_add(1, Ordering::Relaxed);
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
                FUTEX_WAKE_WOKEN.fetch_add(count as u64, Ordering::Relaxed);
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
