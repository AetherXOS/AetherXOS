use crate::hal::common::virt::{
    BACKEND_MODE_BLOCKED, GOVERNOR_BIAS_AGGRESSIVE, GOVERNOR_BIAS_BALANCED, GOVERNOR_BIAS_RELAXED,
    GOVERNOR_CLASS_BACKGROUND_OPTIMIZED, GOVERNOR_CLASS_BALANCED, GOVERNOR_CLASS_EFFICIENCY,
    GOVERNOR_CLASS_LATENCY_FOCUSED, GOVERNOR_CLASS_PERFORMANCE, GOVERNOR_ENERGY_BALANCED,
    GOVERNOR_ENERGY_PERFORMANCE, GOVERNOR_ENERGY_SAVING, RUNTIME_SCHED_LANE_BACKGROUND,
    RUNTIME_SCHED_LANE_LATENCY_CRITICAL,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualizationSchedulerTuning {
    pub threshold_divisor: usize,
    pub threshold_multiplier: usize,
    pub burst_divisor: usize,
    pub burst_multiplier: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualizationRebalanceTuning {
    pub threshold_divisor: usize,
    pub batch_multiplier: usize,
    pub prefer_local_skip_budget_divisor: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualizationPowerTuning {
    pub prefer_active_pstate: bool,
    pub prefer_shallow_idle: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualizationRuntimeGovernor {
    pub governor_class: &'static str,
    pub latency_bias: &'static str,
    pub energy_bias: &'static str,
    pub scheduler: VirtualizationSchedulerTuning,
    pub rebalance: VirtualizationRebalanceTuning,
    pub power: VirtualizationPowerTuning,
}

#[inline(always)]
fn build_governor(
    governor_class: &'static str,
    latency_bias: &'static str,
    energy_bias: &'static str,
    scheduler: VirtualizationSchedulerTuning,
    rebalance: VirtualizationRebalanceTuning,
    power: VirtualizationPowerTuning,
) -> VirtualizationRuntimeGovernor {
    VirtualizationRuntimeGovernor {
        governor_class,
        latency_bias,
        energy_bias,
        scheduler,
        rebalance,
        power,
    }
}

#[inline(always)]
fn scope_limited_feature_counts() -> (usize, usize) {
    let scope = crate::config::KernelConfig::virtualization_policy_scope_profile();
    let states = [
        scope.entry,
        scope.resume,
        scope.trap_dispatch,
        scope.nested,
        scope.time_virtualization,
        scope.device_passthrough,
        scope.snapshot,
        scope.dirty_logging,
        scope.live_migration,
        scope.trap_tracing,
    ];

    let runtime_limited = states
        .iter()
        .filter(|state| **state == "runtime-limited")
        .count();
    let compiletime_limited = states
        .iter()
        .filter(|state| **state == "compiletime-limited")
        .count();
    (runtime_limited, compiletime_limited)
}

#[inline(always)]
fn adapt_governor_by_runtime_signals(
    mut governor: VirtualizationRuntimeGovernor,
) -> VirtualizationRuntimeGovernor {
    let cpu_count = crate::hal::smp::cpu_count().max(1);
    let (runtime_limited, compiletime_limited) = scope_limited_feature_counts();

    // Wider SMP topologies benefit from stronger rebalance pressure and larger migration windows.
    if cpu_count >= 8 {
        governor.scheduler.threshold_divisor = governor.scheduler.threshold_divisor.saturating_add(1);
        governor.rebalance.batch_multiplier = governor.rebalance.batch_multiplier.saturating_add(1);
        governor.rebalance.threshold_divisor = governor.rebalance.threshold_divisor.saturating_add(1);
    }

    // If runtime policy limits are active, reduce migration aggressiveness until features are re-enabled.
    if runtime_limited > 0 {
        governor.scheduler.threshold_divisor = governor.scheduler.threshold_divisor.saturating_sub(1).max(1);
        governor.rebalance.threshold_divisor = governor.rebalance.threshold_divisor.saturating_sub(1).max(1);
        governor.rebalance.batch_multiplier = governor.rebalance.batch_multiplier.saturating_sub(1).max(1);
    }

    // Compile-time limits imply missing capabilities, so keep policy conservative to avoid oscillation.
    if compiletime_limited > 0 {
        governor.scheduler.burst_multiplier = 1;
        governor.rebalance.batch_multiplier = 1;
        governor.power.prefer_active_pstate = false;
    }

    governor
}

#[inline(always)]
pub fn virtualization_runtime_governor(
    execution_profile: &'static str,
    scheduler_lane: &'static str,
    selected_mode: &'static str,
    dispatch_class: &'static str,
) -> VirtualizationRuntimeGovernor {
    let governor_profile = crate::config::KernelConfig::virtualization_effective_governor_profile();
    if matches!(
        governor_profile.governor_class,
        crate::config::VirtualizationGovernorClass::Performance
    ) {
        return adapt_governor_by_runtime_signals(build_governor(
            GOVERNOR_CLASS_PERFORMANCE,
            GOVERNOR_BIAS_AGGRESSIVE,
            GOVERNOR_ENERGY_PERFORMANCE,
            VirtualizationSchedulerTuning {
                threshold_divisor: 2,
                threshold_multiplier: 1,
                burst_divisor: 2,
                burst_multiplier: 1,
            },
            VirtualizationRebalanceTuning {
                threshold_divisor: 2,
                batch_multiplier: 2,
                prefer_local_skip_budget_divisor: 2,
            },
            VirtualizationPowerTuning {
                prefer_active_pstate: true,
                prefer_shallow_idle: true,
            },
        ));
    }
    if matches!(
        governor_profile.governor_class,
        crate::config::VirtualizationGovernorClass::Efficiency
    ) {
        return adapt_governor_by_runtime_signals(build_governor(
            GOVERNOR_CLASS_EFFICIENCY,
            GOVERNOR_BIAS_RELAXED,
            GOVERNOR_ENERGY_SAVING,
            VirtualizationSchedulerTuning {
                threshold_divisor: 1,
                threshold_multiplier: 2,
                burst_divisor: 1,
                burst_multiplier: 2,
            },
            VirtualizationRebalanceTuning {
                threshold_divisor: 1,
                batch_multiplier: 1,
                prefer_local_skip_budget_divisor: 1,
            },
            VirtualizationPowerTuning {
                prefer_active_pstate: false,
                prefer_shallow_idle: false,
            },
        ));
    }

    let latency_critical = execution_profile.eq_ignore_ascii_case("LatencyCritical")
        || scheduler_lane == RUNTIME_SCHED_LANE_LATENCY_CRITICAL
        || dispatch_class == "low-latency";
    let background = execution_profile.eq_ignore_ascii_case("Background")
        || scheduler_lane == RUNTIME_SCHED_LANE_BACKGROUND
        || selected_mode == BACKEND_MODE_BLOCKED;

    if latency_critical {
        adapt_governor_by_runtime_signals(build_governor(
            GOVERNOR_CLASS_LATENCY_FOCUSED,
            GOVERNOR_BIAS_AGGRESSIVE,
            GOVERNOR_ENERGY_PERFORMANCE,
            VirtualizationSchedulerTuning {
                threshold_divisor: 2,
                threshold_multiplier: 1,
                burst_divisor: 2,
                burst_multiplier: 1,
            },
            VirtualizationRebalanceTuning {
                threshold_divisor: 2,
                batch_multiplier: 2,
                prefer_local_skip_budget_divisor: 2,
            },
            VirtualizationPowerTuning {
                prefer_active_pstate: true,
                prefer_shallow_idle: true,
            },
        ))
    } else if background {
        adapt_governor_by_runtime_signals(build_governor(
            GOVERNOR_CLASS_BACKGROUND_OPTIMIZED,
            GOVERNOR_BIAS_RELAXED,
            GOVERNOR_ENERGY_SAVING,
            VirtualizationSchedulerTuning {
                threshold_divisor: 1,
                threshold_multiplier: 2,
                burst_divisor: 1,
                burst_multiplier: 2,
            },
            VirtualizationRebalanceTuning {
                threshold_divisor: 1,
                batch_multiplier: 1,
                prefer_local_skip_budget_divisor: 1,
            },
            VirtualizationPowerTuning {
                prefer_active_pstate: false,
                prefer_shallow_idle: false,
            },
        ))
    } else {
        adapt_governor_by_runtime_signals(build_governor(
            GOVERNOR_CLASS_BALANCED,
            GOVERNOR_BIAS_BALANCED,
            GOVERNOR_ENERGY_BALANCED,
            VirtualizationSchedulerTuning {
                threshold_divisor: 1,
                threshold_multiplier: 1,
                burst_divisor: 1,
                burst_multiplier: 1,
            },
            VirtualizationRebalanceTuning {
                threshold_divisor: 1,
                batch_multiplier: 1,
                prefer_local_skip_budget_divisor: 1,
            },
            VirtualizationPowerTuning {
                prefer_active_pstate: false,
                prefer_shallow_idle: true,
            },
        ))
    }
}

#[inline(always)]
pub fn current_virtualization_runtime_governor() -> VirtualizationRuntimeGovernor {
    let status = crate::hal::platform::status();
    virtualization_runtime_governor(
        status.virt_runtime_execution_profile,
        status.virt_runtime_scheduler_lane,
        status.virt_runtime_selected_mode,
        status.virt_runtime_dispatch_class,
    )
}

#[inline(always)]
pub fn virtualization_scheduler_tuning(
    execution_profile: &'static str,
    scheduler_lane: &'static str,
    dispatch_class: &'static str,
    selected_mode: &'static str,
) -> VirtualizationSchedulerTuning {
    virtualization_runtime_governor(
        execution_profile,
        scheduler_lane,
        selected_mode,
        dispatch_class,
    )
    .scheduler
}

#[inline(always)]
pub fn current_virtualization_scheduler_tuning() -> VirtualizationSchedulerTuning {
    current_virtualization_runtime_governor().scheduler
}

#[inline(always)]
pub fn virtualization_rebalance_tuning(
    execution_profile: &'static str,
    scheduler_lane: &'static str,
    selected_mode: &'static str,
    dispatch_class: &'static str,
) -> VirtualizationRebalanceTuning {
    virtualization_runtime_governor(
        execution_profile,
        scheduler_lane,
        selected_mode,
        dispatch_class,
    )
    .rebalance
}

#[inline(always)]
pub fn current_virtualization_rebalance_tuning() -> VirtualizationRebalanceTuning {
    current_virtualization_runtime_governor().rebalance
}

#[inline(always)]
pub fn virtualization_power_tuning(
    execution_profile: &'static str,
    scheduler_lane: &'static str,
    selected_mode: &'static str,
    dispatch_class: &'static str,
) -> VirtualizationPowerTuning {
    virtualization_runtime_governor(
        execution_profile,
        scheduler_lane,
        selected_mode,
        dispatch_class,
    )
    .power
}

#[inline(always)]
pub fn current_virtualization_power_tuning() -> VirtualizationPowerTuning {
    current_virtualization_runtime_governor().power
}
