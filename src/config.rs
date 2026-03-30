//! Kernel Configuration Manager
//! Provides a unified interface to access both compile-time and potentially runtime settings.

use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use crate::generated_consts::*;

mod control_plane;
mod constants;
mod debug_macros;
mod drivers;
mod feature_catalog;
mod key_api;
mod library_surface;
mod network;
mod overrides;
mod parsers;
mod policy;
mod policy_profiles;
mod policy_virtualization;
mod profiles;
mod reset;
mod runtime_tuning;
mod scheduler;
mod vfs_devfs;

use self::constants::*;
use self::overrides::*;
pub use control_plane::{
    ConfigAuditStats, ConfigBatchApplyError, ConfigBlockerSeverity,
    ConfigFeatureCategorySummary,
    ConfigFeatureControl, ConfigLinuxCompatBlocker, ConfigLinuxCompatReadiness,
    ConfigOverridePreviewEntry, ConfigOverridePreviewSummary, KernelConfigSnapshot,
};
pub use debug_macros::{ObservabilityCategory, is_category_enabled_compile_time};
pub use key_api::{ConfigKeySpec, ConfigSetError, ConfigValue, ConfigValueKind};
pub use parsers::{
    AffinityPolicy, BoundaryMode, DevFsPolicyProfile, IdleStrategy, LibNetFastPathStrategy,
    PanicAction, TlsPolicyProfile, VirtualizationExecutionClass, VirtualizationGovernorClass,
    WatchdogAction,
};
pub use profiles::{
    CompatSurfaceProfile, CredentialRuntimeProfile, DevFsRuntimeProfile,
    DriverNetworkRuntimeProfile, LibraryRuntimeFeatureProfile, NetworkRuntimeProfile,
    NetworkSloRuntimeConfig, RuntimePolicyDriftRuntimeProfile, SchedulerRuntimeProfile,
    TelemetryRuntimeProfile, VfsRuntimeProfile, VirtualizationExecutionPolicyProfile,
    VirtualizationExecutionProfile, VirtualizationGovernorPolicyProfile,
    VirtualizationGovernorProfile, VirtualizationPolicyProfile, VirtualizationRuntimeProfile,
};

pub struct KernelConfig;

static WATCHDOG_HARD_STALL_NS_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static RT_FORCE_RESCHEDULE_MIN_TICKS_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static RT_DEADLINE_BURST_THRESHOLD_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static MODULE_LOADER_MAX_LOAD_SEGMENTS_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static MODULE_LOADER_MAX_TOTAL_IMAGE_BYTES_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static LAUNCH_MAX_PROCESS_NAME_LEN_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static LAUNCH_MAX_BOOT_IMAGE_BYTES_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static LAUNCH_STAGE_TIMEOUT_EPOCHS_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static VFS_MAX_MOUNTS_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static VFS_MAX_MOUNT_PATH_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static IRQ_VECTOR_BASE_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static POWER_RUNQUEUE_SATURATION_LIMIT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static IRQSAFE_MUTEX_DEADLOCK_SPIN_LIMIT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static DISKFS_MAX_PATH_LEN_OVERRIDE: AtomicUsize = AtomicUsize::new(0);

static VFS_ENABLE_BUFFERED_IO_OVERRIDE: core::sync::atomic::AtomicUsize =
    core::sync::atomic::AtomicUsize::new(0); // 0=default, 1=false, 2=true
static VFS_HEALTH_SLO_MS_OVERRIDE: core::sync::atomic::AtomicU64 =
    core::sync::atomic::AtomicU64::new(0);
static VFS_HEALTH_MAX_MOUNT_FAILURE_RATE_PER_MILLE_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static VFS_HEALTH_MAX_UNMOUNT_FAILURE_RATE_PER_MILLE_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static VFS_HEALTH_MAX_PATH_VALIDATION_FAILURES_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static VFS_HEALTH_MAX_MOUNT_CAPACITY_PERCENT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static DEVFS_DEFAULT_MODE_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static DEVFS_DEFAULT_UID_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static DEVFS_DEFAULT_GID_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static DEVFS_POLICY_PROFILE_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static DEVFS_NET_MODE_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static DEVFS_NET_GID_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static DEVFS_STORAGE_MODE_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static DEVFS_STORAGE_GID_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static DEVFS_ENABLE_HOTPLUG_NET_NODES_OVERRIDE: AtomicUsize = AtomicUsize::new(0); // 0=default, 1=false, 2=true
static DEVFS_ENABLE_HOTPLUG_STORAGE_NODES_OVERRIDE: AtomicUsize = AtomicUsize::new(0); // 0=default, 1=false, 2=true
const DEFAULT_VFS_ENABLE_BUFFERED_IO: bool = VFS_ENABLE_BUFFERED_IO_STUB;
const DEFAULT_VFS_HEALTH_SLO_MS: u64 = VFS_HEALTH_SLO_MS;
const DEFAULT_DEVFS_DEFAULT_MODE: u16 = (VFS_DEVFS_DEFAULT_MODE as u16) & 0o777;
const DEFAULT_DEVFS_DEFAULT_UID: u32 = VFS_DEVFS_DEFAULT_UID;
const DEFAULT_DEVFS_DEFAULT_GID: u32 = VFS_DEVFS_DEFAULT_GID;
const DEFAULT_DEVFS_POLICY_PROFILE: &'static str = VFS_DEVFS_POLICY_PROFILE;
const DEFAULT_DEVFS_NET_MODE: u16 = (VFS_DEVFS_NET_MODE as u16) & 0o777;
const DEFAULT_DEVFS_NET_GID: u32 = VFS_DEVFS_NET_GID;
const DEFAULT_DEVFS_STORAGE_MODE: u16 = (VFS_DEVFS_STORAGE_MODE as u16) & 0o777;
const DEFAULT_DEVFS_STORAGE_GID: u32 = VFS_DEVFS_STORAGE_GID;
const DEFAULT_DEVFS_ENABLE_HOTPLUG_NET_NODES: bool = VFS_DEVFS_ENABLE_HOTPLUG_NET_NODES;
const DEFAULT_DEVFS_ENABLE_HOTPLUG_STORAGE_NODES: bool = VFS_DEVFS_ENABLE_HOTPLUG_STORAGE_NODES;

static LIBNET_POLL_INTERVAL_LOW_LATENCY_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static LIBNET_POLL_INTERVAL_BALANCED_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static LIBNET_POLL_INTERVAL_POWERSAVE_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static LIBNET_ADAPTIVE_QUEUE_DEPTH_DIVISOR_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static LIBNET_ADAPTIVE_HEALTH_LOW_THRESHOLD_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static LIBNET_ADAPTIVE_POLL_HIGH_THRESHOLD_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static NETWORK_RUNTIME_POLL_INTERVAL_MIN_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static NETWORK_EPOLL_MAX_EVENTS_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static NETWORK_EPOLL_MAX_FDS_PER_INSTANCE_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static NETWORK_LOOPBACK_QUEUE_LIMIT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static NETWORK_UDP_QUEUE_LIMIT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static NETWORK_TCP_QUEUE_LIMIT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static NETWORK_FILTER_RULE_LIMIT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static NETWORK_WIREGUARD_MAX_PEERS_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static NETWORK_HTTP_ASSET_LIMIT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static NETWORK_SLO_SAMPLE_INTERVAL_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static NETWORK_SLO_LOG_INTERVAL_MULTIPLIER_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static NETWORK_TLS_POLICY_PROFILE_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static LIBNET_POSIX_FD_START_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static LIBNET_POSIX_EPHEMERAL_START_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static LIBNET_POSIX_BLOCKING_RECV_RETRIES_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static DRIVER_NETWORK_IRQ_SERVICE_BUDGET_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static DRIVER_NETWORK_LOOP_SERVICE_BUDGET_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static DRIVER_NETWORK_RING_LIMIT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static DRIVER_NETWORK_QUARANTINE_REBIND_FAILURES_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static DRIVER_NETWORK_QUARANTINE_COOLDOWN_SAMPLES_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static DRIVER_NETWORK_SLO_MAX_DROP_RATE_PER_MILLE_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static DRIVER_NETWORK_SLO_MAX_TX_RING_UTILIZATION_PERCENT_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static DRIVER_NETWORK_SLO_MAX_RX_RING_UTILIZATION_PERCENT_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static DRIVER_NETWORK_SLO_MAX_IO_ERRORS_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static DRIVER_NETWORK_LOW_LATENCY_IRQ_BUDGET_DIVISOR_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static DRIVER_NETWORK_LOW_LATENCY_LOOP_BUDGET_DIVISOR_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static DRIVER_NETWORK_LOW_LATENCY_RING_LIMIT_DIVISOR_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static DRIVER_NETWORK_THROUGHPUT_IRQ_BUDGET_MULTIPLIER_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static DRIVER_NETWORK_THROUGHPUT_LOOP_BUDGET_MULTIPLIER_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static DRIVER_NETWORK_THROUGHPUT_RING_LIMIT_MULTIPLIER_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static AHCI_IO_TIMEOUT_SPINS_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static NVME_DISABLE_READY_TIMEOUT_SPINS_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static NVME_POLL_TIMEOUT_SPINS_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static NVME_IO_TIMEOUT_SPINS_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static E1000_RESET_TIMEOUT_SPINS_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static E1000_BUFFER_SIZE_BYTES_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static E1000_RX_DESC_COUNT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static E1000_TX_DESC_COUNT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static RUNTIME_POLICY_DRIFT_SAMPLE_INTERVAL_TICKS_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static RUNTIME_POLICY_DRIFT_REAPPLY_COOLDOWN_TICKS_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static SYSCALL_MAX_PATH_LEN_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static SCHED_LOTTERY_INITIAL_SEED_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static SCHED_LOTTERY_TICKETS_PER_PRIORITY_LEVEL_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static SCHED_LOTTERY_MIN_TICKETS_PER_TASK_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static SCHED_LOTTERY_LCG_MULTIPLIER_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static SCHED_LOTTERY_LCG_INCREMENT_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static SCHED_CFS_MIN_GRANULARITY_NS_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static SCHED_CFS_LATENCY_TARGET_NS_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static SCHED_MLFQ_BASE_SLICE_NS_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static SCHED_MLFQ_BOOST_INTERVAL_TICKS_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static SCHED_MLFQ_DEMOTE_ON_SLICE_EXHAUSTION_OVERRIDE: AtomicUsize = AtomicUsize::new(0); // 0=default,1=false,2=true
static SCHED_EDF_ENFORCE_DEADLINE_OVERRIDE: AtomicUsize = AtomicUsize::new(0); // 0=default,1=false,2=true
static SCHED_EDF_DEFAULT_RELATIVE_DEADLINE_NS_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static SCHED_RT_ENABLE_GROUP_RESERVATION_OVERRIDE: AtomicUsize = AtomicUsize::new(0); // 0=default,1=false,2=true
static SCHED_RT_PERIOD_NS_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static SCHED_RT_TOTAL_UTILIZATION_CAP_PERCENT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static SCHED_RT_MAX_GROUPS_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static TELEMETRY_ENABLED_OVERRIDE: AtomicUsize = AtomicUsize::new(0); // 0=default,1=false,2=true
static TELEMETRY_RUNTIME_SUMMARY_OVERRIDE: AtomicUsize = AtomicUsize::new(0); // 0=default,1=false,2=true
static TELEMETRY_VIRTUALIZATION_OVERRIDE: AtomicUsize = AtomicUsize::new(0); // 0=default,1=false,2=true
static TELEMETRY_PLATFORM_LIFECYCLE_OVERRIDE: AtomicUsize = AtomicUsize::new(0); // 0=default,1=false,2=true
static VIRTUALIZATION_ENTRY_OVERRIDE: AtomicUsize = AtomicUsize::new(0); // 0=default,1=false,2=true
static VIRTUALIZATION_RESUME_OVERRIDE: AtomicUsize = AtomicUsize::new(0); // 0=default,1=false,2=true
static VIRTUALIZATION_TRAP_DISPATCH_OVERRIDE: AtomicUsize = AtomicUsize::new(0); // 0=default,1=false,2=true
static VIRTUALIZATION_NESTED_OVERRIDE: AtomicUsize = AtomicUsize::new(0); // 0=default,1=false,2=true
static VIRTUALIZATION_TIME_VIRTUALIZATION_OVERRIDE: AtomicUsize = AtomicUsize::new(0); // 0=default,1=false,2=true
static VIRTUALIZATION_DEVICE_PASSTHROUGH_OVERRIDE: AtomicUsize = AtomicUsize::new(0); // 0=default,1=false,2=true
static VIRTUALIZATION_SNAPSHOT_OVERRIDE: AtomicUsize = AtomicUsize::new(0); // 0=default,1=false,2=true
static VIRTUALIZATION_DIRTY_LOGGING_OVERRIDE: AtomicUsize = AtomicUsize::new(0); // 0=default,1=false,2=true
static VIRTUALIZATION_LIVE_MIGRATION_OVERRIDE: AtomicUsize = AtomicUsize::new(0); // 0=default,1=false,2=true
static VIRTUALIZATION_TRAP_TRACING_OVERRIDE: AtomicUsize = AtomicUsize::new(0); // 0=default,1=false,2=true
static VIRTUALIZATION_EXECUTION_CLASS_OVERRIDE: AtomicUsize = AtomicUsize::new(0); // 0=default,1=latency,2=balanced,3=background
static VIRTUALIZATION_GOVERNOR_CLASS_OVERRIDE: AtomicUsize = AtomicUsize::new(0); // 0=default,1=performance,2=balanced,3=efficiency
static TELEMETRY_VFS_OVERRIDE: AtomicUsize = AtomicUsize::new(0); // 0=default,1=false,2=true
static TELEMETRY_NETWORK_OVERRIDE: AtomicUsize = AtomicUsize::new(0); // 0=default,1=false,2=true
static TELEMETRY_IPC_OVERRIDE: AtomicUsize = AtomicUsize::new(0); // 0=default,1=false,2=true
static TELEMETRY_SCHEDULER_OVERRIDE: AtomicUsize = AtomicUsize::new(0); // 0=default,1=false,2=true
static TELEMETRY_SECURITY_OVERRIDE: AtomicUsize = AtomicUsize::new(0); // 0=default,1=false,2=true
static TELEMETRY_POWER_OVERRIDE: AtomicUsize = AtomicUsize::new(0); // 0=default,1=false,2=true
static TELEMETRY_DRIVERS_OVERRIDE: AtomicUsize = AtomicUsize::new(0); // 0=default,1=false,2=true
static DEBUG_TRACE_ENABLED_OVERRIDE: AtomicUsize = AtomicUsize::new(0); // 0=default,1=false,2=true
static SERIAL_EARLY_DEBUG_ENABLED_OVERRIDE: AtomicUsize = AtomicUsize::new(0); // 0=default,1=false,2=true

// Granular category-level observability overrides (0=default,1=false,2=true)
static OBSERVABILITY_CORE_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static OBSERVABILITY_BOOT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static OBSERVABILITY_LOADER_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static OBSERVABILITY_TASK_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static OBSERVABILITY_MEMORY_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static OBSERVABILITY_SCHEDULER_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static OBSERVABILITY_FAULT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static OBSERVABILITY_DRIVER_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static OBSERVABILITY_IO_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static OBSERVABILITY_NETWORK_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static OBSERVABILITY_SYSCALL_OVERRIDE: AtomicUsize = AtomicUsize::new(0);

static TELEMETRY_HISTORY_LEN_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static TELEMETRY_LOG_LEVEL_NUM_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static LIBRARY_BOUNDARY_MODE_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static LIBRARY_ENFORCE_CORE_MINIMAL_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static LIBRARY_STRICT_OPTIONAL_FEATURES_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static LIBRARY_EXPOSE_VFS_API_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static LIBRARY_EXPOSE_NETWORK_API_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static LIBRARY_EXPOSE_IPC_API_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static LIBRARY_EXPOSE_PROC_CONFIG_API_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static LIBRARY_EXPOSE_SYSCTL_API_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static SECURITY_ENFORCEMENT_ENABLED_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static CAPABILITY_ENFORCEMENT_ENABLED_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static MULTI_USER_ENABLED_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static CREDENTIAL_ENFORCEMENT_ENABLED_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static LIBNET_FAST_PATH_RUN_PUMP_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static LIBNET_FAST_PATH_COLLECT_TRANSPORT_SNAPSHOT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Copy)]
pub struct CoreRuntimeLimits {
    pub irq_vector_base: u8,
    pub watchdog_hard_stall_ns: u64,
    pub rt_force_reschedule_min_ticks: usize,
    pub rt_deadline_burst_threshold: usize,
    pub rebalance_prefer_local_skip_budget: usize,
    pub module_loader_max_load_segments: usize,
    pub module_loader_max_total_image_bytes: u64,
    pub launch_max_process_name_len: usize,
    pub launch_max_boot_image_bytes: usize,
    pub launch_handoff_stage_timeout_epochs: u64,
    pub vfs_max_mounts: usize,
    pub vfs_max_mount_path: usize,
    pub power_runqueue_saturation_limit: usize,
    pub irqsafe_mutex_deadlock_spin_limit: usize,
    pub load_balance_percentile_window: usize,
}

impl KernelConfig {
    /// Get the default time slice for the active scheduler.
    pub fn time_slice() -> u64 {
        TIME_SLICE_NS
    }

    /// Get the stack size in bytes.
    pub fn stack_size() -> usize {
        STACK_SIZE_PAGES * PAGE_SIZE_BYTES
    }

    /// Check if a specific feature is logically enabled.
    /// This can be expanded to check runtime flags.
    pub fn is_ring_protection_enabled() -> bool {
        cfg!(feature = "ring_protection")
    }

    pub fn is_telemetry_enabled() -> bool {
        decode_bool_override(
            TELEMETRY_ENABLED_OVERRIDE.load(Ordering::Relaxed),
            TELEMETRY_ENABLED,
        )
    }

    /// Get the targeted CPU architecture.
    pub fn arch() -> &'static str {
        TARGET_ARCH
    }

    pub fn is_smp_enabled() -> bool {
        CORE_ENABLE_SMP
    }

    pub fn is_work_stealing_enabled() -> bool {
        CORE_ENABLE_WORK_STEALING
    }

    pub fn is_acpi_discovery_enabled() -> bool {
        CORE_ENABLE_ACPI_DISCOVERY
    }

    pub fn is_iommu_enabled() -> bool {
        CORE_ENABLE_IOMMU
    }

    pub fn is_virtualization_enabled() -> bool {
        CORE_ENABLE_VIRTUALIZATION
    }

    pub fn is_tls_syscalls_enabled() -> bool {
        CORE_ENABLE_TLS_SYSCALLS
    }

    pub fn is_kernel_dump_enabled() -> bool {
        CORE_ENABLE_KERNEL_DUMP
    }

    pub fn is_extended_irq_vectors_enabled() -> bool {
        CORE_ENABLE_EXTENDED_IRQ_VECTORS
    }

    pub fn is_irq_trace_enabled() -> bool {
        CORE_ENABLE_IRQ_TRACE
    }

    pub fn is_scheduler_trace_enabled() -> bool {
        CORE_ENABLE_SCHEDULER_TRACE
    }

    pub fn is_syscall_tracing_enabled() -> bool {
        decode_bool_override(
            OBSERVABILITY_SYSCALL_OVERRIDE.load(Ordering::Relaxed),
            LINUX_VERBOSE_SYSCALL_LOGS,
        )
    }

    pub fn is_advanced_debug_enabled() -> bool {
        Self::is_kernel_dump_enabled()
            || Self::is_irq_trace_enabled()
            || Self::is_scheduler_trace_enabled()
    }

    pub fn idle_strategy() -> &'static str {
        CORE_IDLE_STRATEGY
    }

    pub fn idle_strategy_mode() -> IdleStrategy {
        IdleStrategy::from_str(CORE_IDLE_STRATEGY)
    }

    pub fn panic_action() -> &'static str {
        CORE_PANIC_ACTION
    }

    pub fn panic_action_mode() -> PanicAction {
        PanicAction::from_str(CORE_PANIC_ACTION)
    }

    pub fn is_quiet_boot() -> bool {
        BOOT_QUIET
    }

    pub fn log_level() -> &'static str {
        LOG_LEVEL
    }

    pub fn log_level_num() -> u8 {
        let override_value = TELEMETRY_LOG_LEVEL_NUM_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_TELEMETRY_LOG_LEVEL_NUM
        } else {
            (override_value as u8).clamp(MIN_TELEMETRY_LOG_LEVEL_NUM, MAX_TELEMETRY_LOG_LEVEL_NUM)
        }
    }

    pub fn set_log_level_num(value: Option<u8>) {
        TELEMETRY_LOG_LEVEL_NUM_OVERRIDE.store(value.unwrap_or(0) as usize, Ordering::Relaxed);
    }

    pub fn telemetry_history_len() -> usize {
        let override_value = TELEMETRY_HISTORY_LEN_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_TELEMETRY_HISTORY_LEN.max(1)
        } else {
            override_value.max(1).min(MAX_TELEMETRY_HISTORY_LEN)
        }
    }

    pub fn set_telemetry_history_len(value: Option<usize>) {
        TELEMETRY_HISTORY_LEN_OVERRIDE.store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn set_telemetry_enabled(value: Option<bool>) {
        TELEMETRY_ENABLED_OVERRIDE.store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn is_periodic_rebalance_enabled() -> bool {
        CORE_ENABLE_PERIODIC_REBALANCE
    }

    pub fn rebalance_interval_ticks() -> u64 {
        CORE_REBALANCE_INTERVAL_TICKS.max(MIN_REBALANCE_INTERVAL_TICKS)
    }

    pub fn rebalance_imbalance_threshold() -> usize {
        CORE_REBALANCE_IMBALANCE_THRESHOLD
    }

    pub fn rebalance_batch_size() -> usize {
        CORE_REBALANCE_BATCH_SIZE.max(MIN_REBALANCE_BATCH_SIZE)
    }

    pub fn rebalance_prefer_local_skip_budget() -> usize {
        CORE_REBALANCE_PREFER_LOCAL_SKIP_BUDGET
            .max(MIN_REBALANCE_PREFER_LOCAL_SKIP_BUDGET)
            .min(MAX_REBALANCE_PREFER_LOCAL_SKIP_BUDGET)
    }

    pub fn affinity_policy() -> &'static str {
        CORE_AFFINITY_POLICY
    }

    pub fn affinity_policy_mode() -> AffinityPolicy {
        AffinityPolicy::from_str(CORE_AFFINITY_POLICY)
    }

    pub fn is_affinity_enforcement_enabled() -> bool {
        CORE_ENABLE_AFFINITY_ENFORCEMENT
    }

    pub fn is_interrupt_storm_protection_enabled() -> bool {
        CORE_ENABLE_INTERRUPT_STORM_PROTECTION
    }

    pub fn irq_storm_threshold() -> u32 {
        CORE_IRQ_STORM_THRESHOLD
    }

    pub fn irq_storm_window_ticks() -> u64 {
        CORE_IRQ_STORM_WINDOW_TICKS
    }

    pub fn is_soft_watchdog_enabled() -> bool {
        CORE_ENABLE_SOFT_WATCHDOG
    }

    pub fn soft_watchdog_stall_ticks() -> u64 {
        CORE_SOFT_WATCHDOG_STALL_TICKS
    }

    pub fn soft_watchdog_action() -> &'static str {
        CORE_SOFT_WATCHDOG_ACTION
    }

    pub fn soft_watchdog_action_mode() -> WatchdogAction {
        WatchdogAction::from_str(CORE_SOFT_WATCHDOG_ACTION)
    }
}

#[cfg(test)]
#[path = "config/tests.rs"]
mod tests;

/// Global system limits
pub mod limits {
    pub const MAX_PROCESSES: usize = 1024;
    pub const MAX_THREADS_PER_PROCESS: usize = 256;
    pub const MAX_OPEN_FILES: usize = 65536;
}
