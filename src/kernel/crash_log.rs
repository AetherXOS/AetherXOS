use core::sync::atomic::{AtomicU64, Ordering};

use spin::Mutex;

use crate::generated_consts::CORE_CRASH_LOG_CAPACITY;

pub const EVENT_PANIC: u8 = 1;
pub const EVENT_SOFT_WATCHDOG_STALL: u8 = 2;
pub const EVENT_HARD_WATCHDOG_STALL: u8 = 3;
pub const EVENT_DRIVER_QUARANTINE: u8 = 4;

#[derive(Debug, Clone, Copy)]
pub struct CrashEvent {
    pub seq: u64,
    pub tick: u64,
    pub cpu_id: u32,
    pub task_id: u64,
    pub kind: u8,
    pub reason_hash: u64,
    pub aux0: u64,
    pub aux1: u64,
}

impl CrashEvent {
    pub const EMPTY: Self = Self {
        seq: 0,
        tick: 0,
        cpu_id: u32::MAX,
        task_id: 0,
        kind: 0,
        reason_hash: 0,
        aux0: 0,
        aux1: 0,
    };
}

static EVENTS: Mutex<[CrashEvent; CORE_CRASH_LOG_CAPACITY]> =
    Mutex::new([CrashEvent::EMPTY; CORE_CRASH_LOG_CAPACITY]);
static NEXT_SEQ: AtomicU64 = AtomicU64::new(0);

fn current_cpu_and_task() -> (u32, u64) {
    let cpu_id = crate::hal::cpu::id() as u32;
    let task_id = unsafe {
        crate::kernel::cpu_local::CpuLocal::try_get()
            .map(|cpu| cpu.current_task.load(Ordering::Relaxed) as u64)
            .unwrap_or(0)
    };
    (cpu_id, task_id)
}

pub fn record(kind: u8, reason_hash: u64, aux0: u64, aux1: u64) {
    let seq = NEXT_SEQ.fetch_add(1, Ordering::Relaxed).saturating_add(1);
    let idx = (seq as usize) % CORE_CRASH_LOG_CAPACITY;
    let tick = crate::kernel::watchdog::global_tick();
    let (cpu_id, task_id) = current_cpu_and_task();

    let event = CrashEvent {
        seq,
        tick,
        cpu_id,
        task_id,
        kind,
        reason_hash,
        aux0,
        aux1,
    };
    EVENTS.lock()[idx] = event;
}

pub fn latest() -> Option<CrashEvent> {
    let seq = NEXT_SEQ.load(Ordering::Relaxed);
    if seq == 0 {
        return None;
    }
    let idx = (seq as usize) % CORE_CRASH_LOG_CAPACITY;
    let ev = EVENTS.lock()[idx];
    if ev.seq == 0 {
        None
    } else {
        Some(ev)
    }
}

#[inline(always)]
pub fn event_count() -> usize {
    let seq = NEXT_SEQ.load(Ordering::Relaxed) as usize;
    core::cmp::min(seq, CORE_CRASH_LOG_CAPACITY)
}

pub fn recent_into(out: &mut [CrashEvent]) -> usize {
    if out.is_empty() {
        return 0;
    }

    let events = EVENTS.lock();
    let total = event_count();
    if total == 0 {
        return 0;
    }

    let n = core::cmp::min(total, out.len());
    let oldest = if total == CORE_CRASH_LOG_CAPACITY {
        (NEXT_SEQ.load(Ordering::Relaxed) as usize) % CORE_CRASH_LOG_CAPACITY
    } else {
        0
    };
    let start = total.saturating_sub(n);
    let mut cursor = (oldest + start) % CORE_CRASH_LOG_CAPACITY;
    let mut written = 0usize;

    while written < n {
        let ev = events[cursor];
        if ev.seq != 0 {
            out[written] = ev;
            written += 1;
        }
        cursor = (cursor + 1) % CORE_CRASH_LOG_CAPACITY;
    }
    written
}
