pub mod bit_utils;
pub mod cgroups;
#[cfg(feature = "process_abstraction")]
pub mod fork;
pub mod memory;
pub mod namespaces;
#[cfg(feature = "process_abstraction")]
pub mod process;
#[cfg(feature = "process_abstraction")]
pub mod process_registry;
pub mod syscalls;
pub mod task;

// The Kernel Core manages the active modules.

pub mod boot_health;
pub mod cpu_local;
pub mod crash_log;
pub mod dynamic_linker;
pub mod debug_trace;
pub mod interrupt_guard;
pub mod launch;
pub mod load_balance;
pub mod log;
pub mod module_loader;
pub mod net_core;
pub mod pi_mutex;
pub mod policy;
pub mod power;
pub mod pressure;
pub mod rcu;
pub mod rt_preemption;
pub mod scheduler_contract;
pub mod signal;
pub mod startup;
pub mod sync;
pub mod tty;
pub mod syscall_contract;
pub mod vfs_control;
pub mod virt_bias;
pub mod virtualization_contract;
pub mod vmm;
pub mod watchdog;

// P0 ABI and IPC parity tests
#[cfg(test)]
mod tests;

use crate::config::KernelConfig;
use crate::hal::HAL;
use crate::interfaces::{HardwareAbstraction, Scheduler};
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

    let wd = crate::kernel::watchdog::stats();
    let diag = crate::kernel::startup::diagnostics();
    if let Some(loc) = info.location() {
        klog_error!(
            "PANIC report: count={} reason={} hash={:#x} at {}:{}:{} watchdog_tick={} stalls={} hard_panics={} startup_transitions={} startup_violations={}",
            count,
            reason,
            reason_hash,
            loc.file(),
            loc.line(),
            loc.column(),
            wd.global_tick,
            wd.stall_detections,
            wd.hard_panic_triggered,
            diag.transitions,
            diag.ordering_violations
        );
    } else {
        klog_error!(
            "PANIC report: count={} reason={} hash={:#x} watchdog_tick={} stalls={} hard_panics={} startup_transitions={} startup_violations={}",
            count,
            reason,
            reason_hash,
            wd.global_tick,
            wd.stall_detections,
            wd.hard_panic_triggered,
            diag.transitions,
            diag.ordering_violations
        );
    }
    klog_error!("PANIC message: {}", info);

    fatal_halt(reason)
}

#[inline(always)]
pub fn crash_report() -> CrashReport {
    let wd = crate::kernel::watchdog::stats();
    let diag = crate::kernel::startup::diagnostics();
    CrashReport {
        panic_count: PANIC_COUNT.load(Ordering::Relaxed),
        last_panic_tick: LAST_PANIC_TICK.load(Ordering::Relaxed),
        last_reason_hash: LAST_PANIC_REASON_HASH.load(Ordering::Relaxed),
        watchdog_tick: wd.global_tick,
        watchdog_stalls: wd.stall_detections,
        watchdog_hard_panics: wd.hard_panic_triggered,
        startup_stage_transitions: diag.transitions,
        startup_order_violations: diag.ordering_violations,
        crash_log_latest_seq: crate::kernel::crash_log::latest()
            .map(|e| e.seq)
            .unwrap_or(0),
        crash_log_latest_kind: crate::kernel::crash_log::latest()
            .map(|e| e.kind)
            .unwrap_or(0),
    }
}

pub fn dump_kernel_state(reason: &str) {
    if !KernelConfig::is_kernel_dump_enabled() {
        return;
    }

    let cpu_id = crate::hal::cpu::id();
    let online_cpus = crate::hal::smp::CPUS.lock().len();
    let current_task = unsafe {
        crate::kernel::cpu_local::CpuLocal::try_get()
            .map(|cpu| cpu.current_task.load(core::sync::atomic::Ordering::Relaxed))
    };
    let lb = crate::kernel::load_balance::stats_snapshot();
    let wd = crate::kernel::watchdog::stats();
    let startup = crate::kernel::startup::diagnostics();
    let crash = crash_report();
    let launch = crate::kernel::launch::stats();
    let loader = crate::kernel::module_loader::stats();
    let rt = crate::kernel::rt_preemption::stats();
    let (task_total, task_running, task_blocked, task_ready) = crate::kernel::task::task_stats();
    let trace = crate::kernel::debug_trace::stats();
    let trace_categories = crate::kernel::debug_trace::category_stats();
    let latest_core = crate::kernel::debug_trace::latest_record_for_category(
        crate::kernel::debug_trace::TraceCategory::Core,
    );
    let latest_launch = crate::kernel::debug_trace::latest_record_for_category(
        crate::kernel::debug_trace::TraceCategory::Launch,
    );
    let latest_loader = crate::kernel::debug_trace::latest_record_for_category(
        crate::kernel::debug_trace::TraceCategory::Loader,
    );
    let latest_task = crate::kernel::debug_trace::latest_record_for_category(
        crate::kernel::debug_trace::TraceCategory::Task,
    );
    let latest_memory = crate::kernel::debug_trace::latest_record_for_category(
        crate::kernel::debug_trace::TraceCategory::Memory,
    );
    let latest_scheduler = crate::kernel::debug_trace::latest_record_for_category(
        crate::kernel::debug_trace::TraceCategory::Scheduler,
    );
    let latest_fault = crate::kernel::debug_trace::latest_record_for_category(
        crate::kernel::debug_trace::TraceCategory::Fault,
    );
    let mut launch_entries = [crate::kernel::launch::LaunchRegistrySnapshotEntry::default(); 4];
    let launch_entry_count = crate::kernel::launch::launch_registry_snapshot(&mut launch_entries);
    let mut task_entries = [crate::kernel::task::TaskRegistrySnapshotEntry::default(); 8];
    let task_entry_count = crate::kernel::task::task_registry_snapshot(&mut task_entries);
    #[cfg(target_arch = "x86_64")]
    let serial = crate::hal::x86_64::serial::stats();
    #[cfg(target_arch = "aarch64")]
    let serial = crate::hal::aarch64::serial::stats();

    klog_error!("\n[KERNEL DUMP] reason={}", reason);
    klog_error!(
        "[KERNEL DUMP] cpu_id={} online_cpus={}",
        cpu_id,
        online_cpus
    );
    klog_error!(
        "[KERNEL DUMP] rebalance attempts={} moved={} affinity_skips={} prefer_local_skips={} prefer_local_forced={} p50={} p90={} p99={} samples={}",
        lb.attempts,
        lb.moved,
        lb.affinity_skips,
        lb.prefer_local_skips,
        lb.prefer_local_forced_moves,
        lb.imbalance_p50,
        lb.imbalance_p90,
        lb.imbalance_p99,
        lb.imbalance_samples
    );
    klog_error!(
        "[KERNEL DUMP] watchdog tick={} checks={} stalls={} hard_panics={} hard_ticks={}",
        wd.global_tick,
        wd.checks,
        wd.stall_detections,
        wd.hard_panic_triggered,
        wd.hard_panic_ticks
    );
    klog_error!(
        "[KERNEL DUMP] startup stage={:?} transitions={} violations={}",
        startup.last_stage,
        startup.transitions,
        startup.ordering_violations
    );
    klog_error!(
        "[KERNEL DUMP] panic_count={} last_panic_tick={} last_reason_hash={:#x}",
        crash.panic_count,
        crash.last_panic_tick,
        crash.last_reason_hash
    );
    if let Some(ev) = crate::kernel::crash_log::latest() {
        klog_error!(
            "[KERNEL DUMP] crash_event seq={} kind={} tick={} cpu={} task={} reason_hash={:#x} aux0={} aux1={}",
            ev.seq,
            ev.kind,
            ev.tick,
            ev.cpu_id,
            ev.task_id,
            ev.reason_hash,
            ev.aux0,
            ev.aux1
        );
    } else {
        klog_error!("[KERNEL DUMP] crash_event=none");
    }
    klog_error!(
        "[KERNEL DUMP] launch spawn_attempts={} success={} failures={} enqueue_failures={} validation_failures={} claim_success={} consume_success={} execute_success={} registered_processes={} last_task={}",
        launch.spawn_attempts,
        launch.spawn_success,
        launch.spawn_failures,
        launch.enqueue_failures,
        launch.validation_failures,
        launch.claim_success,
        launch.handoff_consume_success,
        launch.handoff_execute_success,
        launch.registered_processes,
        launch.last_task_id.0
    );
    for entry in launch_entries.iter().take(launch_entry_count) {
        klog_error!(
            "[KERNEL DUMP] launch entry pid={} tid={} stage={} image_pages={} mapped_pages={}",
            entry.process_id.0,
            entry.task_id.0,
            entry.stage,
            entry.image_pages,
            entry.mapped_pages
        );
    }
    klog_error!(
        "[KERNEL DUMP] loader preflight={}/{} parse={}/{} plan={}/{} mapping={}/{} bootstrap_task={}/{} materialize={}/{} bytes={}",
        loader.preflight_success,
        loader.preflight_attempts,
        loader.parse_success,
        loader.parse_attempts,
        loader.plan_success,
        loader.plan_attempts,
        loader.mapping_plan_success,
        loader.mapping_plan_attempts,
        loader.bootstrap_task_success,
        loader.bootstrap_task_attempts,
        loader.segment_materialization_success,
        loader.segment_materialization_attempts,
        loader.segment_materialized_bytes
    );
    klog_error!(
        "[KERNEL DUMP] trace events={} valued={} dump={} warn={} fault={} context={} dropped={} latest_seq={}",
        trace.events,
        trace.valued_events,
        trace.dump_events,
        trace.warn_events,
        trace.fault_events,
        trace.context_events,
        trace.dropped_history,
        trace.latest_seq
    );
    klog_error!(
        "[KERNEL DUMP] trace categories core={} launch={} loader={} task={} memory={} scheduler={} fault={} unknown={}",
        trace_categories.core,
        trace_categories.launch,
        trace_categories.loader,
        trace_categories.task,
        trace_categories.memory,
        trace_categories.scheduler,
        trace_categories.fault,
        trace_categories.unknown
    );
    for latest in [
        latest_core,
        latest_launch,
        latest_loader,
        latest_task,
        latest_memory,
        latest_scheduler,
        latest_fault,
    ]
    .into_iter()
    .flatten()
    {
        if (latest.flags & 1) != 0 {
            klog_error!(
                "[KERNEL DUMP] trace latest cat={} seq={} {} {} value={:#x}",
                latest.category_str(),
                latest.seq,
                latest.scope_str(),
                latest.stage_str(),
                latest.value
            );
        } else {
            klog_error!(
                "[KERNEL DUMP] trace latest cat={} seq={} {} {}",
                latest.category_str(),
                latest.seq,
                latest.scope_str(),
                latest.stage_str()
            );
        }
    }
    crate::kernel::debug_trace::dump_category_to_klog(
        crate::kernel::debug_trace::TraceCategory::Launch,
        6,
    );
    crate::kernel::debug_trace::dump_category_to_klog(
        crate::kernel::debug_trace::TraceCategory::Loader,
        8,
    );
    crate::kernel::debug_trace::dump_category_to_klog(
        crate::kernel::debug_trace::TraceCategory::Task,
        8,
    );
    crate::kernel::debug_trace::dump_category_to_klog(
        crate::kernel::debug_trace::TraceCategory::Fault,
        8,
    );
    klog_error!(
        "[KERNEL DUMP] rt ticks={} reschedules={} forced={} streak={} max_streak={} runqueue={} starved={} deadline_alert={} deadline_events={}",
        rt.ticks,
        rt.reschedules,
        rt.forced_reschedules,
        rt.continue_streak,
        rt.max_continue_streak,
        rt.last_runqueue_len,
        rt.starvation_alert,
        rt.deadline_alert_active,
        rt.deadline_alert_events
    );
    klog_error!(
        "[KERNEL DUMP] tasks total={} running={} blocked={} ready={}",
        task_total,
        task_running,
        task_blocked,
        task_ready
    );
    klog_error!(
        "[KERNEL DUMP] serial tx_bytes={} drops={} spins={} timeouts={} trace_events={}",
        serial.tx_bytes,
        serial.tx_drops,
        serial.tx_spin_loops,
        serial.tx_timeouts,
        serial.trace_events
    );
    klog_error!(
        "[KERNEL DUMP] trace stats events={} valued={} dumps={} warns={} faults={} dropped_history={} latest_seq={}",
        trace.events,
        trace.valued_events,
        trace.dump_events,
        trace.warn_events,
        trace.fault_events,
        trace.dropped_history,
        trace.latest_seq
    );
    for entry in task_entries.iter().take(task_entry_count) {
        if let Some((state, pid, cr3, _)) =
            crate::kernel::task::task_context_snapshot(entry.task_id)
        {
            klog_error!(
                "[KERNEL DUMP] task tid={} state={:?} pid={} cr3={:#x} ksp={:#x}",
                entry.task_id.0,
                state,
                pid.map(|p| p.0).unwrap_or(0),
                cr3,
                entry.kernel_stack_pointer
            );
        } else {
            klog_error!(
                "[KERNEL DUMP] task tid={} state_raw={} pid={} ksp={:#x} snapshot=stale",
                entry.task_id.0,
                entry.state,
                entry.process_id,
                entry.kernel_stack_pointer
            );
        }
    }
    crate::kernel::debug_trace::dump_to_klog();
    #[cfg(target_os = "none")]
    crate::kernel::debug_trace::dump_to_early_serial();
    #[cfg(feature = "sched_lottery")]
    {
        let lot = crate::modules::schedulers::lottery::runtime_stats();
        klog_error!(
            "[KERNEL DUMP] lottery add={} remove={} picks={} empty={} fallback_first={} replay_seq={} replay_overwrites={}",
            lot.add_calls,
            lot.remove_calls,
            lot.pick_calls,
            lot.pick_empty,
            lot.fallback_first,
            lot.replay_latest_seq,
            lot.replay_overwrites
        );
        if let Some(rev) = crate::modules::schedulers::lottery::latest_replay_event() {
            klog_error!(
                "[KERNEL DUMP] lottery_replay seq={} task={} winner_ticket={} total_tickets={} rng_state={:#x}",
                rev.seq,
                rev.task_id.0,
                rev.winner_ticket,
                rev.total_tickets,
                rev.rng_state
            );
        } else {
            klog_error!("[KERNEL DUMP] lottery_replay=none");
        }
    }
    match current_task {
        Some(task_id) => klog_error!("[KERNEL DUMP] current_task={}", task_id),
        None => klog_error!("[KERNEL DUMP] current_task=unavailable"),
    }
}

pub fn fatal_halt(reason: &str) -> ! {
    dump_kernel_state(reason);
    match KernelConfig::panic_action_mode() {
        crate::config::PanicAction::Spin => loop {
            core::hint::spin_loop();
        },
        crate::config::PanicAction::Halt => loop {
            HAL::halt();
        },
    }
}

#[inline(always)]
pub fn idle_once() {
    let runqueue_len = unsafe {
        crate::kernel::cpu_local::CpuLocal::try_get()
            .map(|cpu| cpu.scheduler.lock().runqueue_len())
            .unwrap_or(0)
    };
    let _ = crate::kernel::power::on_idle(runqueue_len);

    match KernelConfig::idle_strategy_mode() {
        crate::config::IdleStrategy::Spin => core::hint::spin_loop(),
        crate::config::IdleStrategy::Halt => HAL::halt(),
    }
}
