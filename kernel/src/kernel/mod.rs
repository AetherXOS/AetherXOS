// --- Foundations & Utilities ---
pub mod bit_utils;
pub mod boot_logger;
pub mod log;
pub mod debug_trace;
pub mod cpu_local;
pub mod sync;
pub mod rcu;
pub mod interrupt_guard;
pub mod jitter;
pub mod watchdog;
pub mod boot_health;
pub mod crash_log;
pub mod power;

// --- Memory & Virtualization ---
pub mod memory;
pub mod vmm;
pub mod virt_bias;
pub mod virtualization_contract;

// --- Process & Task Management ---
#[cfg(feature = "process_abstraction")]
pub mod process;
#[cfg(feature = "process_abstraction")]
pub mod fork;
#[cfg(feature = "process_abstraction")]
pub mod process_registry {
    pub use super::process::registry::*;
}
pub mod task;
pub mod scheduler_contract;
pub mod load_balance;
pub mod rt_preemption;

// --- Execution & Loading ---
pub mod launch;
pub mod startup;
pub mod boot_manager;
pub mod boot_subsystems;
pub mod device_manager;
pub mod runtime_manager;
pub mod scheduler_extensions;
pub mod memory_extensions;
pub mod vfs_extensions;
pub mod dynamic_linker;
pub mod module_loader;

// --- Security & Resource Management ---
pub mod namespaces;
pub mod cgroups;
pub mod policy;
pub mod security_posture;
pub mod pressure;

// --- Inter-Process & Syscalls ---
pub mod syscalls;
pub mod syscall_contract;
pub mod signal;
pub mod signals {
    pub use super::signal::queue;
}
pub mod symbols;
pub mod pi_mutex;

// --- Subsystems ---
pub mod tty;
pub mod net_core;
pub mod vfs_control;
pub mod bpf;



// P0 ABI and IPC parity tests
#[cfg(test)]
mod tests;

use crate::hal::HAL;
use crate::interfaces::HardwareAbstraction;
#[allow(unused_imports)]
use crate::klog_error;
#[allow(unused_imports)]
use crate::klog_trace;
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicU64, Ordering};

static PANIC_COUNT: AtomicU64 = AtomicU64::new(0);
static LAST_PANIC_TICK: AtomicU64 = AtomicU64::new(0);
static LAST_PANIC_REASON_HASH: AtomicU64 = AtomicU64::new(0);
const FNV64_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV64_PRIME: u64 = 0x100000001b3;

#[derive(Debug, Clone, Copy)]
pub struct CrashReport {
    pub panic_count: u64,
    pub last_panic_tick: u64,
    pub last_reason_hash: u64,
    pub watchdog_tick: u64,
    pub watchdog_stalls: u64,
    pub watchdog_hard_panics: u64,
    pub startup_stage_transitions: u64,
    pub startup_order_violations: u64,
    pub crash_log_latest_seq: u64,
    pub crash_log_latest_kind: u8,
}

#[inline(always)]
fn hash_reason(reason: &str) -> u64 {
    let mut hash = FNV64_OFFSET_BASIS;
    for &b in reason.as_bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(FNV64_PRIME);
    }
    hash
}

pub fn panic_report(info: &PanicInfo, reason: &str) -> ! {
    let count = PANIC_COUNT
        .fetch_add(1, Ordering::Relaxed)
        .saturating_add(1);
    let tick = crate::kernel::watchdog::global_tick();
    LAST_PANIC_TICK.store(tick, Ordering::Relaxed);
    let reason_hash = hash_reason(reason);
    LAST_PANIC_REASON_HASH.store(reason_hash, Ordering::Relaxed);
    crate::kernel::crash_log::record(crate::kernel::crash_log::EVENT_PANIC, reason_hash, 0, 0);

    let watchdog_stats = crate::kernel::watchdog::stats();
    let startup_stats = crate::kernel::startup::diagnostics();
    let crash_log_stats = crate::kernel::crash_log::stats();

    let report = CrashReport {
        panic_count: count,
        last_panic_tick: tick,
        last_reason_hash: reason_hash,
        watchdog_tick: watchdog_stats.global_tick,
        watchdog_stalls: watchdog_stats.stall_detections,
        watchdog_hard_panics: watchdog_stats.hard_panic_triggered,
        startup_stage_transitions: startup_stats.transitions,
        startup_order_violations: startup_stats.ordering_violations,
        crash_log_latest_seq: crash_log_stats.latest_seq,
        crash_log_latest_kind: crash_log_stats.latest_event_kind,
    };

    crate::hal::HAL::panic_with_report(info, &report);
}

pub fn core_pressure_snapshot() -> crate::kernel::pressure::CorePressureSnapshot {
    crate::kernel::pressure::snapshot()
}

pub fn core_load_avg() -> [u64; 3] {
    crate::kernel::watchdog::load_avg()
}

pub fn idle_once() {
    HAL::idle_once();
}

pub fn fatal_halt(reason: &str) -> ! {
    HAL::fatal_halt(reason);
}

pub fn crash_report() -> CrashReport {
    let watchdog_stats = crate::kernel::watchdog::stats();
    let startup_stats = crate::kernel::startup::diagnostics();
    let crash_log_stats = crate::kernel::crash_log::stats();
    CrashReport {
        panic_count: PANIC_COUNT.load(Ordering::Relaxed),
        last_panic_tick: LAST_PANIC_TICK.load(Ordering::Relaxed),
        last_reason_hash: LAST_PANIC_REASON_HASH.load(Ordering::Relaxed),
        watchdog_tick: watchdog_stats.global_tick,
        watchdog_stalls: watchdog_stats.stall_detections,
        watchdog_hard_panics: watchdog_stats.hard_panic_triggered,
        startup_stage_transitions: startup_stats.transitions,
        startup_order_violations: startup_stats.ordering_violations,
        crash_log_latest_seq: crash_log_stats.latest_seq,
        crash_log_latest_kind: crash_log_stats.latest_event_kind,
    }
}
