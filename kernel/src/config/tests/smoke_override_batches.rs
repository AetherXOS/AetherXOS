use super::*;

#[test_case]
fn runtime_override_template_is_sorted_and_annotated() {
    let template = KernelConfig::runtime_override_template();
    assert_eq!(template.len(), KernelConfig::runtime_config_catalog().len());
    assert!(template.iter().all(|line| line.contains(" critical=")));

    let mut previous: Option<(alloc::string::String, alloc::string::String)> = None;
    for line in &template {
        let key = alloc::string::String::from(line.split_whitespace().next().expect("key token"));
        let category_start = line.find('[').expect("category start");
        let category_end = line.find(']').expect("category end");
        let category = alloc::string::String::from(&line[category_start + 1..category_end]);
        let current = (category, key);
        if let Some(prev) = &previous {
            assert!(prev <= &current);
        }
        previous = Some(current);
    }

    assert!(
        template
            .iter()
            .any(|line| line.starts_with("telemetry_enabled") && line.contains("critical=true"))
    );
}

#[test_case]
fn override_batch_preview_flags_invalid_entries_without_mutation() {
    KernelConfig::reset_runtime_overrides();
    let baseline = KernelConfig::network_epoll_max_events();

    let preview = KernelConfig::preview_override_batch_str(
        "network.epoll.max_events=2048,network.epoll.max_events=not_a_number,not.exists=1",
    );
    assert_eq!(preview.len(), 3);
    assert!(preview[0].valid);
    assert!(!preview[1].valid);
    assert_eq!(preview[1].cause, Some(ConfigSetError::InvalidValue));
    assert!(!preview[2].valid);
    assert_eq!(preview[2].cause, Some(ConfigSetError::UnknownKey));

    assert_eq!(KernelConfig::network_epoll_max_events(), baseline);

    KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn override_preview_marks_critical_keys() {
    KernelConfig::reset_runtime_overrides();

    let preview =
        KernelConfig::preview_override_batch_str("telemetry.enabled=false, network.epoll.max_events=1024");
    assert_eq!(preview.len(), 2);
    assert!(preview[0].critical);
    assert_eq!(preview[0].category, Some("telemetry"));
    assert!(preview[1].valid);

    KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn override_preview_summary_counts_valid_invalid_and_critical() {
    KernelConfig::reset_runtime_overrides();
    let baseline = KernelConfig::network_epoll_max_events();

    let summary = KernelConfig::preview_override_batch_summary_str(
        "telemetry.enabled=false, network.epoll.max_events=2048, network.epoll.max_events=bad,not.exists=1",
    );
    assert_eq!(summary.total, 4);
    assert_eq!(summary.valid, 2);
    assert_eq!(summary.invalid, 2);
    assert_eq!(summary.critical_touched, 1);

    assert_eq!(KernelConfig::network_epoll_max_events(), baseline);

    KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn strict_override_batch_rejects_disabling_critical_keys() {
    KernelConfig::reset_runtime_overrides();
    super::assert_batch_error(
        KernelConfig::apply_override_batch_strict("telemetry_enabled=false"),
        0,
        "telemetry_enabled",
        ConfigSetError::InvalidValue,
    );
    KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn strict_override_batch_rejects_reset_for_critical_keys() {
    KernelConfig::reset_runtime_overrides();
    super::assert_batch_error(
        KernelConfig::apply_override_batch_strict("reset.telemetry_enabled"),
        0,
        "telemetry_enabled",
        ConfigSetError::InvalidValue,
    );
    KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn strict_override_batch_accepts_non_critical_changes() {
    KernelConfig::reset_runtime_overrides();

    let applied = KernelConfig::apply_override_batch_strict("network_epoll_max_events=4096")
        .expect("strict mode should allow non-critical updates");
    assert_eq!(applied, 1);
    assert_eq!(KernelConfig::network_epoll_max_events(), 4096);

    KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn override_batch_rejects_invalid_enum_payloads() {
    KernelConfig::reset_runtime_overrides();

    let cases = [
        ("network_tls_policy_profile=ultra", "network_tls_policy_profile"),
        ("library_boundary_mode=unsafe", "library_boundary_mode"),
        ("devfs_policy_profile=wild", "devfs_policy_profile"),
        (
            "virtualization_execution_profile=realtime",
            "virtualization_execution_profile",
        ),
        (
            "virtualization_governor_profile=turbo",
            "virtualization_governor_profile",
        ),
    ];

    for (raw, key) in cases {
        super::assert_batch_error(
            KernelConfig::apply_override_batch_str(raw),
            0,
            key,
            ConfigSetError::InvalidValue,
        );
    }

    KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn cmdline_overrides_report_mixed_batch_error_with_token_offset() {
    KernelConfig::reset_runtime_overrides();

    super::assert_batch_error(
        KernelConfig::apply_kernel_cmdline_overrides(
            "quiet cfg.telemetry_enabled=false config=network_runtime_poll_interval_min=4,network_tls_policy_profile=invalid cfg.network_epoll_max_events=2048",
        ),
        2,
        "network_tls_policy_profile",
        ConfigSetError::InvalidValue,
    );

    assert!(!KernelConfig::is_telemetry_enabled());
    assert_eq!(KernelConfig::network_runtime_poll_interval_min(), 4);

    KernelConfig::reset_runtime_overrides();
}

#[cfg(feature = "vfs")]
fn read_ramfs_text_file(mount_id: usize, path: &str) -> alloc::string::String {
    let mut file = crate::kernel::vfs_control::ramfs_open_file(
        mount_id,
        path,
        crate::interfaces::TaskId(0),
    )
    .expect("open exported file");

    let mut bytes = alloc::vec::Vec::new();
    loop {
        let mut chunk = [0u8; 256];
        let n = file.read(&mut chunk).expect("read exported file");
        if n == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..n]);
    }

    alloc::string::String::from_utf8(bytes).expect("utf8 export")
}

#[cfg(feature = "vfs")]
#[test_case]
fn export_snapshot_to_mount_writes_expected_schema_files() {
    KernelConfig::reset_runtime_overrides();

    let mount_id = crate::kernel::vfs_control::mount_ramfs(b"/cfg_export_test").expect("mount");
    let exported = KernelConfig::export_snapshot_to_mount("/cfg_export_test", "/config_dump")
        .expect("export snapshot");
    assert!(exported >= 5);

    let snapshot = read_ramfs_text_file(mount_id, "/config_dump/snapshot.txt");
    let features = read_ramfs_text_file(mount_id, "/config_dump/features.txt");
    let summary = read_ramfs_text_file(mount_id, "/config_dump/feature_summary.txt");
    let readiness = read_ramfs_text_file(mount_id, "/config_dump/linux_compat_readiness.txt");
    let keys = read_ramfs_text_file(mount_id, "/config_dump/runtime_keys.txt");

    assert!(snapshot.contains("audit.apply_attempts="));
    assert!(snapshot.contains("library.boundary_mode="));
    assert!(features.contains("category="));
    assert!(summary.contains("feature_runtime_drift_count="));
    assert!(readiness.contains("recommended_next_action="));
    assert!(keys.contains("critical="));

    crate::kernel::vfs_control::unmount(mount_id).expect("unmount");
    KernelConfig::reset_runtime_overrides();
}
