use core::sync::atomic::Ordering;
use crate::config::{AffinityPolicy, KernelConfig};
use crate::hal::common::virt::current_virtualization_rebalance_tuning;
use crate::interfaces::task::CpuId;
use crate::interfaces::Scheduler;
use crate::kernel::cpu_local::CpuLocal;
use super::stats::{
    GLOBAL_TICK, REBALANCE_AFFINITY_SKIPS, REBALANCE_ATTEMPTS, REBALANCE_MOVED,
    REBALANCE_PREFER_LOCAL_FORCED_MOVES, REBALANCE_PREFER_LOCAL_SKIPS,
};
use super::decision::{record_rebalance_decision, RebalanceDecisionReason, should_emit_rebalance_trace};
use super::adaptive::{
    prefer_local_skip_budget, rebalance_batch_size, rebalance_threshold, record_imbalance_histogram,
};

#[inline(always)]
pub fn should_attempt_local_steal() -> bool {
    match KernelConfig::affinity_policy_mode() {
        AffinityPolicy::Spread => true,
        AffinityPolicy::StrictLocal => KernelConfig::is_work_stealing_enabled(),
        AffinityPolicy::Balanced => KernelConfig::is_work_stealing_enabled(),
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

pub fn rebalance_once() {
    REBALANCE_ATTEMPTS.fetch_add(1, Ordering::Relaxed);

    let tuning = current_virtualization_rebalance_tuning();
    let threshold = rebalance_threshold(tuning);
    let batch = rebalance_batch_size(tuning);

    let (source_cpu, source_load, target_cpu, target_load) = {
        let cpus = crate::hal::smp::CPUS.lock();
        if cpus.len() < 2 {
            record_rebalance_decision(
                RebalanceDecisionReason::InsufficientCpus,
                0,
                0,
                0,
                threshold,
                batch,
                0,
            );
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
            record_rebalance_decision(
                RebalanceDecisionReason::NoCandidates,
                0,
                0,
                0,
                threshold,
                batch,
                0,
            );
            return;
        };
        let Some((i_cpu, i_load)) = idlest else {
            record_rebalance_decision(
                RebalanceDecisionReason::NoCandidates,
                0,
                0,
                0,
                threshold,
                batch,
                0,
            );
            return;
        };

        (b_cpu, b_load, i_cpu, i_load)
    };

    if source_cpu.cpu_id == target_cpu.cpu_id {
        record_rebalance_decision(
            RebalanceDecisionReason::SameCpu,
            source_load,
            target_load,
            0,
            threshold,
            batch,
            0,
        );
        return;
    }

    let imbalance = source_load.saturating_sub(target_load);
    record_imbalance_histogram(imbalance);

    if source_load <= target_load.saturating_add(threshold) {
        record_rebalance_decision(
            RebalanceDecisionReason::BelowThreshold,
            source_load,
            target_load,
            imbalance,
            threshold,
            batch,
            0,
        );
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

    let reason = if moved > 0 {
        RebalanceDecisionReason::Rebalanced
    } else {
        RebalanceDecisionReason::NoEligibleTasks
    };
    record_rebalance_decision(
        reason,
        source_load,
        target_load,
        imbalance,
        threshold,
        batch,
        moved,
    );

    if moved > 0 && should_emit_rebalance_trace() {
        crate::klog_trace!(
            "rebalance reason={} moved={} from cpu={} to cpu={} load {}->{} policy={}",
            reason.as_str(),
            moved,
            source_cpu.cpu_id,
            target_cpu.cpu_id,
            source_load,
            target_load,
            KernelConfig::affinity_policy().as_str()
        );
    }
}
