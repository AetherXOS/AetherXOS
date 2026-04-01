use super::super::*;

#[inline(always)]
pub(crate) fn runtime_policy_snapshot_contract_holds(snapshot: CoreRuntimePolicySnapshot) -> bool {
    let effective_execution =
        crate::config::KernelConfig::virtualization_effective_execution_profile();
    let effective_governor =
        crate::config::KernelConfig::virtualization_effective_governor_profile();
    snapshot.virtualization_execution_profile == effective_execution.scheduling_class.as_str()
        && snapshot.virtualization_governor_profile == effective_governor.governor_class.as_str()
        && snapshot.virtualization_governor_class
            == current_virtualization_runtime_governor().governor_class
        && snapshot.virtualization_latency_bias
            == current_virtualization_runtime_governor().latency_bias
}
