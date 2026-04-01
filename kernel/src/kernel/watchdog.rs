use core::sync::atomic::{AtomicU64, Ordering};

use crate::config::KernelConfig;
use crate::hal::common::virt::current_virtualization_runtime_governor;
use crate::kernel::cpu_local::CpuLocal;

static GLOBAL_TICK: AtomicU64 = AtomicU64::new(0);
static WATCHDOG_CHECKS: AtomicU64 = AtomicU64::new(0);
static WATCHDOG_STALL_DETECTIONS: AtomicU64 = AtomicU64::new(0);
static WATCHDOG_LAST_STALLED_CPU: AtomicU64 = AtomicU64::new(u64::MAX);
static WATCHDOG_HARD_PANIC_TICKS: AtomicU64 = AtomicU64::new(0);
static WATCHDOG_HARD_PANIC_TRIGGERED: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
pub struct WatchdogStats {
    pub global_tick: u64,
    pub checks: u64,
    pub stall_detections: u64,
    pub last_stalled_cpu: u64,
    pub hard_panic_ticks: u64,
    pub hard_panic_triggered: u64,
}

#[inline(always)]
pub fn stats() -> WatchdogStats {
    WatchdogStats {
        global_tick: GLOBAL_TICK.load(Ordering::Relaxed),
        checks: WATCHDOG_CHECKS.load(Ordering::Relaxed),
        stall_detections: WATCHDOG_STALL_DETECTIONS.load(Ordering::Relaxed),
        last_stalled_cpu: WATCHDOG_LAST_STALLED_CPU.load(Ordering::Relaxed),
        hard_panic_ticks: WATCHDOG_HARD_PANIC_TICKS.load(Ordering::Relaxed),
        hard_panic_triggered: WATCHDOG_HARD_PANIC_TRIGGERED.load(Ordering::Relaxed),
    }
}

#[inline(always)]
fn governor_adjusted_hard_stall_ticks(base_ticks: u64, latency_bias: &'static str) -> u64 {
    match latency_bias {
        "aggressive" => base_ticks.saturating_sub((base_ticks / 4).max(1)).max(1),
        "relaxed" => base_ticks.saturating_add((base_ticks / 4).max(1)),
        _ => base_ticks.max(1),
    }
}

#[inline(always)]
fn hard_stall_ticks() -> u64 {
    let hard_stall_ns = KernelConfig::watchdog_hard_stall_ns();
    let slice_ns = KernelConfig::time_slice();
    let base_ticks = if slice_ns == 0 {
        KernelConfig::soft_watchdog_stall_ticks()
    } else {
        let base = hard_stall_ns / slice_ns;
        if hard_stall_ns % slice_ns == 0 {
            base
        } else {
            base.saturating_add(1)
        }
    };
    let soft = KernelConfig::soft_watchdog_stall_ticks();
    let bounded = if soft == 0 {
        base_ticks.max(1)
    } else {
        base_ticks.max(soft)
    };
    let governor = current_virtualization_runtime_governor();
    governor_adjusted_hard_stall_ticks(bounded, governor.latency_bias)
}

#[cfg(test)]
#[path = "watchdog/tests.rs"]
mod tests;

static LOAD_1: AtomicU64 = AtomicU64::new(0);
static LOAD_5: AtomicU64 = AtomicU64::new(0);
static LOAD_15: AtomicU64 = AtomicU64::new(0);

#[inline(always)]
pub fn on_timer_tick(cpu: &'static CpuLocal) {
    let tick = GLOBAL_TICK.fetch_add(1, Ordering::Relaxed) + 1;
    WATCHDOG_CHECKS.fetch_add(1, Ordering::Relaxed);
    cpu.heartbeat_tick.store(tick, Ordering::Relaxed);

    #[cfg(all(feature = "process_abstraction", feature = "posix_mman"))]
    if let Some(process) = crate::kernel::launch::current_process_arc() {
        let _ = process.refresh_linux_runtime_vvar();
    }

    let slice_ns = KernelConfig::time_slice();
    let hz = if slice_ns > 0 {
        1_000_000_000 / slice_ns
    } else {
        100
    };
    let interval = 5 * hz;
    if interval > 0 && tick % interval == 0 {
        update_load_avg();
    }

    if !KernelConfig::is_soft_watchdog_enabled() {
        return;
    }

    let stall = KernelConfig::soft_watchdog_stall_ticks();
    if stall == 0 || tick % stall != 0 {
        return;
    }

    let hard_stall = hard_stall_ticks();
    WATCHDOG_HARD_PANIC_TICKS.store(hard_stall, Ordering::Relaxed);

    let cpus = crate::hal::smp::CPUS.lock();
    for other in cpus.iter() {
        if other.cpu_id == cpu.cpu_id {
            continue;
        }

        let peer_tick = other.heartbeat_tick.load(Ordering::Relaxed);
        let lag = tick.saturating_sub(peer_tick);
        if lag > stall {
            WATCHDOG_STALL_DETECTIONS.fetch_add(1, Ordering::Relaxed);
            WATCHDOG_LAST_STALLED_CPU.store(other.cpu_id.0 as u64, Ordering::Relaxed);
            crate::kernel::crash_log::record(
                crate::kernel::crash_log::EVENT_SOFT_WATCHDOG_STALL,
                0,
                other.cpu_id.0 as u64,
                lag,
            );
            crate::klog_error!(
                "soft watchdog stall detected current_cpu={} stalled_cpu={} tick={} peer_tick={} stall={}",
                cpu.cpu_id,
                other.cpu_id,
                tick,
                peer_tick,
                stall
            );

            if lag >= hard_stall {
                WATCHDOG_HARD_PANIC_TRIGGERED.fetch_add(1, Ordering::Relaxed);
                crate::kernel::crash_log::record(
                    crate::kernel::crash_log::EVENT_HARD_WATCHDOG_STALL,
                    0,
                    other.cpu_id.0 as u64,
                    lag,
                );
                crate::klog_error!(
                    "watchdog hard panic current_cpu={} stalled_cpu={} lag={} hard_stall={} (~5s)",
                    cpu.cpu_id,
                    other.cpu_id,
                    lag,
                    hard_stall
                );
                crate::kernel::fatal_halt("nmi_watchdog_emulation");
            }
            match KernelConfig::soft_watchdog_action_mode() {
                crate::config::WatchdogAction::Halt => {
                    crate::kernel::fatal_halt("soft_watchdog");
                }
                crate::config::WatchdogAction::LogOnly => {
                    crate::klog_warn!(
                        "soft watchdog recovery action=LogOnly current_cpu={} stalled_cpu={} lag={} stall={}",
                        cpu.cpu_id,
                        other.cpu_id,
                        lag,
                        stall
                    );
                }
            }
        }
    }
}

fn update_load_avg() {
    #[cfg(feature = "posix_process")]
    let active = crate::modules::posix::process::process_count() as u64;
    #[cfg(not(feature = "posix_process"))]
    let active = 1u64;

    let active_fixed = active << 16;

    update_ema(&LOAD_1, active_fixed, 1);
    update_ema(&LOAD_5, active_fixed, 5);
    update_ema(&LOAD_15, active_fixed, 15);
}

fn update_ema(ema: &AtomicU64, active: u64, mins: u64) {
    let old = ema.load(Ordering::Relaxed);
    let factor = mins * 12;
    let next = if factor > 0 {
        (old * (factor - 1) + active) / factor
    } else {
        active
    };
    ema.store(next, Ordering::Relaxed);
}

pub fn load_avg() -> [u64; 3] {
    [
        LOAD_1.load(Ordering::Relaxed),
        LOAD_5.load(Ordering::Relaxed),
        LOAD_15.load(Ordering::Relaxed),
    ]
}

#[inline(always)]
pub fn global_tick() -> u64 {
    GLOBAL_TICK.load(Ordering::Relaxed)
}

/// Lightweight tick called by the watchdog kernel service daemon.
/// Advances the global tick counter and updates load averages, but does not
/// perform the per-CPU stall checks (those happen in `on_timer_tick`).
#[inline(always)]
pub fn tick() {
    GLOBAL_TICK.fetch_add(1, Ordering::Relaxed);
    #[cfg(all(feature = "process_abstraction", feature = "posix_mman"))]
    crate::kernel::launch::refresh_all_linux_runtime_vvar();
    update_load_avg();
}
