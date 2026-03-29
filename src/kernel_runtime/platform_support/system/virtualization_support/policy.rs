pub(crate) struct VirtualizationPolicyLogSnapshot {
    pub runtime: hypercore::config::VirtualizationRuntimeProfile,
    pub cargo: hypercore::config::VirtualizationRuntimeProfile,
    pub effective: hypercore::config::VirtualizationRuntimeProfile,
    pub runtime_execution_profile: &'static str,
    pub cargo_execution_profile: &'static str,
    pub effective_execution_profile: &'static str,
    pub runtime_governor_profile: &'static str,
    pub cargo_governor_profile: &'static str,
    pub effective_governor_profile: &'static str,
}

pub(crate) fn current_virtualization_policy_log_snapshot() -> VirtualizationPolicyLogSnapshot {
    let policy = hypercore::config::KernelConfig::virtualization_policy_profile();
    let execution = hypercore::config::KernelConfig::virtualization_execution_policy_profile();
    let governor = hypercore::config::KernelConfig::virtualization_governor_policy_profile();

    VirtualizationPolicyLogSnapshot {
        runtime: policy.runtime,
        cargo: policy.cargo,
        effective: policy.effective,
        runtime_execution_profile: execution.runtime.scheduling_class.as_str(),
        cargo_execution_profile: execution.cargo.scheduling_class.as_str(),
        effective_execution_profile: execution.effective.scheduling_class.as_str(),
        runtime_governor_profile: governor.runtime.governor_class.as_str(),
        cargo_governor_profile: governor.cargo.governor_class.as_str(),
        effective_governor_profile: governor.effective.governor_class.as_str(),
    }
}
