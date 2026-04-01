use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

use crate::config::{AffinityPolicy, KernelConfig};
use crate::hal::common::virt::current_virtualization_rebalance_tuning;
#[cfg(test)]
use crate::hal::common::virt::virtualization_rebalance_tuning;
#[cfg(test)]
use crate::hal::common::virt::VirtualizationRebalanceTuning;
use crate::interfaces::task::CpuId;
use crate::interfaces::Scheduler;
use crate::kernel::cpu_local::CpuLocal;

static GLOBAL_TICK: AtomicU64 = AtomicU64::new(0);
static REBALANCE_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static REBALANCE_MOVED: AtomicU64 = AtomicU64::new(0);
static REBALANCE_AFFINITY_SKIPS: AtomicU64 = AtomicU64::new(0);
static REBALANCE_PREFER_LOCAL_SKIPS: AtomicU64 = AtomicU64::new(0);
static REBALANCE_PREFER_LOCAL_FORCED_MOVES: AtomicU64 = AtomicU64::new(0);
static REBALANCE_IMBALANCE_BIN_LT2: AtomicU64 = AtomicU64::new(0);
static REBALANCE_IMBALANCE_BIN_2_3: AtomicU64 = AtomicU64::new(0);
static REBALANCE_IMBALANCE_BIN_4_7: AtomicU64 = AtomicU64::new(0);
static REBALANCE_IMBALANCE_BIN_8_15: AtomicU64 = AtomicU64::new(0);
static REBALANCE_IMBALANCE_BIN_GE16: AtomicU64 = AtomicU64::new(0);
static REBALANCE_IMBALANCE_SEQ: AtomicU64 = AtomicU64::new(0);

const IMBALANCE_WINDOW: usize = crate::generated_consts::GOVERNOR_LOAD_BALANCE_PERCENTILE_WINDOW;
static REBALANCE_IMBALANCE_RING: Mutex<[u32; IMBALANCE_WINDOW]> = Mutex::new([0; IMBALANCE_WINDOW]);

#[derive(Debug, Clone, Copy)]
pub struct RebalanceStats {
    pub attempts: u64,
    pub moved: u64,
    pub affinity_skips: u64,
    pub prefer_local_skips: u64,
    pub prefer_local_forced_moves: u64,
    pub imbalance_lt2: u64,
    pub imbalance_2_3: u64,
    pub imbalance_4_7: u64,
    pub imbalance_8_15: u64,
    pub imbalance_ge16: u64,
    pub imbalance_samples: usize,
    pub imbalance_p50: usize,
    pub imbalance_p90: usize,
    pub imbalance_p99: usize,
}

#[derive(Debug, Clone, Copy)]
struct RebalancePercentiles {
    samples: usize,
    p50: usize,
    p90: usize,
    p99: usize,
}

#[inline(always)]
pub fn stats_snapshot() -> RebalanceStats {
    let p = imbalance_percentiles_snapshot();
    RebalanceStats {
        attempts: REBALANCE_ATTEMPTS.load(Ordering::Relaxed),
        moved: REBALANCE_MOVED.load(Ordering::Relaxed),
        affinity_skips: REBALANCE_AFFINITY_SKIPS.load(Ordering::Relaxed),
        prefer_local_skips: REBALANCE_PREFER_LOCAL_SKIPS.load(Ordering::Relaxed),
        prefer_local_forced_moves: REBALANCE_PREFER_LOCAL_FORCED_MOVES.load(Ordering::Relaxed),
        imbalance_lt2: REBALANCE_IMBALANCE_BIN_LT2.load(Ordering::Relaxed),
        imbalance_2_3: REBALANCE_IMBALANCE_BIN_2_3.load(Ordering::Relaxed),
        imbalance_4_7: REBALANCE_IMBALANCE_BIN_4_7.load(Ordering::Relaxed),
        imbalance_8_15: REBALANCE_IMBALANCE_BIN_8_15.load(Ordering::Relaxed),
        imbalance_ge16: REBALANCE_IMBALANCE_BIN_GE16.load(Ordering::Relaxed),
        imbalance_samples: p.samples,
        imbalance_p50: p.p50,
        imbalance_p90: p.p90,
        imbalance_p99: p.p99,
    }
}

#[inline(always)]
fn percentile_index(n: usize, pct: usize) -> usize {
    if n <= 1 {
        0
    } else {
        ((n - 1) * pct) / 100
    }
}

fn imbalance_percentiles_snapshot() -> RebalancePercentiles {
    let seq = REBALANCE_IMBALANCE_SEQ.load(Ordering::Relaxed) as usize;
    let total = core::cmp::min(seq, IMBALANCE_WINDOW);
    if total == 0 {
        return RebalancePercentiles {
            samples: 0,
            p50: 0,
            p90: 0,
            p99: 0,
        };
    }

    let oldest = if total == IMBALANCE_WINDOW {
        seq % IMBALANCE_WINDOW
    } else {
        0
    };
    let ring = REBALANCE_IMBALANCE_RING.lock();
    let mut samples = Vec::with_capacity(total);
    let mut cursor = oldest;
    for _ in 0..total {
        samples.push(ring[cursor] as usize);
        cursor = (cursor + 1) % IMBALANCE_WINDOW;
    }
    drop(ring);
    samples.sort_unstable();

    let p50 = samples[percentile_index(samples.len(), 50)];
    let p90 = samples[percentile_index(samples.len(), 90)];
    let p99 = samples[percentile_index(samples.len(), 99)];
    RebalancePercentiles {
        samples: samples.len(),
        p50,
        p90,
        p99,
    }
}

#[inline(always)]
fn record_imbalance_histogram(imbalance: usize) {
    let seq = REBALANCE_IMBALANCE_SEQ
        .fetch_add(1, Ordering::Relaxed)
        .saturating_add(1);
    let idx = (seq as usize) % IMBALANCE_WINDOW;
    let value = core::cmp::min(imbalance, u32::MAX as usize) as u32;
    REBALANCE_IMBALANCE_RING.lock()[idx] = value;

    match imbalance {
        0..=1 => {
            REBALANCE_IMBALANCE_BIN_LT2.fetch_add(1, Ordering::Relaxed);
        }
        2..=3 => {
            REBALANCE_IMBALANCE_BIN_2_3.fetch_add(1, Ordering::Relaxed);
        }
        4..=7 => {
            REBALANCE_IMBALANCE_BIN_4_7.fetch_add(1, Ordering::Relaxed);
        }
        8..=15 => {
            REBALANCE_IMBALANCE_BIN_8_15.fetch_add(1, Ordering::Relaxed);
        }
        _ => {
            REBALANCE_IMBALANCE_BIN_GE16.fetch_add(1, Ordering::Relaxed);
        }
    }
}

#[inline(always)]
fn rebalance_threshold(tuning: crate::hal::common::virt::VirtualizationRebalanceTuning) -> usize {
    KernelConfig::rebalance_imbalance_threshold()
        .saturating_div(tuning.threshold_divisor.max(1))
        .max(1)
}

#[inline(always)]
fn rebalance_batch_size(tuning: crate::hal::common::virt::VirtualizationRebalanceTuning) -> usize {
    KernelConfig::rebalance_batch_size()
        .saturating_mul(tuning.batch_multiplier.max(1))
        .max(1)
}

#[inline(always)]
fn prefer_local_skip_budget(
    tuning: crate::hal::common::virt::VirtualizationRebalanceTuning,
) -> usize {
    KernelConfig::rebalance_prefer_local_skip_budget()
        .saturating_div(tuning.prefer_local_skip_budget_divisor.max(1))
}

#[inline(always)]
pub fn should_attempt_local_steal() -> bool {
    match KernelConfig::affinity_policy_mode() {
        AffinityPolicy::Spread => true,
        AffinityPolicy::PreferLocal => KernelConfig::is_work_stealing_enabled(),
    }
}

pub fn maybe_periodic_rebalance() {
    if !KernelConfig::is_periodic_rebalance_enabled() {
        return;
    }

    let interval = KernelConfig::rebalance_interval_ticks();

    let tick = GLOBAL_TICK.fetch_add(1, Ordering::Relaxed) + 1;
    if tick % interval != 0 {
        return;
    }

    rebalance_once();
}

pub fn try_steal_for_cpu(
    target_cpu: &'static CpuLocal,
) -> Option<alloc::sync::Arc<crate::kernel::sync::IrqSafeMutex<crate::interfaces::KernelTask>>> {
    if !should_attempt_local_steal() {
        return None;
    }

    let cpus = crate::hal::smp::CPUS.lock();
    for other_cpu in cpus.iter() {
        if other_cpu.cpu_id == target_cpu.cpu_id {
            continue;
        }

        let stolen = {
            let mut src = other_cpu.scheduler.lock();
            src.steal_task()
        };

        let Some(task_arc) = stolen else { continue };

        if !task_arc.lock().can_run_on_cpu_id(target_cpu.cpu_id) {
            REBALANCE_AFFINITY_SKIPS.fetch_add(1, Ordering::Relaxed);
            other_cpu.scheduler.lock().add_task(task_arc);
            continue;
        }

        return Some(task_arc);
    }

    None
}

#[inline(always)]
fn task_can_run_on_cpu(
    task: &alloc::sync::Arc<crate::kernel::sync::IrqSafeMutex<crate::interfaces::KernelTask>>,
    cpu_id: CpuId,
) -> bool {
    if !KernelConfig::is_affinity_enforcement_enabled() {
        return true;
    }
    task.lock().can_run_on_cpu_id(cpu_id)
}

fn rebalance_once() {
    REBALANCE_ATTEMPTS.fetch_add(1, Ordering::Relaxed);

    let tuning = current_virtualization_rebalance_tuning();
    let threshold = rebalance_threshold(tuning);
    let batch = rebalance_batch_size(tuning);

    let (source_cpu, source_load, target_cpu, target_load) = {
        let cpus = crate::hal::smp::CPUS.lock();
        if cpus.len() < 2 {
            return;
        }

        let mut busiest: Option<(&'static CpuLocal, usize)> = None;
        let mut idlest: Option<(&'static CpuLocal, usize)> = None;

        for cpu in cpus.iter() {
            let load = cpu.scheduler.lock().cpu_load();

            if busiest.as_ref().map(|(_, l)| load > *l).unwrap_or(true) {
                busiest = Some((*cpu, load));
            }

            if idlest.as_ref().map(|(_, l)| load < *l).unwrap_or(true) {
                idlest = Some((*cpu, load));
            }
        }

        let Some((b_cpu, b_load)) = busiest else {
            return;
        };
        let Some((i_cpu, i_load)) = idlest else {
            return;
        };

        (b_cpu, b_load, i_cpu, i_load)
    };

    if source_cpu.cpu_id == target_cpu.cpu_id {
        return;
    }

    let imbalance = source_load.saturating_sub(target_load);
    record_imbalance_histogram(imbalance);

    if source_load <= target_load.saturating_add(threshold) {
        return;
    }

    let mut moved = 0usize;
    let mut prefer_local_skips = 0usize;
    let prefer_local_skip_budget = prefer_local_skip_budget(tuning);
    for _ in 0..batch {
        let stolen = {
            let mut src = source_cpu.scheduler.lock();
            src.steal_task()
        };

        let Some(task_arc) = stolen else {
            break;
        };

        if !task_can_run_on_cpu(&task_arc, target_cpu.cpu_id) {
            REBALANCE_AFFINITY_SKIPS.fetch_add(1, Ordering::Relaxed);
            source_cpu.scheduler.lock().add_task(task_arc);
            continue;
        }

        if KernelConfig::affinity_policy_mode() == AffinityPolicy::PreferLocal {
            let (is_any, preferred) = {
                let t = task_arc.lock();
                (t.preferred_cpu.is_any(), t.preferred_cpu)
            };
            if !is_any && preferred != target_cpu.cpu_id {
                if prefer_local_skips < prefer_local_skip_budget {
                    REBALANCE_AFFINITY_SKIPS.fetch_add(1, Ordering::Relaxed);
                    REBALANCE_PREFER_LOCAL_SKIPS.fetch_add(1, Ordering::Relaxed);
                    prefer_local_skips = prefer_local_skips.saturating_add(1);
                    source_cpu.scheduler.lock().add_task(task_arc);
                    continue;
                }
                REBALANCE_PREFER_LOCAL_FORCED_MOVES.fetch_add(1, Ordering::Relaxed);
            }
        }

        target_cpu.scheduler.lock().add_task(task_arc);
        moved += 1;
    }

    if moved > 0 {
        REBALANCE_MOVED.fetch_add(moved as u64, Ordering::Relaxed);
    }

    if moved > 0 && KernelConfig::is_scheduler_trace_enabled() {
        crate::klog_trace!(
            "rebalance moved={} from cpu={} to cpu={} load {}->{} policy={}",
            moved,
            source_cpu.cpu_id,
            target_cpu.cpu_id,
            source_load,
            target_load,
            KernelConfig::affinity_policy()
        );
    }
}

#[cfg(test)]
#[path = "load_balance/tests.rs"]
mod tests;
