use super::*;

#[test_case]
fn parses_bool_shorthand_and_reset_entries() {
    let (key, value) = parse_override_entry("telemetry_enabled").expect("bool shorthand");
    assert_eq!(key, "telemetry_enabled");
    assert_eq!(value.as_deref(), Some("true"));

    let (key, value) = parse_override_entry("!telemetry_enabled").expect("bool false");
    assert_eq!(key, "telemetry_enabled");
    assert_eq!(value.as_deref(), Some("false"));

    let (key, value) = parse_override_entry("reset.telemetry_enabled").expect("reset");
    assert_eq!(key, "telemetry_enabled");
    assert_eq!(value, None);
}

#[test_case]
fn rejects_non_bool_shorthand_without_value() {
    let err = parse_override_entry("launch_max_boot_image_bytes").unwrap_err();
    assert_eq!(err.0, "launch_max_boot_image_bytes");
    assert_eq!(err.1, ConfigSetError::TypeMismatch);
}

#[test_case]
fn batch_apply_updates_runtime_override() {
    KernelConfig::reset_runtime_overrides();
    assert!(KernelConfig::is_telemetry_enabled());

    let applied =
        KernelConfig::apply_override_batch_str("!telemetry_enabled, telemetry_history_len=2048")
            .expect("apply config batch");
    assert_eq!(applied, 2);
    assert!(!KernelConfig::is_telemetry_enabled());
    assert_eq!(KernelConfig::telemetry_history_len(), 2048);

    KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn kernel_cmdline_parser_applies_prefixed_tokens() {
    KernelConfig::reset_runtime_overrides();
    let count = KernelConfig::apply_kernel_cmdline_overrides(
        "quiet cfg.telemetry_enabled=false cfg:telemetry_history_len=1024 config=network_runtime_poll_interval_min=4,!telemetry_runtime_summary_enabled",
    )
    .expect("apply cmdline config");
    assert_eq!(count, 4);
    assert!(!KernelConfig::is_telemetry_enabled());
    assert_eq!(KernelConfig::telemetry_history_len(), 1024);
    assert_eq!(KernelConfig::network_runtime_poll_interval_min(), 4);
    assert!(!KernelConfig::telemetry_runtime_summary_enabled());
    KernelConfig::reset_runtime_overrides();
}

#[cfg(feature = "vfs")]
#[test_case]
fn feature_summary_report_contains_drift_and_category_rows() {
    let rows = [
        ConfigFeatureCategorySummary {
            category: "telemetry",
            total: 2,
            compile_enabled: 2,
            runtime_gateable: 1,
            effective_enabled: 1,
        },
        ConfigFeatureCategorySummary {
            category: "networking",
            total: 3,
            compile_enabled: 2,
            runtime_gateable: 2,
            effective_enabled: 2,
        },
    ];

    let report = build_feature_summary_report(&rows, 4);
    assert!(report.contains("feature_runtime_drift_count=4"));
    assert!(report.contains("category=telemetry total=2"));
    assert!(report.contains("category=networking total=3"));
}

#[cfg(feature = "vfs")]
#[test_case]
fn readiness_report_includes_blockers_and_recommended_action() {
    let readiness = ConfigLinuxCompatReadiness {
        compile_linux_compat: true,
        compile_vfs: true,
        boundary_allows_compat: false,
        vfs_api_exposed: true,
        network_api_exposed: false,
        ipc_api_exposed: false,
        proc_config_api_exposed: false,
        sysctl_api_exposed: false,
        effective_surface_enabled: false,
    };
    let blockers = [ConfigLinuxCompatBlocker {
        code: "boundary_mode_strict_blocks_compat_surface",
        severity: ConfigBlockerSeverity::High,
        next_action: "set library_boundary_mode to Balanced or Compat at runtime",
    }];

    let report = build_linux_compat_readiness_report(
        &readiness,
        blockers.as_slice(),
        "set library_boundary_mode to Balanced or Compat at runtime",
    );
    assert!(report.contains("compile_linux_compat=true"));
    assert!(report.contains("effective_surface_enabled=false"));
    assert!(report.contains("blocker=boundary_mode_strict_blocks_compat_surface"));
    assert!(report.contains("severity=high"));
    assert!(report.contains("recommended_next_action=set library_boundary_mode to Balanced or Compat at runtime"));
}

#[cfg(feature = "vfs")]
#[test_case]
fn security_posture_report_contains_gate_state_and_reasons() {
    let snapshot = crate::kernel::security_posture::current_snapshot();
    let gate = crate::kernel::security_posture::strict_profile_gate_report();
    let release_gate = crate::kernel::security_posture::release_gate_decision();
    let report = build_security_posture_report(&snapshot, &gate, &release_gate);

    assert!(report.contains("boundary_mode="));
    assert!(report.contains("deployment_context="));
    assert!(report.contains("strict_gate_deployment_context="));
    assert!(report.contains("strict_gate_passed="));
    assert!(report.contains("syscall_contract_passed="));
    assert!(report.contains("namespace_policy_passed="));
    assert!(report.contains("namespace_lifecycle_sane="));
    assert!(report.contains("release_gate_blocked="));
    assert!(report.contains("release_gate_deployment_context="));
    assert!(report.contains("release_gate_reason_count="));
    if gate.reasons.is_empty() {
        assert!(report.contains("strict_gate_reasons=none"));
    } else {
        assert!(report.contains("strict_gate_reason="));
    }
}

#[cfg(feature = "vfs")]
#[test_case]
fn vfs_behavior_report_contains_operation_and_path_metrics() {
    let report = build_vfs_behavior_report();
    assert!(report.contains("vfs_max_mounts="));
    assert!(report.contains("mount_attempts="));
    assert!(report.contains("path_validation_failures="));
    assert!(report.contains("initrd_load_calls="));
    assert!(report.contains("mounts_reachable="));
}

#[cfg(feature = "vfs")]
#[test_case]
fn vfs_matrix_report_contains_operation_rows_and_feature_totals() {
    let report = build_vfs_matrix_report();
    assert!(report.contains("feature_inventory_total="));
    assert!(report.contains("feature_categories:"));
    assert!(report.contains("operation_matrix:"));
    assert!(report.contains("op=open default=required"));
    assert!(report.contains("op=rename default=optional"));
    assert!(report.contains("op=lock default=unlock-only"));
    assert!(report.contains("matrix_scores:"));
    assert!(report.contains("fs=ramfs readiness_score="));
    assert!(report.contains("band="));
    assert!(report.contains("operation_scores:"));
    assert!(report.contains("op=open aggregate_score="));
    assert!(report.contains("matrix_overall_score="));
    assert!(report.contains("required_operation_gap_count="));
    assert!(report.contains("matrix_gate="));
    assert!(report.contains("matrix_gate_reason="));
    assert!(report.contains("required_gaps_by_fs:"));
    assert!(report.contains("fs=ramfs required_gap_count="));
    assert!(report.contains("operation_hotspots_count="));
    assert!(report.contains("operation_hotspot="));
    assert!(report.contains("next_focus_count="));
    assert!(report.contains("next_focus="));
    assert!(report.contains("weak_filesystems_count="));
    assert!(report.contains("strong_filesystems_count="));
    assert!(report.contains("weak_filesystem="));
    assert!(report.contains("weakest_filesystem="));
    assert!(report.contains("recommended_action="));
    assert!(report.contains("xattr_get_calls="));
    assert!(report.contains("library_backends:"));
}

#[cfg(feature = "vfs")]
#[test_case]
fn vfs_focus_report_contains_gate_and_focus_lines() {
    let report = build_vfs_focus_report();
    assert!(report.contains("matrix_gate="));
    assert!(report.contains("matrix_gate_reason="));
    assert!(report.contains("matrix_overall_score="));
    assert!(report.contains("required_operation_gap_count="));
    assert!(report.contains("next_focus_count="));
    assert!(report.contains("next_focus="));
    assert!(report.contains("weak_filesystems_count="));
    assert!(report.contains("weak_filesystem="));
    assert!(report.contains("strong_filesystems_count="));
    assert!(report.contains("weakest_filesystem="));
    assert!(report.contains("recommended_action="));
}

#[cfg(feature = "vfs")]
#[test_case]
fn runtime_keys_report_preserves_line_boundaries() {
    let keys = [
        String::from("telemetry_enabled [telemetry] bool critical=true"),
        String::from("network_epoll_max_events [network] usize critical=false"),
    ];
    let report = build_runtime_keys_report(&keys);
    let lines: Vec<&str> = report.lines().collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], keys[0]);
    assert_eq!(lines[1], keys[1]);
}
