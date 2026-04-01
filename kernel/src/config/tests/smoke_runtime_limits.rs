use super::*;

#[test_case]
fn runtime_overrides_clamp_and_reset() {
    KernelConfig::reset_runtime_overrides();

    KernelConfig::apply_overrides(&[
        ("launch.max_process_name_len", ConfigValue::Usize(4096)),
        ("vfs.max_mount_path", ConfigValue::Usize(1)),
        ("irq.vector_base", ConfigValue::U8(255)),
    ])
    .unwrap();

    assert_eq!(KernelConfig::launch_max_process_name_len(), 32);
    assert_eq!(KernelConfig::vfs_max_mount_path(), 2);
    assert_eq!(KernelConfig::irq_vector_base(), 240);

    KernelConfig::reset_runtime_overrides();
    let limits = KernelConfig::runtime_limits();
    assert_eq!(limits.launch_max_process_name_len, 32);
    assert_eq!(
        limits.vfs_max_mount_path,
        crate::generated_consts::VFS_MAX_MOUNT_PATH
    );
    assert_eq!(limits.irq_vector_base, 32);
}

#[test_case]
fn module_loader_override_has_upper_bound() {
    KernelConfig::reset_runtime_overrides();

    KernelConfig::apply_overrides(&[(
        "module_loader.max_total_image_bytes",
        ConfigValue::U64(u64::MAX),
    )])
    .unwrap();
    assert_eq!(
        KernelConfig::module_loader_max_total_image_bytes(),
        16 * 1024 * 1024 * 1024
    );

    KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn telemetry_overrides_apply_and_reset() {
    KernelConfig::reset_runtime_overrides();

    KernelConfig::apply_overrides(&[
        ("telemetry.enabled", ConfigValue::Bool(true)),
        (
            "telemetry.runtime_summary_enabled",
            ConfigValue::Bool(false),
        ),
        ("telemetry.network_enabled", ConfigValue::Bool(false)),
        ("telemetry.history_len", ConfigValue::Usize(usize::MAX)),
        ("telemetry.log_level_num", ConfigValue::U8(99)),
    ])
    .unwrap();

    assert!(KernelConfig::is_telemetry_enabled());
    assert!(!KernelConfig::telemetry_runtime_summary_enabled());
    assert!(!KernelConfig::telemetry_network_enabled());
    assert_eq!(KernelConfig::telemetry_history_len(), 1_000_000);
    assert_eq!(KernelConfig::log_level_num(), 5);

    KernelConfig::reset_runtime_overrides();
    assert_eq!(
        KernelConfig::telemetry_history_len(),
        crate::generated_consts::TELEMETRY_HISTORY_LEN
    );
    assert_eq!(
        KernelConfig::log_level_num(),
        crate::generated_consts::LOG_LEVEL_NUM
    );
}

#[test_case]
fn library_overrides_clamp_and_reset() {
    KernelConfig::reset_runtime_overrides();

    KernelConfig::apply_overrides(&[
        (
            "network.loopback_queue_limit",
            ConfigValue::Usize(usize::MAX),
        ),
        ("network.epoll.max_events", ConfigValue::Usize(usize::MAX)),
        ("network.epoll.max_fds_per_instance", ConfigValue::Usize(0)),
        ("network.runtime_poll_interval_min", ConfigValue::U64(0)),
        ("libnet.posix_ephemeral_start", ConfigValue::U64(10)),
        ("diskfs.max_path_len", ConfigValue::Usize(2)),
        (
            "vfs.health.max_mount_failure_rate_per_mille",
            ConfigValue::U64(u64::MAX),
        ),
        (
            "vfs.health.max_unmount_failure_rate_per_mille",
            ConfigValue::U64(0),
        ),
        (
            "vfs.health.max_path_validation_failures",
            ConfigValue::U64(0),
        ),
        (
            "vfs.health.max_mount_capacity_percent",
            ConfigValue::Usize(usize::MAX),
        ),
        (
            "driver.network.quarantine_rebind_failures",
            ConfigValue::U64(u64::MAX),
        ),
        (
            "driver.network.quarantine_cooldown_samples",
            ConfigValue::U64(u64::MAX),
        ),
        (
            "driver.network.slo.max_drop_rate_per_mille",
            ConfigValue::U64(u64::MAX),
        ),
        (
            "driver.network.slo.max_tx_ring_utilization_percent",
            ConfigValue::U64(u64::MAX),
        ),
        (
            "driver.network.slo.max_rx_ring_utilization_percent",
            ConfigValue::U64(u64::MAX),
        ),
        (
            "driver.network.slo.max_io_errors",
            ConfigValue::U64(u64::MAX),
        ),
        (
            "driver.network.low_latency_irq_budget_divisor",
            ConfigValue::Usize(usize::MAX),
        ),
        (
            "driver.network.low_latency_loop_budget_divisor",
            ConfigValue::Usize(usize::MAX),
        ),
        (
            "driver.network.low_latency_ring_limit_divisor",
            ConfigValue::Usize(usize::MAX),
        ),
        (
            "driver.network.throughput_irq_budget_multiplier",
            ConfigValue::Usize(usize::MAX),
        ),
        (
            "driver.network.throughput_loop_budget_multiplier",
            ConfigValue::Usize(usize::MAX),
        ),
        (
            "driver.network.throughput_ring_limit_multiplier",
            ConfigValue::Usize(usize::MAX),
        ),
        ("ahci.io_timeout_spins", ConfigValue::Usize(usize::MAX)),
        (
            "nvme.disable_ready_timeout_spins",
            ConfigValue::Usize(usize::MAX),
        ),
        ("nvme.poll_timeout_spins", ConfigValue::Usize(usize::MAX)),
        ("nvme.io_timeout_spins", ConfigValue::Usize(usize::MAX)),
        ("e1000.reset_timeout_spins", ConfigValue::Usize(usize::MAX)),
        ("e1000.buffer_size_bytes", ConfigValue::Usize(usize::MAX)),
        ("e1000.rx_desc_count", ConfigValue::Usize(usize::MAX)),
        ("e1000.tx_desc_count", ConfigValue::Usize(usize::MAX)),
        ("network.slo.sample_interval", ConfigValue::U64(0)),
        (
            "network.slo.log_interval_multiplier",
            ConfigValue::U64(u64::MAX),
        ),
        (
            "network.tls_policy_profile",
            ConfigValue::TlsPolicy(super::TlsPolicyProfile::Strict),
        ),
    ])
    .unwrap();

    assert_eq!(KernelConfig::network_loopback_queue_limit(), 65_536);
    assert_eq!(KernelConfig::network_epoll_max_events(), 65_536);
    assert_eq!(KernelConfig::network_epoll_max_fds_per_instance(), 1);
    assert_eq!(KernelConfig::network_runtime_poll_interval_min(), 1);
    assert_eq!(KernelConfig::libnet_posix_ephemeral_start(), 40_000);
    assert_eq!(KernelConfig::diskfs_max_path_len(), 8);
    assert_eq!(
        KernelConfig::vfs_health_max_mount_failure_rate_per_mille(),
        1_000
    );
    assert_eq!(
        KernelConfig::vfs_health_max_unmount_failure_rate_per_mille(),
        50
    );
    assert_eq!(KernelConfig::vfs_health_max_path_validation_failures(), 1);
    assert_eq!(KernelConfig::vfs_health_max_mount_capacity_percent(), 100);
    assert_eq!(
        KernelConfig::driver_network_quarantine_rebind_failures(),
        1_000_000
    );
    assert_eq!(
        KernelConfig::driver_network_quarantine_cooldown_samples(),
        1_000_000
    );
    assert_eq!(
        KernelConfig::driver_network_slo_max_drop_rate_per_mille(),
        1000
    );
    assert_eq!(
        KernelConfig::driver_network_slo_max_tx_ring_utilization_percent(),
        100
    );
    assert_eq!(
        KernelConfig::driver_network_slo_max_rx_ring_utilization_percent(),
        100
    );
    assert_eq!(KernelConfig::driver_network_slo_max_io_errors(), u64::MAX);
    assert_eq!(
        KernelConfig::driver_network_low_latency_irq_budget_divisor(),
        1024
    );
    assert_eq!(
        KernelConfig::driver_network_low_latency_loop_budget_divisor(),
        1024
    );
    assert_eq!(
        KernelConfig::driver_network_low_latency_ring_limit_divisor(),
        1024
    );
    assert_eq!(
        KernelConfig::driver_network_throughput_irq_budget_multiplier(),
        1024
    );
    assert_eq!(
        KernelConfig::driver_network_throughput_loop_budget_multiplier(),
        1024
    );
    assert_eq!(
        KernelConfig::driver_network_throughput_ring_limit_multiplier(),
        1024
    );
    assert_eq!(KernelConfig::ahci_io_timeout_spins(), 100_000_000);
    assert_eq!(
        KernelConfig::nvme_disable_ready_timeout_spins(),
        100_000_000
    );
    assert_eq!(KernelConfig::nvme_poll_timeout_spins(), 100_000_000);
    assert_eq!(KernelConfig::nvme_io_timeout_spins(), 100_000_000);
    assert_eq!(KernelConfig::e1000_reset_timeout_spins(), 100_000_000);
    assert_eq!(KernelConfig::e1000_buffer_size_bytes(), 16_384);
    assert_eq!(KernelConfig::e1000_rx_desc_count(), 4096);
    assert_eq!(KernelConfig::e1000_tx_desc_count(), 4096);
    assert_eq!(KernelConfig::network_slo_sample_interval(), 1);
    assert_eq!(
        KernelConfig::network_slo_log_interval_multiplier(),
        1_000_000_000
    );
    assert_eq!(
        KernelConfig::network_tls_policy_profile(),
        super::TlsPolicyProfile::Strict
    );

    KernelConfig::reset_runtime_overrides();
    assert_eq!(KernelConfig::network_loopback_queue_limit(), 128);
    assert_eq!(KernelConfig::network_epoll_max_events(), 1024);
    assert_eq!(KernelConfig::network_epoll_max_fds_per_instance(), 4096);
    assert_eq!(KernelConfig::network_runtime_poll_interval_min(), 1);
    assert_eq!(KernelConfig::libnet_posix_ephemeral_start(), 40_000);
    assert_eq!(
        KernelConfig::vfs_health_max_mount_failure_rate_per_mille(),
        50
    );
    assert_eq!(
        KernelConfig::vfs_health_max_unmount_failure_rate_per_mille(),
        50
    );
    assert_eq!(KernelConfig::vfs_health_max_path_validation_failures(), 8);
    assert_eq!(KernelConfig::vfs_health_max_mount_capacity_percent(), 90);
    assert_eq!(
        KernelConfig::network_tls_policy_profile(),
        super::TlsPolicyProfile::Balanced
    );
    assert_eq!(
        KernelConfig::network_slo_sample_interval(),
        crate::generated_consts::NETWORK_SLO_SAMPLE_INTERVAL
    );
    assert_eq!(
        KernelConfig::network_slo_log_interval_multiplier(),
        crate::generated_consts::NETWORK_SLO_LOG_INTERVAL_MULTIPLIER
    );
    assert_eq!(KernelConfig::diskfs_max_path_len(), 255);
    assert_eq!(
        KernelConfig::driver_network_quarantine_rebind_failures(),
        crate::generated_consts::DRIVER_NETWORK_QUARANTINE_REBIND_FAILURES
    );
    assert_eq!(
        KernelConfig::driver_network_quarantine_cooldown_samples(),
        crate::generated_consts::DRIVER_NETWORK_QUARANTINE_COOLDOWN_SAMPLES
    );
    assert_eq!(
        KernelConfig::driver_network_slo_max_drop_rate_per_mille(),
        25
    );
    assert_eq!(
        KernelConfig::driver_network_slo_max_tx_ring_utilization_percent(),
        90
    );
    assert_eq!(
        KernelConfig::driver_network_slo_max_rx_ring_utilization_percent(),
        90
    );
    assert_eq!(KernelConfig::driver_network_slo_max_io_errors(), 0);
    assert_eq!(
        KernelConfig::driver_network_low_latency_irq_budget_divisor(),
        4
    );
    assert_eq!(
        KernelConfig::driver_network_low_latency_loop_budget_divisor(),
        2
    );
    assert_eq!(
        KernelConfig::driver_network_low_latency_ring_limit_divisor(),
        2
    );
    assert_eq!(
        KernelConfig::driver_network_throughput_irq_budget_multiplier(),
        4
    );
    assert_eq!(
        KernelConfig::driver_network_throughput_loop_budget_multiplier(),
        4
    );
    assert_eq!(
        KernelConfig::driver_network_throughput_ring_limit_multiplier(),
        4
    );
    assert_eq!(KernelConfig::ahci_io_timeout_spins(), 2_000_000);
    assert_eq!(KernelConfig::nvme_disable_ready_timeout_spins(), 100_000);
    assert_eq!(KernelConfig::nvme_poll_timeout_spins(), 500_000);
    assert_eq!(KernelConfig::nvme_io_timeout_spins(), 1_000_000);
    assert_eq!(KernelConfig::e1000_reset_timeout_spins(), 1_000_000);
    assert_eq!(KernelConfig::e1000_buffer_size_bytes(), 2048);
    assert_eq!(KernelConfig::e1000_rx_desc_count(), 32);
    assert_eq!(KernelConfig::e1000_tx_desc_count(), 256);
}

#[test_case]
fn scheduler_overrides_clamp_and_reset() {
    KernelConfig::reset_runtime_overrides();

    KernelConfig::apply_overrides(&[
        (
            "scheduler.cfs_min_granularity_ns",
            ConfigValue::U64(u64::MAX),
        ),
        ("scheduler.cfs_latency_target_ns", ConfigValue::U64(1)),
        ("scheduler.mlfq_base_slice_ns", ConfigValue::U64(u64::MAX)),
        (
            "scheduler.mlfq_boost_interval_ticks",
            ConfigValue::U64(u64::MAX),
        ),
        (
            "scheduler.mlfq_demote_on_slice_exhaustion",
            ConfigValue::Bool(false),
        ),
        ("scheduler.edf_enforce_deadline", ConfigValue::Bool(false)),
        (
            "scheduler.edf_default_relative_deadline_ns",
            ConfigValue::U64(u64::MAX),
        ),
        (
            "scheduler.rt_group_reservation_enabled",
            ConfigValue::Bool(false),
        ),
        ("scheduler.rt_period_ns", ConfigValue::U64(u64::MAX)),
        (
            "scheduler.rt_total_utilization_cap_percent",
            ConfigValue::U64(u8::MAX as u64),
        ),
        ("scheduler.rt_max_groups", ConfigValue::Usize(usize::MAX)),
    ])
    .unwrap();

    assert_eq!(KernelConfig::cfs_min_granularity_ns(), 10_000_000_000);
    assert_eq!(
        KernelConfig::cfs_latency_target_ns(),
        KernelConfig::cfs_min_granularity_ns()
    );
    assert_eq!(KernelConfig::mlfq_base_slice_ns(), 60_000_000_000);
    assert_eq!(KernelConfig::mlfq_boost_interval_ticks(), 1_000_000_000);
    assert!(!KernelConfig::mlfq_demote_on_slice_exhaustion());
    assert!(!KernelConfig::edf_enforce_deadline());
    assert_eq!(
        KernelConfig::edf_default_relative_deadline_ns(),
        60_000_000_000
    );
    assert!(!KernelConfig::rt_group_reservation_enabled());
    assert_eq!(KernelConfig::rt_period_ns(), 60_000_000_000);
    assert_eq!(KernelConfig::rt_total_utilization_cap_percent(), 100);
    assert_eq!(KernelConfig::rt_max_groups(), 65_536);

    KernelConfig::reset_runtime_overrides();
    assert_eq!(
        KernelConfig::cfs_min_granularity_ns(),
        crate::generated_consts::SCHED_CFS_MIN_GRANULARITY_NS
    );
    assert_eq!(
        KernelConfig::cfs_latency_target_ns(),
        crate::generated_consts::SCHED_CFS_LATENCY_TARGET_NS
    );
    assert_eq!(
        KernelConfig::mlfq_base_slice_ns(),
        crate::generated_consts::SCHED_MLFQ_BASE_SLICE_NS
    );
    assert_eq!(
        KernelConfig::mlfq_boost_interval_ticks(),
        crate::generated_consts::SCHED_MLFQ_BOOST_INTERVAL_TICKS
    );
    assert_eq!(
        KernelConfig::mlfq_demote_on_slice_exhaustion(),
        crate::generated_consts::SCHED_MLFQ_DEMOTE_ON_SLICE_EXHAUSTION
    );
    assert_eq!(
        KernelConfig::edf_enforce_deadline(),
        crate::generated_consts::SCHED_EDF_ENFORCE_DEADLINE
    );
    assert_eq!(
        KernelConfig::edf_default_relative_deadline_ns(),
        crate::generated_consts::SCHED_EDF_DEFAULT_RELATIVE_DEADLINE_NS
    );
    assert_eq!(
        KernelConfig::rt_group_reservation_enabled(),
        crate::generated_consts::SCHED_RT_ENABLE_GROUP_RESERVATION
    );
    assert_eq!(
        KernelConfig::rt_period_ns(),
        crate::generated_consts::SCHED_RT_PERIOD_NS
    );
    assert_eq!(
        KernelConfig::rt_total_utilization_cap_percent(),
        crate::generated_consts::SCHED_RT_TOTAL_UTILIZATION_CAP_PERCENT
    );
    assert_eq!(
        KernelConfig::rt_max_groups(),
        crate::generated_consts::SCHED_RT_MAX_GROUPS
    );
}
