use super::enums::CoreRuntimePolicyPreset;

#[derive(Debug, Clone, Copy)]
pub struct RuntimePolicyContractReport {
    pub checks: u32,
    pub failures: u32,
    pub last_error_code: u32,
}

impl RuntimePolicyContractReport {
    #[inline(always)]
    pub const fn passed(self) -> bool {
        self.failures == 0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CoreRuntimePolicySnapshot {
    pub active_preset: CoreRuntimePolicyPreset,
    pub set_calls: u64,
    pub apply_calls: u64,
    pub drift_samples: u64,
    pub drift_events: u64,
    pub drift_reapply_calls: u64,
    pub drift_reapply_suppressed_cooldown: u64,
    pub drift_sample_interval_ticks: u64,
    pub drift_reapply_cooldown_ticks: u64,
    pub last_reapply_tick: u64,
    pub last_drift_reason: u8,
    pub last_driver_wait_timeout_delta: u64,
    pub virtualization_execution_profile: &'static str,
    pub virtualization_governor_profile: &'static str,
    pub virtualization_governor_class: &'static str,
    pub virtualization_latency_bias: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct CoreRuntimePolicyDriftReport {
    pub sampled_tick: u64,
    pub preset: CoreRuntimePolicyPreset,
    pub drifted: bool,
    pub reason: u8,
    pub reason_name: &'static str,
    pub pressure_class: crate::kernel::pressure::CorePressureClass,
    pub scheduler_class: crate::kernel::pressure::SchedulerPressureClass,
    pub rt_starvation_alert: bool,
    pub network_slo_breaches: u8,
    pub vfs_slo_breaches: u8,
    pub driver_wait_timeout_delta: u64,
    pub virtualization_execution_profile: &'static str,
    pub virtualization_governor_profile: &'static str,
    pub virtualization_governor_class: &'static str,
    pub virtualization_latency_bias: &'static str,
    pub reapply_attempted: bool,
    pub reapply_executed: bool,
    pub reapply_suppressed_by_cooldown: bool,
}
