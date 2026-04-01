use super::context::VirtualizationContractContext;
use crate::kernel::virtualization_contract::predicates::{
    execution_profile_matches_status, expected_runtime_governor_class,
    virtualization_dispatch_contract_holds, virtualization_effective_execution_contract_holds,
    virtualization_effective_governor_contract_holds, virtualization_governor_bias_contract_holds,
    virtualization_runtime_mode_contract_holds,
};

pub(super) fn run_contract_checks(
    ctx: &VirtualizationContractContext,
    checks: &mut u32,
    failures: &mut u32,
    last_error_code: &mut u32,
) {
    macro_rules! check {
        ($code:expr, $cond:expr, $msg:expr) => {{
            *checks = checks.saturating_add(1);
            if !($cond) {
                *failures = failures.saturating_add(1);
                *last_error_code = $code;
                crate::klog_error!("[VIRT CONTRACT] E{}: {}", $code, $msg);
            }
        }};
    }

    check!(
        5001,
        virtualization_effective_execution_contract_holds(),
        "virtualization effective execution profile contract is inconsistent"
    );
    check!(
        5002,
        virtualization_effective_governor_contract_holds(),
        "virtualization effective governor profile contract is inconsistent"
    );
    check!(
        5003,
        virtualization_dispatch_contract_holds(
            ctx.status.virt_runtime_dispatch_class,
            ctx.status.virt_runtime_scheduler_lane,
            ctx.status.virt_runtime_preemption_policy,
            ctx.status.virt_runtime_dispatch_window,
        ),
        "virtualization dispatch contract expects lane/window to match dispatch and preemption policy"
    );
    check!(
        5004,
        ctx.governor.governor_class == ctx.status.virt_runtime_governor_class
            && ctx.governor.latency_bias == ctx.status.virt_runtime_latency_bias
            && ctx.governor.energy_bias == ctx.status.virt_runtime_energy_bias,
        "platform status must mirror the effective runtime governor"
    );
    check!(
        5005,
        ctx.scheduler_tuning == ctx.governor.scheduler
            && ctx.rebalance_tuning == ctx.governor.rebalance
            && ctx.power_tuning == ctx.governor.power,
        "virtualization tuning helpers must match the effective governor bundle"
    );
    check!(
        5006,
        virtualization_governor_bias_contract_holds(
            ctx.status.virt_runtime_governor_class,
            ctx.status.virt_runtime_latency_bias,
            ctx.status.virt_runtime_energy_bias,
        ),
        "virtualization governor class/bias/energy contract is inconsistent"
    );
    check!(
        5007,
        execution_profile_matches_status(
            ctx.effective_execution.scheduling_class,
            ctx.status.virt_runtime_execution_profile,
        ),
        "platform execution profile must match effective config"
    );
    check!(
        5008,
        ctx.status.virt_runtime_governor_profile == ctx.effective_governor.governor_class.as_str()
            && ctx.status.virt_runtime_governor_class
                == expected_runtime_governor_class(ctx.effective_governor.governor_class),
        "platform governor profile/class must match effective config"
    );
    check!(
        5009,
        ctx.policy_snapshot.virtualization_execution_profile
            == ctx.effective_execution.scheduling_class.as_str()
            && ctx.policy_snapshot.virtualization_governor_profile
                == ctx.effective_governor.governor_class.as_str()
            && ctx.policy_snapshot.virtualization_governor_class == ctx.governor.governor_class
            && ctx.policy_snapshot.virtualization_latency_bias == ctx.governor.latency_bias,
        "runtime policy snapshot must match effective virtualization execution/governor state"
    );
    check!(
        5010,
        virtualization_runtime_mode_contract_holds(
            ctx.status.virt_runtime_selected_mode,
            ctx.status.virt_runtime_operation_class,
            ctx.status.virt_runtime_blocked_by,
            ctx.status.virt_runtime_policy_limited_by,
        ),
        "runtime selected_mode/operation_class/blocked/policy-limited contract is inconsistent"
    );
}
