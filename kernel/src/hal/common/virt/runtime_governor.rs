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
        return build_governor(
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
        );
    }
    if matches!(
        governor_profile.governor_class,
        crate::config::VirtualizationGovernorClass::Efficiency
    ) {
        return build_governor(
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
        );
    }

    let latency_critical = execution_profile.eq_ignore_ascii_case("LatencyCritical")
        || scheduler_lane == RUNTIME_SCHED_LANE_LATENCY_CRITICAL
        || dispatch_class == "low-latency";
    let background = execution_profile.eq_ignore_ascii_case("Background")
        || scheduler_lane == RUNTIME_SCHED_LANE_BACKGROUND
        || selected_mode == BACKEND_MODE_BLOCKED;

    if latency_critical {
        build_governor(
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
        )
    } else if background {
        build_governor(
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
        )
    } else {
        build_governor(
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
        )
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
