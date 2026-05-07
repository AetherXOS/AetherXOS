use crate::modules::drivers::hybrid::orchestrator::HybridVirtualizationReadinessReport;
use crate::config::{VirtualizationExecutionClass, VirtualizationGovernorClass};

pub fn virtualization_readiness_report() -> HybridVirtualizationReadinessReport {
    HybridVirtualizationReadinessReport {
        readiness_score: 0,
        policy_scope: "",
        core_path_scope: "",
        advanced_path_scope: "",
        execution_class: VirtualizationExecutionClass::Balanced,
        governor_class: VirtualizationGovernorClass::Balanced,
        entry_enabled: false,
        resume_enabled: false,
        trap_dispatch_enabled: false,
        nested_enabled: false,
        time_virtualization_enabled: false,
        device_passthrough_enabled: false,
        snapshot_enabled: false,
        dirty_logging_enabled: false,
        live_migration_enabled: false,
        trap_tracing_enabled: false,
        enabled_feature_count: 0,
        runtime_limited_features: 0,
        compiletime_limited_features: 0,
        fully_disabled_features: 0,
        can_launch_guests: false,
        advanced_ops_ready: false,
        blockers: alloc::vec::Vec::new(),
        release_ready: false,
    }
}
