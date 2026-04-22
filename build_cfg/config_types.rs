//! All configuration type definitions parsed from Cargo.toml.
//! Each subsystem struct includes field-level documentation.

use serde::Deserialize;
use serde::de::{self, Deserializer};

// ── Top-level Config ──────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
pub struct Config {
    pub meta: MetaConfig,
    pub kernel: KernelConfig,
    #[serde(default)]
    pub core: CoreConfig,
    #[serde(default)]
    pub library: LibraryConfig,
    pub boot: BootConfig,
    pub memory: MemoryConfig,
    pub scheduler: SchedulerConfig,
    pub dispatcher: DispatcherConfig,
    pub ipc: IpcConfig,
    pub security: SecurityConfig,
    pub telemetry: TelemetryConfig,
    #[serde(default)]
    pub linux_compat: LinuxCompatConfig,
    #[serde(default)]
    pub vfs: VfsConfig,
    #[serde(default)]
    pub network: NetworkConfig,
    #[serde(default)]
    pub drivers: DriverConfig,
    #[serde(default)]
    pub governor: GovernorConfig,
    #[serde(default)]
    pub linux_os: LinuxOsConfig,
    #[serde(default)]
    pub aarch64: Aarch64Config,
    #[serde(rename = "scheduler.lottery")]
    #[serde(default)]
    pub lottery: LotteryConfig,
    #[serde(default)]
    pub rtos: RtosConfig,
}

// ── Cargo manifest wrappers ───────────────────────────────────────────────────

#[derive(Deserialize, Debug, Default)]
pub struct CargoManifest {
    #[serde(default)]
    pub package: CargoPackage,
}

#[derive(Deserialize, Debug, Default)]
pub struct CargoPackage {
    #[serde(default)]
    pub metadata: CargoMetadata,
}

#[derive(Deserialize, Debug, Default)]
pub struct CargoMetadata {
    #[serde(default)]
    pub aethercore: AethercoreMetadata,
}

#[derive(Deserialize, Debug, Default)]
pub struct AethercoreMetadata {
    pub config: Option<Config>,
}

// ── MetaConfig ────────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
pub struct MetaConfig {
    pub profile: String,
    pub version: String,
}

// ── KernelConfig ──────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
pub struct KernelConfig {
    pub arch: String,
    pub time_slice_ns: u64,
    pub stack_size_pages: usize,
    pub max_cpus: usize,
    pub interrupt_stack_size_pages: usize,
}

// ── CoreConfig ────────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct CoreConfig {
    pub enable_smp: bool,
    pub enable_work_stealing: bool,
    pub enable_pci_enumeration: bool,
    pub enable_acpi_discovery: bool,
    pub enable_iommu: bool,
    pub enable_virtualization: bool,
    pub enable_driver_init: bool,
    pub enable_tls_syscalls: bool,
    pub enable_kernel_dump: bool,
    pub enable_extended_irq_vectors: bool,
    pub enable_irq_trace: bool,
    pub enable_scheduler_trace: bool,
    pub idle_strategy: String,
    pub panic_action: String,
    pub enable_periodic_rebalance: bool,
    pub rebalance_interval_ticks: u64,
    pub rebalance_imbalance_threshold: usize,
    pub rebalance_batch_size: usize,
    pub rebalance_prefer_local_skip_budget: usize,
    pub affinity_policy: String,
    pub enable_affinity_enforcement: bool,
    pub enable_interrupt_storm_protection: bool,
    pub irq_storm_threshold: u32,
    pub irq_storm_window_ticks: u64,
    pub enable_soft_watchdog: bool,
    pub soft_watchdog_stall_ticks: u64,
    pub soft_watchdog_action: String,
    pub crash_log_capacity: usize,
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            enable_smp: true,
            enable_work_stealing: true,
            enable_pci_enumeration: true,
            enable_acpi_discovery: true,
            enable_iommu: false,
            enable_virtualization: false,
            enable_driver_init: true,
            enable_tls_syscalls: true,
            enable_kernel_dump: true,
            enable_extended_irq_vectors: true,
            enable_irq_trace: false,
            enable_scheduler_trace: false,
            idle_strategy: "Halt".to_string(),
            panic_action: "Halt".to_string(),
            enable_periodic_rebalance: true,
            rebalance_interval_ticks: 128,
            rebalance_imbalance_threshold: 2,
            rebalance_batch_size: 4,
            rebalance_prefer_local_skip_budget: 2,
            affinity_policy: "PreferLocal".to_string(),
            enable_affinity_enforcement: true,
            enable_interrupt_storm_protection: true,
            irq_storm_threshold: 4096,
            irq_storm_window_ticks: 256,
            enable_soft_watchdog: true,
            soft_watchdog_stall_ticks: 4096,
            soft_watchdog_action: "Halt".to_string(),
            crash_log_capacity: 64,
        }
    }
}

// ── LibraryConfig ─────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct LibraryConfig {
    pub boundary_mode: String,
    pub enforce_core_minimal: bool,
    pub strict_optional_features: bool,
    pub expose_vfs_api: bool,
    pub expose_network_api: bool,
    pub expose_ipc_api: bool,
    pub libnet_l2_enabled: bool,
    pub libnet_l34_enabled: bool,
    pub libnet_l6_enabled: bool,
    pub libnet_l7_enabled: bool,
    pub libnet_fast_path_default_strategy: String,
    pub libnet_fast_path_run_pump: bool,
    pub libnet_fast_path_collect_transport_snapshot: bool,
    pub libnet_fast_path_pump_budget: usize,
    pub max_services: usize,
    pub verbose_boot_inventory: bool,
}

impl Default for LibraryConfig {
    fn default() -> Self {
        Self {
            boundary_mode: "Strict".to_string(),
            enforce_core_minimal: true,
            strict_optional_features: true,
            expose_vfs_api: true,
            expose_network_api: true,
            expose_ipc_api: true,
            libnet_l2_enabled: true,
            libnet_l34_enabled: true,
            libnet_l6_enabled: true,
            libnet_l7_enabled: true,
            libnet_fast_path_default_strategy: "Adaptive".to_string(),
            libnet_fast_path_run_pump: true,
            libnet_fast_path_collect_transport_snapshot: true,
            libnet_fast_path_pump_budget: 64,
            max_services: 128,
            verbose_boot_inventory: true,
        }
    }
}

// ── BootConfig ────────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
pub struct BootConfig {
    pub cmdline_parse: bool,
    pub splash_screen: bool,
    pub quiet_boot: bool,
}

// ── MemoryConfig ──────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
pub struct MemoryConfig {
    pub allocator: String,
    pub paging: bool,
    pub heap_size_mb: usize,
    pub guardian_pages: bool,
    #[serde(default = "default_mem_slab_refill_bytes")]
    pub slab_refill_bytes: usize,
    #[serde(default = "default_mem_slab_cache_limit")]
    pub slab_cache_limit: usize,
    #[serde(default = "default_mem_slab_release_batch")]
    pub slab_release_batch: usize,
    #[serde(default = "default_mem_slab_cross_cpu_steal")]
    pub slab_cross_cpu_steal: bool,
    #[serde(default = "default_mem_compaction_budget_pages")]
    pub compaction_budget_pages: usize,
    #[serde(default = "default_mem_oom_kill_threshold")]
    pub oom_kill_threshold: u64,
    #[serde(default = "default_mem_prefer_local_numa")]
    pub prefer_local_numa: bool,
    #[serde(default = "default_mem_pool_block_size")]
    pub pool_block_size: usize,
    #[serde(default = "default_mem_slab_reclaim_profile")]
    pub slab_reclaim_profile: String,
    #[serde(default = "default_mem_slab_pressure_scan_budget")]
    pub slab_pressure_scan_budget: usize,
    #[serde(default = "default_mem_slab_max_tracked_segments")]
    pub slab_max_tracked_segments: usize,
}

fn default_mem_slab_refill_bytes() -> usize {
    16 * 1024
}
fn default_mem_slab_cache_limit() -> usize {
    64
}
fn default_mem_slab_release_batch() -> usize {
    32
}
fn default_mem_slab_cross_cpu_steal() -> bool {
    true
}
fn default_mem_compaction_budget_pages() -> usize {
    1024
}
fn default_mem_oom_kill_threshold() -> u64 {
    1
}
fn default_mem_prefer_local_numa() -> bool {
    true
}
fn default_mem_pool_block_size() -> usize {
    4096
}
fn default_mem_slab_reclaim_profile() -> String {
    "Balanced".to_string()
}
fn default_mem_slab_pressure_scan_budget() -> usize {
    8
}
fn default_mem_slab_max_tracked_segments() -> usize {
    1024
}

// ── SchedulerConfig ───────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
pub struct SchedulerConfig {
    pub strategy: String,
    pub priority_levels: usize,
    pub round_robin: Option<RoundRobinConfig>,
    pub cfs: Option<CfsConfig>,
    pub edf: Option<EdfConfig>,
    pub mlfq: Option<MlfqConfig>,
    pub rt: Option<RtSchedulerConfig>,
}

#[derive(Deserialize, Debug)]
pub struct RoundRobinConfig {
    pub max_tasks: usize,
    pub default_slice_ns: u64,
}

#[derive(Deserialize, Debug)]
pub struct CfsConfig {
    pub min_granularity_ns: u64,
    pub latency_target_ns: u64,
}

#[derive(Deserialize, Debug)]
pub struct EdfConfig {
    pub max_deadlines: usize,
    pub enforce_deadline: bool,
    pub default_relative_deadline_ns: u64,
}

#[derive(Deserialize, Debug)]
pub struct MlfqConfig {
    pub num_queues: usize,
    pub base_slice_ns: u64,
    pub boost_interval_ticks: u64,
    pub demote_on_slice_exhaustion: bool,
}

#[derive(Deserialize, Debug)]
pub struct RtSchedulerConfig {
    pub enable_group_reservation: bool,
    pub period_ns: u64,
    pub total_utilization_cap_percent: u8,
    pub max_groups: usize,
}

// ── DispatcherConfig ──────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
pub struct DispatcherConfig {
    pub strategy: String,
    pub buffer_size: usize,
    pub vector_table_align: usize,
}

// ── IpcConfig ─────────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
pub struct IpcConfig {
    pub mechanism: String,
    pub max_channels: usize,
    pub msg_size_limit: usize,
    #[serde(default = "default_ring_buffer_size_kb")]
    pub ring_buffer_size_kb: usize,
    #[serde(default = "default_unix_socket_queue_limit")]
    pub unix_socket_queue_limit: usize,
    #[serde(default = "default_binder_max_objects")]
    pub binder_max_objects: usize,
    #[serde(default = "default_futex_wake_event_limit")]
    pub futex_wake_event_limit: usize,
}

fn default_ring_buffer_size_kb() -> usize {
    64
}
fn default_unix_socket_queue_limit() -> usize {
    256
}
fn default_binder_max_objects() -> usize {
    1024
}
fn default_futex_wake_event_limit() -> usize {
    256
}

// ── SecurityConfig ────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
pub struct SecurityConfig {
    pub ring_level: String,
    pub monitor: String,
    pub nx_bit: bool,
    pub smap_smep: bool,
    #[serde(default)]
    pub zero_trust_mode: bool,
    #[serde(default)]
    pub enable_mac: bool,
    #[serde(default)]
    pub enable_audit: bool,
    #[serde(default = "default_max_security_labels")]
    pub max_security_labels: usize,
    #[serde(default = "default_max_capability_tokens")]
    pub max_capability_tokens: usize,
    #[serde(default)]
    pub default_user_capabilities: u64,
}

fn default_max_security_labels() -> usize {
    256
}
fn default_max_capability_tokens() -> usize {
    4096
}

// ── TelemetryConfig ───────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct TelemetryConfig {
    pub enabled: bool,
    pub history_len: usize,
    pub log_level: String,
    pub runtime_summary: bool,
    pub vfs: bool,
    pub network: bool,
    pub ipc: bool,
    pub scheduler: bool,
    pub security: bool,
    pub power: bool,
    pub drivers: bool,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            history_len: 100,
            log_level: "Info".to_string(),
            runtime_summary: true,
            vfs: true,
            network: true,
            ipc: true,
            scheduler: true,
            security: true,
            power: true,
            drivers: true,
        }
    }
}

// ── LinuxCompatConfig ─────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct LinuxCompatConfig {
    pub max_path_len: usize,
    pub max_iov_count: usize,
    pub max_sockaddr_len: usize,
    pub max_xattr_name_len: usize,
    pub max_xattr_value_size: usize,
    pub default_pipe_size: usize,
    pub max_mount_path: usize,
    pub stat_block_size: u64,
    pub stat_blksize: u32,
    pub mount_ctx_fd_base: i32,
    pub mount_fd_base: i32,
    pub legacy_support: bool,
    pub futex_waitv_max: usize,
    pub robust_list_head_size: usize,
    pub enable_standard_error_mapping: bool,
    pub verbose_syscall_logs: bool,
}

impl Default for LinuxCompatConfig {
    fn default() -> Self {
        Self {
            max_path_len: 4096,
            max_iov_count: 1024,
            max_sockaddr_len: 1024,
            max_xattr_name_len: 255,
            max_xattr_value_size: 65536,
            default_pipe_size: 65536,
            max_mount_path: 256,
            stat_block_size: 512,
            stat_blksize: 4096,
            mount_ctx_fd_base: 2000,
            mount_fd_base: 3000,
            legacy_support: true,
            futex_waitv_max: 128,
            robust_list_head_size: 24,
            enable_standard_error_mapping: true,
            verbose_syscall_logs: false,
        }
    }
}

// ── VfsConfig ─────────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct VfsConfig {
    pub max_mounts: usize,
    pub max_mount_path: usize,
    pub enable_buffered_io: bool,
    pub health_slo_ms: u64,
    pub devfs_default_mode: u32,
    pub devfs_default_uid: u32,
    pub devfs_default_gid: u32,
    pub devfs_enable_hotplug_net_nodes: bool,
    pub devfs_enable_hotplug_storage_nodes: bool,
    pub devfs_policy_profile: String,
    pub devfs_net_mode: u32,
    pub devfs_net_gid: u32,
    pub devfs_storage_mode: u32,
    pub devfs_storage_gid: u32,
}

impl Default for VfsConfig {
    fn default() -> Self {
        Self {
            max_mounts: 256,
            max_mount_path: 4096,
            enable_buffered_io: false,
            health_slo_ms: 100,
            devfs_default_mode: 0o660,
            devfs_default_uid: 0,
            devfs_default_gid: 0,
            devfs_enable_hotplug_net_nodes: true,
            devfs_enable_hotplug_storage_nodes: true,
            devfs_policy_profile: "Balanced".to_string(),
            devfs_net_mode: 0o660,
            devfs_net_gid: 0,
            devfs_storage_mode: 0o660,
            devfs_storage_gid: 0,
        }
    }
}

// ── NetworkConfig ─────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct NetworkConfig {
    pub loopback_queue_limit: usize,
    pub udp_queue_limit: usize,
    pub tcp_queue_limit: usize,
    pub filter_rule_limit: usize,
    pub wireguard_max_peers: usize,
    pub http_asset_limit: usize,
    #[serde(default = "default_network_tls_policy_profile")]
    pub tls_policy_profile: String,
    pub posix_fd_start: u32,
    pub posix_ephemeral_start: u16,
    pub blocking_recv_retries: usize,
    #[serde(default = "default_network_slo_sample_interval")]
    pub slo_sample_interval: u64,
    #[serde(default = "default_network_slo_log_interval_multiplier")]
    pub slo_log_interval_multiplier: u64,
}

fn default_network_slo_sample_interval() -> u64 {
    1024
}
fn default_network_slo_log_interval_multiplier() -> u64 {
    8
}
fn default_network_tls_policy_profile() -> String {
    "Balanced".to_string()
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            loopback_queue_limit: 128,
            udp_queue_limit: 256,
            tcp_queue_limit: 256,
            filter_rule_limit: 128,
            wireguard_max_peers: 1024,
            http_asset_limit: 1024,
            tls_policy_profile: "Balanced".to_string(),
            posix_fd_start: 3,
            posix_ephemeral_start: 40000,
            blocking_recv_retries: 64,
            slo_sample_interval: 1024,
            slo_log_interval_multiplier: 8,
        }
    }
}

// ── DriverConfig ──────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct DriverConfig {
    pub network_irq_service_budget: usize,
    pub network_loop_service_budget: usize,
    pub network_ring_limit: usize,
    pub network_quarantine_rebind_failures: u64,
    pub network_quarantine_cooldown_samples: u64,
    pub network_slo_max_drop_rate_per_mille: u64,
    pub network_slo_max_tx_ring_utilization_percent: u64,
    pub network_slo_max_rx_ring_utilization_percent: u64,
    pub network_slo_max_io_errors: u64,
    pub network_low_latency_irq_budget_divisor: usize,
    pub network_low_latency_loop_budget_divisor: usize,
    pub network_low_latency_ring_limit_divisor: usize,
    pub network_throughput_irq_budget_multiplier: usize,
    pub network_throughput_loop_budget_multiplier: usize,
    pub network_throughput_ring_limit_multiplier: usize,
    pub ahci_io_timeout_spins: usize,
    pub nvme_disable_ready_timeout_spins: usize,
    pub nvme_poll_timeout_spins: usize,
    pub nvme_io_timeout_spins: usize,
    pub e1000_reset_timeout_spins: usize,
    pub e1000_buffer_size_bytes: usize,
    pub e1000_rx_desc_count: usize,
    pub e1000_tx_desc_count: usize,
}

impl Default for DriverConfig {
    fn default() -> Self {
        Self {
            network_irq_service_budget: 64,
            network_loop_service_budget: 128,
            network_ring_limit: 512,
            network_quarantine_rebind_failures: 3,
            network_quarantine_cooldown_samples: 256,
            network_slo_max_drop_rate_per_mille: 25,
            network_slo_max_tx_ring_utilization_percent: 90,
            network_slo_max_rx_ring_utilization_percent: 90,
            network_slo_max_io_errors: 0,
            network_low_latency_irq_budget_divisor: 4,
            network_low_latency_loop_budget_divisor: 2,
            network_low_latency_ring_limit_divisor: 2,
            network_throughput_irq_budget_multiplier: 4,
            network_throughput_loop_budget_multiplier: 4,
            network_throughput_ring_limit_multiplier: 4,
            ahci_io_timeout_spins: 2_000_000,
            nvme_disable_ready_timeout_spins: 100_000,
            nvme_poll_timeout_spins: 500_000,
            nvme_io_timeout_spins: 1_000_000,
            e1000_reset_timeout_spins: 1_000_000,
            e1000_buffer_size_bytes: 2048,
            e1000_rx_desc_count: 32,
            e1000_tx_desc_count: 256,
        }
    }
}

// ── GovernorConfig ────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct GovernorConfig {
    pub watchdog_hard_stall_ns: u64,
    pub rt_force_min_ticks: usize,
    pub rt_deadline_burst_threshold: usize,
    pub irqsafe_mutex_deadlock_spin_limit: usize,
    pub power_runqueue_saturation_limit: usize,
    pub load_balance_percentile_window: usize,
    pub runtime_policy_drift_sample_interval_ticks: u64,
    pub runtime_policy_drift_reapply_cooldown_ticks: u64,
}

impl Default for GovernorConfig {
    fn default() -> Self {
        Self {
            watchdog_hard_stall_ns: 5000000000,
            rt_force_min_ticks: 2,
            rt_deadline_burst_threshold: 3,
            irqsafe_mutex_deadlock_spin_limit: 10000000,
            power_runqueue_saturation_limit: 4096,
            load_balance_percentile_window: 256,
            runtime_policy_drift_sample_interval_ticks: 1024,
            runtime_policy_drift_reapply_cooldown_ticks: 8192,
        }
    }
}

// ── LinuxOsConfig ─────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct LinuxOsConfig {
    pub release: String,
    pub version: String,
}

impl Default for LinuxOsConfig {
    fn default() -> Self {
        Self {
            release: "5.15.0".to_string(),
            version: "1.0 aethercore-os".to_string(),
        }
    }
}

// ── Aarch64Config ─────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct Aarch64Config {
    pub pci_ecam_bases: Vec<u64>,
    pub pci_scan_stop_on_first_hit: bool,
    pub pci_max_bus: u16,
    pub pci_max_device: u8,
    pub pci_max_function: u8,
    pub smp_boot_timeout_spins: u64,
    pub exception_kill_user_sync: bool,
    pub exception_kill_user_async: bool,
    pub exception_panic_on_kernel_sync: bool,
    pub exception_panic_on_kernel_async: bool,
    pub irq_storm_window_ticks: u64,
    pub irq_storm_threshold: u64,
    pub irq_storm_log_every: u64,
    pub timer_rearm_min_ticks: u64,
    pub timer_rearm_max_ticks: u64,
    pub timer_jitter_tolerance_ticks: u64,
    pub gic_cpu_priority_mask: u32,
    pub irq_rate_track_limit: usize,
    pub irq_per_line_storm_threshold: u64,
    pub irq_per_line_log_every: u64,
    pub tlb_shootdown_timeout_spins: usize,
    pub smp_known_mpidrs: Vec<u64>,
}

impl Default for Aarch64Config {
    fn default() -> Self {
        Self {
            pci_ecam_bases: vec![0x1000_0000, 0x3F00_0000],
            pci_scan_stop_on_first_hit: true,
            pci_max_bus: 255,
            pci_max_device: 31,
            pci_max_function: 7,
            smp_boot_timeout_spins: 5_000_000,
            exception_kill_user_sync: true,
            exception_kill_user_async: true,
            exception_panic_on_kernel_sync: true,
            exception_panic_on_kernel_async: true,
            irq_storm_window_ticks: 1_000_000,
            irq_storm_threshold: 1024,
            irq_storm_log_every: 256,
            timer_rearm_min_ticks: 1,
            timer_rearm_max_ticks: 50_000_000,
            timer_jitter_tolerance_ticks: 50_000,
            gic_cpu_priority_mask: 0xFF,
            irq_rate_track_limit: 256,
            irq_per_line_storm_threshold: 256,
            irq_per_line_log_every: 64,
            tlb_shootdown_timeout_spins: 1_000_000,
            smp_known_mpidrs: vec![0x0000_0001, 0x0000_0002, 0x0000_0003],
        }
    }
}

// ── LotteryConfig ─────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct LotteryConfig {
    pub initial_seed: u64,
    pub tickets_per_priority_level: u64,
    pub min_tickets_per_task: u64,
    pub replay_trace_capacity: usize,
    #[serde(deserialize_with = "deserialize_u64_or_string")]
    pub lcg_multiplier: u64,
    pub lcg_increment: u64,
}

impl Default for LotteryConfig {
    fn default() -> Self {
        Self {
            initial_seed: 0xCAFEBABE,
            tickets_per_priority_level: 10,
            min_tickets_per_task: 1,
            replay_trace_capacity: 128,
            lcg_multiplier: 6364136223846793005,
            lcg_increment: 1,
        }
    }
}

fn deserialize_u64_or_string<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Value {
        Integer(u64),
        String(String),
    }

    match Value::deserialize(deserializer)? {
        Value::Integer(value) => Ok(value),
        Value::String(value) => value.parse::<u64>().map_err(de::Error::custom),
    }
}

// ── RtosConfig ────────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct RtosConfig {
    pub strict_profile_enabled: bool,
    pub enforce_o1_scheduler_bounds: bool,
    pub disable_fast_path_alloc: bool,
    pub posix_compat_enabled: bool,
}

impl Default for RtosConfig {
    fn default() -> Self {
        Self {
            strict_profile_enabled: false,
            enforce_o1_scheduler_bounds: false,
            disable_fast_path_alloc: false,
            posix_compat_enabled: false,
        }
    }
}
