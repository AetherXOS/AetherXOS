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
    VirtualizationContractContext {
        status: crate::hal::platform::status(),
        governor: current_virtualization_runtime_governor(),
        scheduler_tuning: current_virtualization_scheduler_tuning(),
        rebalance_tuning: current_virtualization_rebalance_tuning(),
        power_tuning: current_virtualization_power_tuning(),
        effective_execution: KernelConfig::virtualization_effective_execution_profile(),
        effective_governor: KernelConfig::virtualization_effective_governor_profile(),
        policy_snapshot: crate::kernel::policy::runtime_policy_snapshot(),
    }
}
