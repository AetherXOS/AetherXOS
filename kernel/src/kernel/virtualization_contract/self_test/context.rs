use crate::config::KernelConfig;
use crate::hal::common::virt::{
    current_virtualization_power_tuning, current_virtualization_rebalance_tuning,
    current_virtualization_runtime_governor, current_virtualization_scheduler_tuning,
};
use crate::hal::platform::PlatformStatus;

pub(super) struct VirtualizationContractContext {
    pub(super) status: PlatformStatus,
    pub(super) governor: crate::hal::common::virt::VirtualizationRuntimeGovernor,
    pub(super) scheduler_tuning: crate::hal::common::virt::VirtualizationSchedulerTuning,
    pub(super) rebalance_tuning: crate::hal::common::virt::VirtualizationRebalanceTuning,
    pub(super) power_tuning: crate::hal::common::virt::VirtualizationPowerTuning,
    pub(super) effective_execution: crate::config::VirtualizationExecutionProfile,
    pub(super) effective_governor: crate::config::VirtualizationGovernorProfile,
    pub(super) policy_snapshot: crate::kernel::policy::CoreRuntimePolicySnapshot,
}

pub(super) fn current_virtualization_contract_context() -> VirtualizationContractContext {
    let status = crate::hal::platform::status();

    let governor = current_virtualization_runtime_governor();

    let scheduler_tuning = current_virtualization_scheduler_tuning();

    let rebalance_tuning = current_virtualization_rebalance_tuning();

    let power_tuning = current_virtualization_power_tuning();

    let effective_execution = KernelConfig::virtualization_effective_execution_profile();
    let effective_governor = KernelConfig::virtualization_effective_governor_profile();

    let policy_snapshot = crate::kernel::policy::runtime_policy_snapshot();

    VirtualizationContractContext {
        status,
        governor,
        scheduler_tuning,
        rebalance_tuning,
        power_tuning,
        effective_execution,
        effective_governor,
        policy_snapshot,
    }
}
