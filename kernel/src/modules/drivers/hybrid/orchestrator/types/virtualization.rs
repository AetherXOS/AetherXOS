use crate::config::{VirtualizationExecutionClass, VirtualizationGovernorClass};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridVirtualizationReadinessReport {
    pub readiness_score: u8,
    pub policy_scope: &'static str,
    pub core_path_scope: &'static str,
    pub advanced_path_scope: &'static str,
    pub execution_class: VirtualizationExecutionClass,
    pub governor_class: VirtualizationGovernorClass,
    pub entry_enabled: bool,
    pub resume_enabled: bool,
    pub trap_dispatch_enabled: bool,
    pub nested_enabled: bool,
    pub time_virtualization_enabled: bool,
    pub device_passthrough_enabled: bool,
    pub snapshot_enabled: bool,
    pub dirty_logging_enabled: bool,
    pub live_migration_enabled: bool,
    pub trap_tracing_enabled: bool,
    pub enabled_feature_count: usize,
    pub runtime_limited_features: usize,
    pub compiletime_limited_features: usize,
    pub fully_disabled_features: usize,
    pub can_launch_guests: bool,
    pub advanced_ops_ready: bool,
    pub blockers: Vec<&'static str>,
    pub release_ready: bool,
}
