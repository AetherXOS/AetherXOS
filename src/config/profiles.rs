use super::parsers::{VirtualizationExecutionClass, VirtualizationGovernorClass};
use super::{BoundaryMode, DevFsPolicyProfile, TlsPolicyProfile};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetworkSloRuntimeConfig {
    pub sample_interval: u64,
    pub log_interval_multiplier: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetworkRuntimeProfile {
    pub tls_policy_profile: TlsPolicyProfile,
    pub slo: NetworkSloRuntimeConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerRuntimeProfile {
    pub cfs_min_granularity_ns: u64,
    pub cfs_latency_target_ns: u64,
    pub mlfq_base_slice_ns: u64,
    pub mlfq_boost_interval_ticks: u64,
    pub mlfq_demote_on_slice_exhaustion: bool,
    pub edf_enforce_deadline: bool,
    pub edf_default_relative_deadline_ns: u64,
    pub rt_group_reservation_enabled: bool,
    pub rt_period_ns: u64,
    pub rt_total_utilization_cap_percent: u8,
    pub rt_max_groups: usize,
    pub lottery_tickets_per_priority_level: u64,
    pub lottery_min_tickets_per_task: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriverNetworkRuntimeProfile {
    pub irq_service_budget: usize,
    pub loop_service_budget: usize,
    pub ring_limit: usize,
    pub quarantine_rebind_failures: u64,
    pub quarantine_cooldown_samples: u64,
    pub slo_max_drop_rate_per_mille: u64,
    pub slo_max_tx_ring_utilization_percent: u64,
    pub slo_max_rx_ring_utilization_percent: u64,
    pub slo_max_io_errors: u64,
    pub low_latency_irq_budget_divisor: usize,
    pub low_latency_loop_budget_divisor: usize,
    pub low_latency_ring_limit_divisor: usize,
    pub throughput_irq_budget_multiplier: usize,
    pub throughput_loop_budget_multiplier: usize,
    pub throughput_ring_limit_multiplier: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TelemetryRuntimeProfile {
    pub enabled: bool,
    pub runtime_summary: bool,
    pub virtualization: bool,
    pub platform_lifecycle: bool,
    pub vfs: bool,
    pub network: bool,
    pub ipc: bool,
    pub scheduler: bool,
    pub security: bool,
    pub power: bool,
    pub drivers: bool,
    pub debug_trace: bool,
    pub early_serial_debug: bool,
    pub history_len: usize,
    pub log_level_num: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualizationRuntimeProfile {
    pub telemetry: bool,
    pub platform_lifecycle: bool,
    pub entry: bool,
    pub resume: bool,
    pub trap_dispatch: bool,
    pub nested: bool,
    pub time_virtualization: bool,
    pub device_passthrough: bool,
    pub snapshot: bool,
    pub dirty_logging: bool,
    pub live_migration: bool,
    pub trap_tracing: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualizationExecutionProfile {
    pub scheduling_class: VirtualizationExecutionClass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualizationGovernorProfile {
    pub governor_class: VirtualizationGovernorClass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualizationPolicyProfile {
    pub runtime: VirtualizationRuntimeProfile,
    pub cargo: VirtualizationRuntimeProfile,
    pub effective: VirtualizationRuntimeProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualizationExecutionPolicyProfile {
    pub runtime: VirtualizationExecutionProfile,
    pub cargo: VirtualizationExecutionProfile,
    pub effective: VirtualizationExecutionProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualizationGovernorPolicyProfile {
    pub runtime: VirtualizationGovernorProfile,
    pub cargo: VirtualizationGovernorProfile,
    pub effective: VirtualizationGovernorProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualizationPolicyScopeProfile {
    pub overall: &'static str,
    pub entry: &'static str,
    pub resume: &'static str,
    pub trap_dispatch: &'static str,
    pub nested: &'static str,
    pub time_virtualization: &'static str,
    pub device_passthrough: &'static str,
    pub snapshot: &'static str,
    pub dirty_logging: &'static str,
    pub live_migration: &'static str,
    pub trap_tracing: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VfsRuntimeProfile {
    pub enable_buffered_io: bool,
    pub health_slo_ms: u64,
    pub diskfs_max_path_len: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DevFsRuntimeProfile {
    pub policy_profile: DevFsPolicyProfile,
    pub default_mode: u16,
    pub default_uid: u32,
    pub default_gid: u32,
    pub net_mode: u16,
    pub net_gid: u32,
    pub storage_mode: u16,
    pub storage_gid: u32,
    pub hotplug_net_nodes: bool,
    pub hotplug_storage_nodes: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimePolicyDriftRuntimeProfile {
    pub sample_interval_ticks: u64,
    pub reapply_cooldown_ticks: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LibraryRuntimeFeatureProfile {
    pub boundary_mode: BoundaryMode,
    pub enforce_core_minimal: bool,
    pub strict_optional_features: bool,
    pub expose_vfs_api: bool,
    pub expose_network_api: bool,
    pub expose_ipc_api: bool,
    pub expose_proc_config_api: bool,
    pub expose_sysctl_api: bool,
    pub libnet_fast_path_run_pump: bool,
    pub libnet_fast_path_collect_transport_snapshot: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompatSurfaceProfile {
    pub expose_proc_config_api: bool,
    pub expose_sysctl_api: bool,
    pub expose_linux_compat_surface: bool,
    pub attack_surface_budget: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CredentialRuntimeProfile {
    pub security_enforcement: bool,
    pub capability_enforcement: bool,
    pub multi_user: bool,
    pub credential_enforcement: bool,
}
