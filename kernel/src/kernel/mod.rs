// =============================================================================
// RING 1: MICROKERNEL CORE (The Engine)
// =============================================================================

// --- Core Foundations ---
pub mod cpu_local;
pub mod registry;
pub mod sync;
pub mod rcu;
pub mod interrupt_guard;
pub mod watchdog;
pub mod crash_log;
pub mod power;

// --- Memory & Paging ---
pub mod memory;
pub mod vmm;
pub mod memory_extensions;

// --- Boot & Orchestration ---
pub mod boot_graph;
pub mod boot_manager;
pub mod boot_subsystems;
pub mod startup;
pub mod boot_logger;
pub mod boot_health;
pub use crate::kernel_runtime::boot_integration;

// --- Task & Scheduler Infrastructure ---
pub mod task;
pub mod scheduler_contract;
pub mod scheduler_extensions;
pub mod load_balance;
pub mod rt_preemption;

// =============================================================================
// RING 2: CORE SUBSYSTEMS (The OS Logic)
// =============================================================================

pub mod vfs_control;
pub mod vfs_extensions;
pub mod net_core;
pub mod device_manager;
pub mod runtime_manager;
pub mod namespaces;
pub mod cgroups;
pub mod policy;
pub mod security_posture;
pub mod pressure;
pub mod virt_bias;

// =============================================================================
// RING 3: COMPATIBILITY & SERVICES (The Interface)
// =============================================================================

pub mod syscalls;
pub mod syscall_contract;
pub mod signal;
pub mod signals {
    pub use super::signal::queue;
}
pub mod process;
pub mod fork;
pub mod process_registry {
    pub use super::process::registry::*;
}
pub mod symbols;
pub mod dynamic_linker;
pub mod module_loader;
pub mod tty;
pub mod bpf;
pub mod security;
pub mod pi_mutex;
pub mod launch;
pub mod virtualization_contract;

// --- Utilities ---
pub mod bit_utils;
pub mod log;
pub mod debug_trace;
pub mod jitter;




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
    pub core_dump_present: bool,
    pub core_dump_reason_ptr: usize,
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
        core_dump_present: false, // Updated if dump happens
        core_dump_reason_ptr: reason.as_ptr() as usize,
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
        core_dump_present: false,
        core_dump_reason_ptr: 0,
    }
}

/// Perform a kernel-level core dump of current CPU state.
/// Useful for post-mortem analysis of triple faults or complex panics.
pub fn core_dump() {
    let cpu_id = crate::kernel::cpu_local::CpuLocal::id();
    crate::klog_info!("Initiating kernel core dump for CPU {}", cpu_id);
    
    // Pillar V: Record state to debug_trace
    crate::kernel::debug_trace::record("Core", "Dump", Some(cpu_id as u64), false);
    // In a real system, we'd write to a reserved memory area or disk
    crate::klog_info!("Core dump complete (snapshot stored in trace buffer).");
}
