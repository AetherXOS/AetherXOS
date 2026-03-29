use crate::hal::common::virt::{
    current_virtualization_power_tuning, current_virtualization_rebalance_tuning,
    current_virtualization_runtime_governor, current_virtualization_scheduler_tuning,
    RUNTIME_DISPATCH_BALANCED, RUNTIME_DISPATCH_CONSERVATIVE, RUNTIME_DISPATCH_LATENCY_SAFE,
    RUNTIME_DISPATCH_WINDOW_ADAPTIVE, RUNTIME_DISPATCH_WINDOW_HOLD, RUNTIME_DISPATCH_WINDOW_SHORT,
    RUNTIME_SCHED_LANE_BACKGROUND, RUNTIME_SCHED_LANE_BALANCED,
    RUNTIME_SCHED_LANE_LATENCY_CRITICAL,
};
use crate::kernel::virtualization_contract::{
    execution_profile_matches_status, expected_runtime_governor_class,
    virtualization_dispatch_contract_holds, virtualization_governor_bias_contract_holds,
};

#[derive(Debug, Clone, Copy)]
pub struct SchedulerContractReport {
    pub checks: u32,
    pub failures: u32,
    pub last_error_code: u32,
}

impl SchedulerContractReport {
    #[inline(always)]
    pub const fn passed(self) -> bool {
        self.failures == 0
    }
}

pub fn run_scheduler_contract_self_test() -> SchedulerContractReport {
    let mut checks = 0u32;
    let mut failures = 0u32;
    let mut last_error_code = 0u32;

    macro_rules! check {
        ($code:expr, $cond:expr, $msg:expr) => {{
            checks = checks.saturating_add(1);
            if !($cond) {
                failures = failures.saturating_add(1);
                last_error_code = $code;
                crate::klog_error!("[SCHED CONTRACT] E{}: {}", $code, $msg);
            }
        }};
    }

    check!(
        3001,
        crate::config::KernelConfig::time_slice() > 0,
        "time_slice_ns must be > 0 for scheduler contracts"
    );

    #[cfg(feature = "sched_lottery")]
    {
        check!(
            3002,
            crate::config::KernelConfig::sched_lottery_tickets_per_priority_level() > 0,
            "lottery tickets_per_priority_level must be > 0"
        );
        check!(
            3003,
            crate::config::KernelConfig::sched_lottery_min_tickets_per_task() > 0,
            "lottery min_tickets_per_task must be > 0"
        );

        let high = crate::modules::schedulers::lottery::priority_ticket_count_for_contract(0);
        let mid = crate::modules::schedulers::lottery::priority_ticket_count_for_contract(128);
        let low = crate::modules::schedulers::lottery::priority_ticket_count_for_contract(255);
        check!(
            3004,
            high > mid && mid > low,
            "lottery priority contract expects p0>p128>p255 ticket counts"
        );
    }

    #[cfg(feature = "sched_cfs")]
    {
        let high = crate::modules::schedulers::cfs::priority_weight_for_contract(0);
        let mid = crate::modules::schedulers::cfs::priority_weight_for_contract(128);
        let low = crate::modules::schedulers::cfs::priority_weight_for_contract(255);
        check!(
            3005,
            high > mid && mid > low,
            "cfs priority contract expects p0>p128>p255 weights"
        );
    }

    let status = crate::hal::platform::status();
    let governor = current_virtualization_runtime_governor();
    let scheduler_tuning = current_virtualization_scheduler_tuning();
    let rebalance_tuning = current_virtualization_rebalance_tuning();
    let power_tuning = current_virtualization_power_tuning();
    let effective_execution =
        crate::config::KernelConfig::virtualization_effective_execution_profile();
    let effective_governor =
        crate::config::KernelConfig::virtualization_effective_governor_profile();

    check!(
        3006,
        virtualization_dispatch_contract_holds(
            status.virt_runtime_dispatch_class,
            status.virt_runtime_scheduler_lane,
            status.virt_runtime_preemption_policy,
            status.virt_runtime_dispatch_window,
        ),
        "virtualization dispatch contract expects lane/window to match dispatch and preemption policy"
    );
    check!(
        3007,
        governor.governor_class == status.virt_runtime_governor_class
            && governor.latency_bias == status.virt_runtime_latency_bias
            && governor.energy_bias == status.virt_runtime_energy_bias,
        "virtualization governor contract expects platform status to mirror effective governor"
    );
    check!(
        3008,
        scheduler_tuning == governor.scheduler
            && rebalance_tuning == governor.rebalance
            && power_tuning == governor.power,
        "virtualization tuning helpers must match the effective governor tuning bundle"
    );
    check!(
        3009,
        virtualization_governor_bias_contract_holds(
            status.virt_runtime_governor_class,
            status.virt_runtime_latency_bias,
            status.virt_runtime_energy_bias,
        ),
        "virtualization governor class/bias/energy contract is inconsistent"
    );
    check!(
        3010,
        matches!(
            status.virt_runtime_dispatch_class,
            RUNTIME_DISPATCH_LATENCY_SAFE
                | RUNTIME_DISPATCH_BALANCED
                | RUNTIME_DISPATCH_CONSERVATIVE
        ) && matches!(
            status.virt_runtime_scheduler_lane,
            RUNTIME_SCHED_LANE_LATENCY_CRITICAL
                | RUNTIME_SCHED_LANE_BALANCED
                | RUNTIME_SCHED_LANE_BACKGROUND
        ) && matches!(
            status.virt_runtime_dispatch_window,
            RUNTIME_DISPATCH_WINDOW_SHORT
                | RUNTIME_DISPATCH_WINDOW_ADAPTIVE
                | RUNTIME_DISPATCH_WINDOW_HOLD
        ),
        "virtualization runtime contract expects known dispatch, lane, and window classes"
    );
    check!(
        3011,
        execution_profile_matches_status(
            effective_execution.scheduling_class,
            status.virt_runtime_execution_profile,
        ),
        "virtualization execution contract expects platform execution profile to match effective config"
    );
    check!(
        3012,
        status.virt_runtime_governor_profile == effective_governor.governor_class.as_str()
            && status.virt_runtime_governor_class
                == expected_runtime_governor_class(effective_governor.governor_class),
        "virtualization governor contract expects platform governor profile/class to match effective config"
    );

    if failures == 0 {
        crate::klog_info!("[SCHED CONTRACT] passed checks={}", checks);
    } else {
        crate::klog_error!(
            "[SCHED CONTRACT] failed checks={} failures={} last_error=E{}",
            checks,
            failures,
            last_error_code
        );
    }

    SchedulerContractReport {
        checks,
        failures,
        last_error_code,
    }
}
