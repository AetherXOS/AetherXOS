use core::sync::atomic::{AtomicU64, AtomicUsize};

pub static DEVFS_DEFAULT_GID_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static DEVFS_DEFAULT_MODE_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static DEVFS_DEFAULT_UID_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static DEVFS_ENABLE_HOTPLUG_NET_NODES_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static DEVFS_ENABLE_HOTPLUG_STORAGE_NODES_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static DEVFS_NET_GID_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static DEVFS_NET_MODE_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static DEVFS_POLICY_PROFILE_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static DEVFS_STORAGE_GID_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static DEVFS_STORAGE_MODE_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static DISKFS_MAX_PATH_LEN_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static VFS_ENABLE_BUFFERED_IO_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static VFS_HEALTH_MAX_MOUNT_CAPACITY_PERCENT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static VFS_HEALTH_MAX_MOUNT_FAILURE_RATE_PER_MILLE_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static VFS_HEALTH_MAX_PATH_VALIDATION_FAILURES_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static VFS_HEALTH_MAX_UNMOUNT_FAILURE_RATE_PER_MILLE_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static VFS_HEALTH_SLO_MS_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static VFS_MAX_MOUNTS_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static VFS_MAX_MOUNT_PATH_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static LIBNET_POSIX_EPHEMERAL_START_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static LIBNET_POSIX_FD_START_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static NETWORK_EPOLL_MAX_EVENTS_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static NETWORK_EPOLL_MAX_FDS_PER_INSTANCE_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static NETWORK_FILTER_RULE_LIMIT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static NETWORK_HTTP_ASSET_LIMIT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static NETWORK_LOOPBACK_QUEUE_LIMIT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static NETWORK_RUNTIME_POLL_INTERVAL_MIN_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static NETWORK_SLO_LOG_INTERVAL_MULTIPLIER_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static NETWORK_SLO_SAMPLE_INTERVAL_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static NETWORK_TCP_QUEUE_LIMIT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static NETWORK_TLS_POLICY_PROFILE_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static NETWORK_UDP_QUEUE_LIMIT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static NETWORK_WIREGUARD_MAX_PEERS_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static LIBNET_ADAPTIVE_HEALTH_LOW_THRESHOLD_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static LIBNET_ADAPTIVE_POLL_HIGH_THRESHOLD_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static LIBNET_ADAPTIVE_QUEUE_DEPTH_DIVISOR_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static LIBNET_POLL_INTERVAL_POWERSAVE_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static LIBNET_POSIX_BLOCKING_RECV_RETRIES_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static LIBNET_POLL_INTERVAL_BALANCED_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static LIBNET_POLL_INTERVAL_LOW_LATENCY_OVERRIDE: AtomicU64 = AtomicU64::new(0);

// Driver/Network overrides
pub static DRIVER_NETWORK_IRQ_SERVICE_BUDGET_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static DRIVER_NETWORK_LOOP_SERVICE_BUDGET_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static DRIVER_NETWORK_LOW_LATENCY_IRQ_BUDGET_DIVISOR_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static DRIVER_NETWORK_LOW_LATENCY_LOOP_BUDGET_DIVISOR_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static DRIVER_NETWORK_LOW_LATENCY_RING_LIMIT_DIVISOR_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static DRIVER_NETWORK_QUARANTINE_COOLDOWN_SAMPLES_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static DRIVER_NETWORK_QUARANTINE_REBIND_FAILURES_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static DRIVER_NETWORK_RING_LIMIT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static DRIVER_NETWORK_SLO_MAX_DROP_RATE_PER_MILLE_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static DRIVER_NETWORK_SLO_MAX_IO_ERRORS_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static DRIVER_NETWORK_SLO_MAX_RX_RING_UTILIZATION_PERCENT_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static DRIVER_NETWORK_SLO_MAX_TX_RING_UTILIZATION_PERCENT_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static DRIVER_NETWORK_THROUGHPUT_IRQ_BUDGET_MULTIPLIER_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static DRIVER_NETWORK_THROUGHPUT_LOOP_BUDGET_MULTIPLIER_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static DRIVER_NETWORK_THROUGHPUT_RING_LIMIT_MULTIPLIER_OVERRIDE: AtomicUsize = AtomicUsize::new(0);

// Core overrides
pub static WATCHDOG_HARD_STALL_NS_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static RT_FORCE_RESCHEDULE_MIN_TICKS_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static RT_DEADLINE_BURST_THRESHOLD_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static MODULE_LOADER_MAX_LOAD_SEGMENTS_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static MODULE_LOADER_MAX_TOTAL_IMAGE_BYTES_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static LAUNCH_MAX_PROCESS_NAME_LEN_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static LAUNCH_MAX_BOOT_IMAGE_BYTES_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static LAUNCH_STAGE_TIMEOUT_EPOCHS_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static IRQ_VECTOR_BASE_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static POWER_RUNQUEUE_SATURATION_LIMIT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static IRQSAFE_MUTEX_DEADLOCK_SPIN_LIMIT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static LOAD_BALANCE_PERCENTILE_WINDOW_OVERRIDE: AtomicUsize = AtomicUsize::new(0);

// Hardware overrides
pub static AHCI_IO_TIMEOUT_SPINS_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static NVME_DISABLE_READY_TIMEOUT_SPINS_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static NVME_POLL_TIMEOUT_SPINS_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static NVME_IO_TIMEOUT_SPINS_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static E1000_RESET_TIMEOUT_SPINS_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static E1000_BUFFER_SIZE_BYTES_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static E1000_RX_DESC_COUNT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static E1000_TX_DESC_COUNT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);

// Policy/Scheduler overrides
pub static RUNTIME_POLICY_DRIFT_SAMPLE_INTERVAL_TICKS_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static RUNTIME_POLICY_DRIFT_REAPPLY_COOLDOWN_TICKS_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static SYSCALL_MAX_PATH_LEN_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static SCHED_LOTTERY_INITIAL_SEED_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static SCHED_LOTTERY_TICKETS_PER_PRIORITY_LEVEL_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static SCHED_LOTTERY_MIN_TICKETS_PER_TASK_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static SCHED_LOTTERY_LCG_MULTIPLIER_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static SCHED_LOTTERY_LCG_INCREMENT_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static SCHED_CFS_MIN_GRANULARITY_NS_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static SCHED_CFS_LATENCY_TARGET_NS_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static SCHED_MLFQ_BASE_SLICE_NS_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static SCHED_MLFQ_BOOST_INTERVAL_TICKS_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static SCHED_MLFQ_DEMOTE_ON_SLICE_EXHAUSTION_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static SCHED_EDF_ENFORCE_DEADLINE_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static SCHED_EDF_DEFAULT_RELATIVE_DEADLINE_NS_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static SCHED_RT_ENABLE_GROUP_RESERVATION_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static SCHED_RT_PERIOD_NS_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static SCHED_RT_TOTAL_UTILIZATION_CAP_PERCENT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static SCHED_RT_MAX_GROUPS_OVERRIDE: AtomicUsize = AtomicUsize::new(0);

// Library/System Policy overrides
pub static LIBRARY_BOUNDARY_MODE_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static LIBRARY_ENFORCE_CORE_MINIMAL_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static LIBRARY_STRICT_OPTIONAL_FEATURES_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static LIBRARY_EXPOSE_VFS_API_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static LIBRARY_EXPOSE_NETWORK_API_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static LIBRARY_EXPOSE_IPC_API_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static LIBRARY_EXPOSE_PROC_CONFIG_API_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static LIBRARY_EXPOSE_SYSCTL_API_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static EXEC_ELF_REQUIRE_ABSOLUTE_INTERP_PATH_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static EXEC_ELF_ENFORCE_INTERP_PATH_SANITIZATION_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static EXEC_ELF_ENFORCE_SYSTEM_LOADER_PATHS_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static EXEC_ELF_ENFORCE_SEGMENT_CONGRUENCE_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static EXEC_AUXV_ENFORCE_HANDOFF_CONTRACT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static EXEC_AUXV_REQUIRE_PHDR_TRIPLET_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static USERSPACE_ABI_REQUIRE_GLIBC_VFS_SURFACE_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static USERSPACE_ABI_REQUIRE_GLIBC_NETWORK_SURFACE_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static USERSPACE_ABI_REQUIRE_GLIBC_IPC_SURFACE_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static USERSPACE_ABI_LIBC_SURFACE_WEIGHT_PERCENT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static SECURITY_ENFORCEMENT_ENABLED_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static CAPABILITY_ENFORCEMENT_ENABLED_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static MULTI_USER_ENABLED_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static CREDENTIAL_ENFORCEMENT_ENABLED_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static LIBNET_FAST_PATH_RUN_PUMP_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static LIBNET_FAST_PATH_COLLECT_TRANSPORT_SNAPSHOT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);

// Telemetry overrides
pub static TELEMETRY_ENABLED_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static TELEMETRY_RUNTIME_SUMMARY_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static TELEMETRY_VFS_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static TELEMETRY_VIRTUALIZATION_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static TELEMETRY_PLATFORM_LIFECYCLE_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static TELEMETRY_NETWORK_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static TELEMETRY_IPC_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static TELEMETRY_SCHEDULER_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static TELEMETRY_SECURITY_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static TELEMETRY_POWER_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static TELEMETRY_DRIVERS_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static TELEMETRY_HISTORY_LEN_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static TELEMETRY_LOG_LEVEL_NUM_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static DEBUG_TRACE_ENABLED_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static SERIAL_EARLY_DEBUG_ENABLED_OVERRIDE: AtomicUsize = AtomicUsize::new(0);

// Virtualization overrides
pub static VIRTUALIZATION_SNAPSHOT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static VIRTUALIZATION_ENTRY_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static VIRTUALIZATION_RESUME_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static VIRTUALIZATION_TRAP_DISPATCH_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static VIRTUALIZATION_NESTED_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static VIRTUALIZATION_TIME_VIRTUALIZATION_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static VIRTUALIZATION_DEVICE_PASSTHROUGH_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static VIRTUALIZATION_DIRTY_LOGGING_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static VIRTUALIZATION_LIVE_MIGRATION_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static VIRTUALIZATION_TRAP_TRACING_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static VIRTUALIZATION_EXECUTION_CLASS_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static VIRTUALIZATION_GOVERNOR_CLASS_OVERRIDE: AtomicUsize = AtomicUsize::new(0);

// Observability overrides
pub static OBSERVABILITY_CORE_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static OBSERVABILITY_BOOT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static OBSERVABILITY_LOADER_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static OBSERVABILITY_TASK_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static OBSERVABILITY_MEMORY_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static OBSERVABILITY_SCHEDULER_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static OBSERVABILITY_FAULT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static OBSERVABILITY_DRIVER_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static OBSERVABILITY_IO_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static OBSERVABILITY_NETWORK_OVERRIDE: AtomicUsize = AtomicUsize::new(0);

// Watchdog and system monitoring
pub static SOFT_WATCHDOG_ENABLED_OVERRIDE: AtomicU64 = AtomicU64::new(1);
pub static SOFT_WATCHDOG_STALL_TICKS_OVERRIDE: AtomicU64 = AtomicU64::new(4096);
pub static SOFT_WATCHDOG_ACTION_OVERRIDE: AtomicU64 = AtomicU64::new(0);

// Stack and memory
pub static STACK_SIZE_OVERRIDE: AtomicUsize = AtomicUsize::new(0);

// Scheduler and load balancing
pub static REBALANCE_IMBALANCE_THRESHOLD_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
pub static SCHEDULER_TRACE_ENABLED_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static AFFINITY_POLICY_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static WORK_STEALING_ENABLED_OVERRIDE: AtomicU64 = AtomicU64::new(1);
pub static PERIODIC_REBALANCE_ENABLED_OVERRIDE: AtomicU64 = AtomicU64::new(1);
pub static REBALANCE_INTERVAL_TICKS_OVERRIDE: AtomicU64 = AtomicU64::new(100);
pub static AFFINITY_ENFORCEMENT_ENABLED_OVERRIDE: AtomicU64 = AtomicU64::new(0);
pub static SYSCALL_TRACING_ENABLED_OVERRIDE: AtomicU64 = AtomicU64::new(0);


pub const DEFAULT_DEVFS_DEFAULT_MODE: u16 = 0o755;
pub const DEFAULT_DEVFS_DEFAULT_GID: u32 = 0;
pub const DEFAULT_DEVFS_ENABLE_HOTPLUG_NET_NODES: bool = true;
pub const DEFAULT_DEVFS_NET_MODE: u16 = 0o660;
pub const DEFAULT_DEVFS_NET_GID: u32 = 0;
pub const DEFAULT_DEVFS_ENABLE_HOTPLUG_STORAGE_NODES: bool = true;
pub const DEFAULT_DEVFS_STORAGE_MODE: u16 = 0o660;
pub const DEFAULT_DEVFS_STORAGE_GID: u32 = 0;
pub const DEFAULT_DEVFS_POLICY_PROFILE: &'static str = "Balanced";
pub const DEFAULT_DEVFS_DEFAULT_UID: u32 = 0;
pub const DEFAULT_VFS_ENABLE_BUFFERED_IO: bool = true;
pub const DEFAULT_VFS_HEALTH_SLO_MS: u64 = 50;

#[derive(Debug, Clone, PartialEq, Eq)]
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
