use super::*;

fn sample_snapshot() -> ProbeSnapshot {
    ProbeSnapshot {
        wayland_probe_ok: true,
        x11_probe_ok: true,
        fs_stack_ok: true,
        package_stack_ok: true,
        desktop_app_stack_ok: true,
        has_any_system_pkg_manager: true,
        seeded_system_pkg_manager_any: true,
        seeded_apt_available: true,
        seeded_pacman_available: false,
        seeded_diskfs_setup_available: true,
        seeded_pivot_root_setup_available: true,
        diskfs_bootstrap_telemetry_ok: true,
        seeded_abi_check_available: true,
        seeded_abi_contract_available: true,
        seeded_mirror_failover_available: true,
        seeded_signature_policy_available: true,
        seeded_checksum_policy_available: true,
        seeded_installer_policy_available: true,
        seeded_retry_timeout_available: true,
        seeded_apt_keyring_list_available: true,
        seeded_pacman_keyring_path_available: true,
        seeded_flutter_closure_audit_available: true,
        seeded_apt_capability_manifest_available: true,
        seeded_apt_host_limitation_note_available: false,
        has_min_dev_pkg_stack: true,
        language_pkg_manager_count: 3,
        has_desktop_session_runtime: true,
        has_flutter_runtime: true,
        seeded_flutter_runtime_available: true,
        desktop_install_capable: true,
        elf_so_runtime_contract_ok: true,
        wayland_x11_depth_ok: true,
        syscall_semantic_parity_ok: true,
        gpu_ioctl_coverage_ok: true,
        linux_host_e2e_pipeline_ok: true,
        cross_layer_health_surface_ok: true,
        parity_module_count: 8,
    }
}

#[test]
fn build_desktop_probes_emits_expected_contract_keys() {
    let snapshot = sample_snapshot();
    let probes = build_desktop_probes(&snapshot);

    assert_eq!(probes.get("wayland_probe_ok").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(probes.get("x11_probe_ok").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(probes.get("package_stack_ok").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(
        probes
            .get("runtime_seeded_retry_timeout_available")
            .and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        probes
            .get("runtime_language_package_manager_count")
            .and_then(|v| v.as_u64()),
        Some(3)
    );
    assert_eq!(
        probes
            .get("source_syscall_parity_module_count")
            .and_then(|v| v.as_u64()),
        Some(8)
    );
}

#[test]
fn probe_snapshot_fields_remain_accessible_for_semantic_tests() {
    let snapshot = sample_snapshot();

    assert!(snapshot.desktop_install_capable);
    assert!(snapshot.seeded_flutter_runtime_available);
    assert!(snapshot.elf_so_runtime_contract_ok);
    assert_eq!(snapshot.parity_module_count, 8);
}
