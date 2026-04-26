use super::helpers::run_source_probe;
use super::{Layer, Totals};

pub(super) struct ProbeSnapshot {
    pub(super) wayland_probe_ok: bool,
    pub(super) x11_probe_ok: bool,
    pub(super) fs_stack_ok: bool,
    pub(super) package_stack_ok: bool,
    pub(super) desktop_app_stack_ok: bool,
    pub(super) has_any_system_pkg_manager: bool,
    pub(super) seeded_system_pkg_manager_any: bool,
    pub(super) seeded_apt_available: bool,
    pub(super) seeded_pacman_available: bool,
    pub(super) seeded_diskfs_setup_available: bool,
    pub(super) seeded_pivot_root_setup_available: bool,
    pub(super) diskfs_bootstrap_telemetry_ok: bool,
    pub(super) seeded_abi_check_available: bool,
    pub(super) seeded_abi_contract_available: bool,
    pub(super) seeded_mirror_failover_available: bool,
    pub(super) seeded_signature_policy_available: bool,
    pub(super) seeded_checksum_policy_available: bool,
    pub(super) seeded_installer_policy_available: bool,
    pub(super) seeded_retry_timeout_available: bool,
    pub(super) seeded_apt_keyring_list_available: bool,
    pub(super) seeded_pacman_keyring_path_available: bool,
    pub(super) seeded_flutter_closure_audit_available: bool,
    pub(super) seeded_apt_capability_manifest_available: bool,
    pub(super) seeded_apt_host_limitation_note_available: bool,
    pub(super) has_min_dev_pkg_stack: bool,
    pub(super) language_pkg_manager_count: usize,
    pub(super) has_desktop_session_runtime: bool,
    pub(super) has_flutter_runtime: bool,
    pub(super) seeded_flutter_runtime_available: bool,
    pub(super) desktop_install_capable: bool,
    pub(super) elf_so_runtime_contract_ok: bool,
    pub(super) wayland_x11_depth_ok: bool,
    pub(super) syscall_semantic_parity_ok: bool,
    pub(super) gpu_ioctl_coverage_ok: bool,
    pub(super) linux_host_e2e_pipeline_ok: bool,
    pub(super) cross_layer_health_surface_ok: bool,
    pub(super) parity_module_count: usize,
}

pub(super) struct ProbeRequirements {
    pub(super) require_wayland: bool,
    pub(super) require_x11: bool,
    pub(super) require_fs_stack: bool,
    pub(super) require_package_stack: bool,
    pub(super) require_desktop_app_stack: bool,
    pub(super) strict: bool,
    pub(super) quick: bool,
}

pub(super) fn maybe_print_package_stack_diagnostics(
    snapshot: &ProbeSnapshot,
    require_package_stack: bool,
) {
    if require_package_stack && !snapshot.package_stack_ok {
        println!(
            "[test::linux-app-compat] package-stack diagnostics: seeded_pkg_mgr={} seeded_apt={} abi_check={} abi_contract={} apt_seed_manifest={} elf_so_contract={}",
            snapshot.seeded_system_pkg_manager_any,
            snapshot.seeded_apt_available,
            snapshot.seeded_abi_check_available,
            snapshot.seeded_abi_contract_available,
            snapshot.seeded_apt_capability_manifest_available,
            snapshot.elf_so_runtime_contract_ok,
        );
        println!(
            "[test::linux-app-compat] package-stack policy diagnostics: mirror_failover={} retry_timeout={} signature_policy={} checksum_policy={} apt_keyring_list={} pacman_keyring_path={} flutter_closure_audit={}",
            snapshot.seeded_mirror_failover_available,
            snapshot.seeded_retry_timeout_available,
            snapshot.seeded_signature_policy_available,
            snapshot.seeded_checksum_policy_available,
            snapshot.seeded_apt_keyring_list_available,
            snapshot.seeded_pacman_keyring_path_available,
            snapshot.seeded_flutter_closure_audit_available,
        );
        if snapshot.seeded_apt_host_limitation_note_available {
            println!(
                "[test::linux-app-compat] hint: apt seed was built on a non-Unix host; run apt-iso on Linux for real apt/dpkg/.so runtime closure"
            );
        }
    }
}

pub(super) fn run_probe_suite(
    compat: &mut Layer,
    totals: &mut Totals,
    requirements: &ProbeRequirements,
    snapshot: &ProbeSnapshot,
) {
    run_source_probe(
        compat,
        totals,
        "wayland userspace graphics probe",
        snapshot.wayland_probe_ok,
        requirements.require_wayland,
    );
    run_source_probe(
        compat,
        totals,
        "x11 userspace graphics probe",
        snapshot.x11_probe_ok,
        requirements.require_x11,
    );
    run_source_probe(
        compat,
        totals,
        "filesystem stack probe (devfs/tmpfs/procfs/sysfs/ext4/fat/overlay)",
        snapshot.fs_stack_ok,
        requirements.require_fs_stack,
    );
    run_source_probe(
        compat,
        totals,
        "linux package install stack probe",
        snapshot.package_stack_ok,
        requirements.require_package_stack,
    );
    run_source_probe(
        compat,
        totals,
        "desktop app stack probe (XFCE/GNOME/Flutter prerequisites)",
        snapshot.desktop_app_stack_ok,
        requirements.require_desktop_app_stack,
    );
    run_source_probe(
        compat,
        totals,
        "wayland/x11 protocol-depth probe (request/reply/event/object lifecycle prefixes)",
        snapshot.wayland_x11_depth_ok,
        requirements.strict && !requirements.quick,
    );
    run_source_probe(
        compat,
        totals,
        "syscall semantic parity suite probe",
        snapshot.syscall_semantic_parity_ok,
        true,
    );
    run_source_probe(
        compat,
        totals,
        "gpu ioctl coverage inventory probe",
        snapshot.gpu_ioctl_coverage_ok,
        requirements.strict && !requirements.quick,
    );
    run_source_probe(
        compat,
        totals,
        "linux-host e2e install proof pipeline probe",
        snapshot.linux_host_e2e_pipeline_ok,
        true,
    );
    run_source_probe(
        compat,
        totals,
        "ELF shared-object runtime contract probe (PT_INTERP/DT_NEEDED/dlopen)",
        snapshot.elf_so_runtime_contract_ok,
        true,
    );
    run_source_probe(
        compat,
        totals,
        "cross-layer health surface probe (fs/net/syscalls/gpu)",
        snapshot.cross_layer_health_surface_ok,
        true,
    );
}

pub(super) fn build_desktop_probes(
    snapshot: &ProbeSnapshot,
) -> serde_json::Map<String, serde_json::Value> {
    let mut desktop_probes = serde_json::Map::new();

    macro_rules! insert_probe {
        ($key:expr, $val:expr) => {
            desktop_probes.insert($key.to_string(), serde_json::json!($val));
        };
    }

    insert_probe!("wayland_probe_ok", snapshot.wayland_probe_ok);
    insert_probe!("x11_probe_ok", snapshot.x11_probe_ok);
    insert_probe!("fs_stack_ok", snapshot.fs_stack_ok);
    insert_probe!("package_stack_ok", snapshot.package_stack_ok);
    insert_probe!("desktop_app_stack_ok", snapshot.desktop_app_stack_ok);
    insert_probe!(
        "runtime_system_package_manager_any",
        snapshot.has_any_system_pkg_manager
    );
    insert_probe!(
        "runtime_seeded_system_package_manager_any",
        snapshot.seeded_system_pkg_manager_any
    );
    insert_probe!(
        "runtime_seeded_apt_available",
        snapshot.seeded_apt_available
    );
    insert_probe!(
        "runtime_seeded_pacman_available",
        snapshot.seeded_pacman_available
    );
    insert_probe!(
        "runtime_seeded_diskfs_setup_available",
        snapshot.seeded_diskfs_setup_available
    );
    insert_probe!(
        "runtime_seeded_pivot_root_setup_available",
        snapshot.seeded_pivot_root_setup_available
    );
    insert_probe!(
        "source_diskfs_bootstrap_telemetry_ok",
        snapshot.diskfs_bootstrap_telemetry_ok
    );
    insert_probe!(
        "runtime_seeded_abi_check_available",
        snapshot.seeded_abi_check_available
    );
    insert_probe!(
        "runtime_seeded_abi_contract_available",
        snapshot.seeded_abi_contract_available
    );
    insert_probe!(
        "runtime_seeded_mirror_failover_available",
        snapshot.seeded_mirror_failover_available
    );
    insert_probe!(
        "runtime_seeded_signature_policy_available",
        snapshot.seeded_signature_policy_available
    );
    insert_probe!(
        "runtime_seeded_checksum_policy_available",
        snapshot.seeded_checksum_policy_available
    );
    insert_probe!(
        "runtime_seeded_installer_policy_available",
        snapshot.seeded_installer_policy_available
    );
    insert_probe!(
        "runtime_seeded_retry_timeout_available",
        snapshot.seeded_retry_timeout_available
    );
    insert_probe!(
        "runtime_seeded_apt_keyring_list_available",
        snapshot.seeded_apt_keyring_list_available
    );
    insert_probe!(
        "runtime_seeded_pacman_keyring_path_available",
        snapshot.seeded_pacman_keyring_path_available
    );
    insert_probe!(
        "runtime_seeded_flutter_closure_audit_available",
        snapshot.seeded_flutter_closure_audit_available
    );
    insert_probe!(
        "runtime_seeded_apt_capability_manifest_available",
        snapshot.seeded_apt_capability_manifest_available
    );
    insert_probe!(
        "runtime_seeded_apt_host_limitation_note_available",
        snapshot.seeded_apt_host_limitation_note_available
    );
    insert_probe!(
        "runtime_dev_package_stack_ok",
        snapshot.has_min_dev_pkg_stack
    );
    insert_probe!(
        "runtime_language_package_manager_count",
        snapshot.language_pkg_manager_count
    );
    insert_probe!(
        "runtime_desktop_session_available",
        snapshot.has_desktop_session_runtime
    );
    insert_probe!("runtime_flutter_available", snapshot.has_flutter_runtime);
    insert_probe!(
        "runtime_seeded_flutter_available",
        snapshot.seeded_flutter_runtime_available
    );
    insert_probe!(
        "runtime_desktop_install_capable",
        snapshot.desktop_install_capable
    );
    insert_probe!(
        "source_elf_so_runtime_contract_ok",
        snapshot.elf_so_runtime_contract_ok
    );
    insert_probe!(
        "source_syscall_semantic_parity_ok",
        snapshot.syscall_semantic_parity_ok
    );
    insert_probe!(
        "source_gpu_ioctl_coverage_ok",
        snapshot.gpu_ioctl_coverage_ok
    );
    insert_probe!(
        "source_linux_host_e2e_pipeline_ok",
        snapshot.linux_host_e2e_pipeline_ok
    );
    insert_probe!(
        "source_cross_layer_health_surface_ok",
        snapshot.cross_layer_health_surface_ok
    );
    insert_probe!(
        "source_wayland_x11_protocol_depth_ok",
        snapshot.wayland_x11_depth_ok
    );
    insert_probe!(
        "source_syscall_parity_module_count",
        snapshot.parity_module_count
    );

    desktop_probes
}

#[cfg(test)]
#[path = "probe_output_tests.rs"]
mod tests;
