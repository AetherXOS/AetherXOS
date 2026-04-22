use crate::utils::paths;

use super::helpers::{
    command_exists, count_occurrences, file_contains, file_contains_all,
};
use super::probe_output::{
    ProbeRequirements, ProbeSnapshot, build_desktop_probes, maybe_print_package_stack_diagnostics,
    run_probe_suite,
};
use super::{constants, Layer, LinuxAppCompatOptions, Totals};

pub(super) fn run_runtime_probes(
    compat: &mut Layer,
    totals: &mut Totals,
    opts: &LinuxAppCompatOptions,
) -> serde_json::Map<String, serde_json::Value> {
    let desktop_smoke = opts.desktop_smoke;
    let quick = opts.quick;
    let strict = opts.strict;
    let require_wayland = opts.require_wayland || desktop_smoke;
    let require_x11 = opts.require_x11 || desktop_smoke;
    let require_fs_stack = opts.require_fs_stack;
    let require_package_stack = opts.require_package_stack;
    let require_desktop_app_stack = opts.require_desktop_app_stack || desktop_smoke;

    let root = paths::repo_root();
    let wayland_mod = paths::kernel_src("modules/userspace_graphics/wayland/mod.rs");
    let wayland_proto = paths::kernel_src("modules/userspace_graphics/wayland/protocol.rs");
    let x11_mod = paths::kernel_src("modules/userspace_graphics/x11/mod.rs");
    let x11_proto = paths::kernel_src("modules/userspace_graphics/x11/protocol.rs");
    let vfs_mod = paths::kernel_src("modules/vfs/mod.rs");
    let linux_mount_setup = paths::kernel_src("modules/vfs/linux_mount_setup.rs");
    let mount_table = paths::kernel_src("modules/vfs/mount_table.rs");
    let disk_fs = paths::kernel_src("modules/vfs/disk_fs.rs");
    let writeback = paths::kernel_src("modules/vfs/writeback.rs");
    let writeback_tests = paths::kernel_src("modules/vfs/writeback/tests.rs");
    let diskfs_bootstrap = root.join("boot/initramfs/usr/bin/aethercore-diskfs-setup");
    let pivot_root_setup = root.join("boot/initramfs/usr/bin/aethercore-pivot-root");
    let userspace_seed = root.join("xtask/src/commands/infra/userspace_seed.rs");
    let apt_seed = root.join("xtask/src/commands/infra/apt_binary_seed.rs");
    let syscall_consts = paths::kernel_src("kernel/syscalls/syscalls_consts.rs");
    let syscall_dispatch = paths::kernel_src("kernel/syscalls/mod.rs");
    let dynamic_linker = paths::kernel_src("kernel/dynamic_linker.rs");
    let dynamic_linker_entry = paths::kernel_src("kernel/dynamic_linker_entry.rs");
    let dynamic_linker_helpers = paths::kernel_src("kernel/dynamic_linker_helpers.rs");
    let so_loader_load = paths::kernel_src("kernel/so_loader/load.rs");
    let so_loader_mod = paths::kernel_src("kernel/so_loader/mod.rs");
    let dl_api = paths::kernel_src("kernel/api.rs");
    let vfs_health = paths::kernel_src("modules/vfs/health.rs");
    let network_metrics_snapshot = paths::kernel_src("modules/network/metrics_snapshot.rs");
    let syscall_stats_api = paths::kernel_src("kernel/syscalls/stats_api.rs");
    let gpu_mod = paths::kernel_src("modules/drivers/gpu/mod.rs");
    let gpu_tests = paths::kernel_src("modules/drivers/gpu/tests.rs");
    let linux_shim_dispatch = paths::kernel_src("kernel/syscalls/linux_shim/dispatch.rs");
    let linux_compat_io = paths::kernel_src("modules/linux_compat/fs/io.rs");
    let kernel_tests_mod = paths::kernel_src("kernel/tests/mod.rs");
    let linux_abi_platform = root.join("xtask/src/commands/validation/linux_abi/platform.rs");
    let linux_abi_desktop_plan =
        root.join("xtask/src/commands/validation/linux_abi/desktop_plan.rs");
    let linux_host_e2e_script = root.join("scripts/linux_host_e2e_proof.sh");
    let linux_host_e2e_doc = root.join("docs/LINUX_HOST_E2E_PROOF_PIPELINE.md");
    let gpu_ioctl_inventory = root.join("config/gpu_ioctl_coverage_p0.json");
    let posix_lifecycle = paths::kernel_src("modules/posix/fs/lifecycle_support.rs");
    let cargo_toml = root.join("Cargo.toml");

    let wayland_probe_ok = file_contains(&wayland_mod, "validate_client_handshake_prefix")
        && file_contains(&wayland_mod, "validate_registry_advertisement_path")
        && file_contains(&wayland_mod, "validate_registry_bind_prefix")
        && file_contains(&wayland_proto, "parse_wire_header")
        && file_contains(&wayland_proto, "is_complete_frame");

    let x11_probe_ok = file_contains(&x11_mod, "validate_client_setup_request")
        && file_contains(&x11_mod, "validate_server_reply_prefix")
        && file_contains(&x11_mod, "validate_core_opcode_dispatch_prefix")
        && file_contains(&x11_proto, "parse_setup_prefix")
        && file_contains(&x11_proto, "parse_server_packet_prefix");

    let wayland_x11_depth_ok = wayland_probe_ok
        && x11_probe_ok
        && file_contains_all(
            &wayland_mod,
            constants::WAYLAND_CLIENT_REQUIRED_PREFIXES,
        )
        && file_contains_all(
            &x11_mod,
            constants::X11_CLIENT_REQUIRED_PREFIXES,
        )
        && file_contains_all(
            &x11_proto,
            constants::X11_PROTO_REQUIRED_PREFIXES,
        )
        && file_contains_all(
            &linux_abi_platform,
            &["score_graphics_stack", "wayland_stack", "x11_stack"],
        )
        && file_contains_all(
            &linux_abi_desktop_plan,
            &["graphics_readiness", "score_stack", "Wayland", "X11"],
        );

    let has_any_system_pkg_manager = constants::SYSTEM_PKG_MANAGERS.iter().any(|pm| command_exists(pm));

    let generated_seed_root = root.join("artifacts/boot_image/generated/initramfs_apt");
    let seeded_apt_available = generated_seed_root.join("usr/bin/apt-get").exists()
        || generated_seed_root.join("usr/bin/apt").exists();
    let seeded_pacman_available = generated_seed_root.join("usr/bin/pacman").exists();
    let seeded_diskfs_setup_available = generated_seed_root
        .join("usr/bin/aethercore-diskfs-setup")
        .exists();
    let seeded_pivot_root_setup_available = generated_seed_root
        .join("usr/bin/aethercore-pivot-root")
        .exists();
    let diskfs_bootstrap_telemetry_ok = file_contains_all(
        &diskfs_bootstrap,
        constants::DISKFS_BOOTSTRAP_TELEMETRY_EVENTS,
    );
    let seeded_abi_check_available = generated_seed_root
        .join("usr/bin/aethercore-userspace-abi-check")
        .exists();
    let seeded_abi_contract_available = generated_seed_root
        .join("usr/lib/aethercore/userspace-apt-abi-contract.txt")
        .exists();
    let seeded_flutter_closure_audit_available = generated_seed_root
        .join("usr/lib/aethercore/flutter-runtime-closure-audit.json")
        .exists();
    let seeded_apt_capability_manifest_available = generated_seed_root
        .join("usr/lib/aethercore/apt-seed-capability.json")
        .exists();
    let seeded_apt_host_limitation_note_available = generated_seed_root
        .join("usr/lib/aethercore/apt-seed-host-limitation.txt")
        .exists();
    let seeded_mirror_failover_available = generated_seed_root
        .join("etc/aethercore/mirror-failover.list")
        .exists();
    let seeded_signature_policy_available = generated_seed_root
        .join("etc/aethercore/metadata-signature-policy.conf")
        .exists();
    let seeded_checksum_policy_available = generated_seed_root
        .join("etc/aethercore/checksum-policy.conf")
        .exists();
    let seeded_installer_policy_available = generated_seed_root
        .join("etc/aethercore/installer-policy.json")
        .exists();
    let seeded_retry_timeout_available = generated_seed_root
        .join("etc/aethercore/installer-timeout.conf")
        .exists();
    let seeded_apt_keyring_list_available = generated_seed_root
        .join("etc/aethercore/apt-trusted-keyrings.list")
        .exists();
    let seeded_pacman_keyring_path_available = generated_seed_root
        .join("etc/aethercore/pacman-keyring-dir.path")
        .exists();
    let seeded_system_pkg_manager_any = seeded_apt_available || seeded_pacman_available;

    let language_pkg_manager_count = constants::LANGUAGE_PKG_MANAGERS
        .iter()
        .filter(|pm| command_exists(pm))
        .count();
    let has_min_dev_pkg_stack = language_pkg_manager_count >= 2;

    let has_desktop_session_runtime = constants::DESKTOP_SESSION_BINARIES
        .iter()
        .any(|bin| command_exists(bin));
    let has_flutter_runtime = command_exists("flutter");
    let seeded_flutter_runtime_available = generated_seed_root.join("usr/bin/flutter").exists()
        || generated_seed_root
            .join("usr/bin/flutter-wrapper.sh")
            .exists()
        || generated_seed_root.join("opt/flutter/bin/flutter").exists()
        || generated_seed_root.join("usr/lib/libflutter.so").exists();
    let desktop_install_capable = has_any_system_pkg_manager && has_min_dev_pkg_stack;

    let fs_stack_ok = file_contains(&posix_lifecycle, "mount_devfs")
        && file_contains(&posix_lifecycle, "mount_ramfs")
        && file_contains_all(
            &diskfs_bootstrap,
            &["probing block devices", "mount -t", "/var/lib/aethercore"],
        )
        && file_contains_all(
            &pivot_root_setup,
            constants::PIVOT_ROOT_SETUP_VARS,
        )
        && file_contains_all(
            &linux_mount_setup,
            &[
                "setup_linux_vfs_mounts",
                "mount_table::mount",
                "FsType::Procfs",
                "FsType::Sysfs",
                "FsType::Tmpfs",
                "/dev/pts",
                "/dev/shm",
                "/run",
            ],
        )
        && file_contains_all(
            &mount_table,
            constants::LINUX_MOUNT_TYPES,
        )
        && file_contains_all(
            &disk_fs,
            &["DiskFsMode", "Fat", "Little", "Ext4", "with_backend"],
        )
        && file_contains_all(
            &writeback,
            &[
                "JournalTransaction",
                "journal_write",
                "journal_commit",
                "JournalOp",
            ],
        )
        && file_contains_all(
            &writeback_tests,
            &[
                "journal_and_writeback_recovery_chain_preserves_replay_until_checkpoint",
                "crash_recovery_soak_chain_handles_multiple_transactions_revoke_and_checkpoint",
            ],
        )
        && file_contains_all(
            &vfs_mod,
            &["pub mod tmpfs", "pub mod procfs", "pub mod sysfs"],
        );

    let package_stack_foundation_ok = file_contains_all(
        &cargo_toml,
        &[
            "linux_compat = [",
            "posix_process",
            "posix_pipe",
            "posix_mman",
            "posix_fs",
            "posix_net",
        ],
    ) && (has_any_system_pkg_manager
        || seeded_system_pkg_manager_any)
        && has_min_dev_pkg_stack
        && file_contains_all(&disk_fs, &["pub fn write_all", "pub fn read_all"])
        && file_contains_all(&linux_mount_setup, &["/tmp", "/run", "/dev/shm"])
        && file_contains(&syscall_consts, "VFS_MOUNT_DISKFS")
        && file_contains(&syscall_dispatch, "sys_vfs_mount_diskfs")
        && file_contains_all(
            &userspace_seed,
            &[
                "userspace-apt-abi-contract.txt",
                "aethercore-userspace-abi-check",
                "apt-xz-utils-probe-failed",
                "mirror-failover.list",
                "run_with_retry",
                "RETRY_MAX",
                "RETRY_BACKOFF",
                "metadata_signature_required",
                "validate_repo_metadata",
                "gpgv",
                "pacman-key",
            ],
        )
        && file_contains_all(
            &apt_seed,
            &[
                "apt-seed-capability.json",
                "package_stack_ready",
                "non-unix-build-host",
            ],
        );

    let elf_so_runtime_contract_ok =
        file_contains_all(
            &dynamic_linker,
            &["pub mod so_loader", "dynamic_linker_entry"],
        ) && file_contains_all(
            &dynamic_linker_entry,
            &[
                "SharedObjectLoader::with_search_paths",
                "dt_needed",
                "process_relocations",
                "resolve_runtime_search_paths",
            ],
        ) && file_contains_all(
            &dynamic_linker_helpers,
            &[
                "sanitize_search_paths",
                "resolve_runtime_search_paths",
                "starts_with('/')",
            ],
        ) && file_contains_all(
            &so_loader_load,
            &[
                "load_needed",
                "load_recursive",
                "/lib/",
                "/usr/lib/",
                "validate_soname",
                "dt_soname",
            ],
        ) && file_contains_all(
            &so_loader_mod,
            &[
                "find_symbol_in_object_versioned",
                "find_symbol_versioned",
                "return None;",
            ],
        ) && file_contains_all(&dl_api, &["dlopen", "dlsym", "dlclose"]);

    let package_stack_ok = if require_package_stack {
        package_stack_foundation_ok
            && seeded_system_pkg_manager_any
            && seeded_diskfs_setup_available
            && seeded_pivot_root_setup_available
            && seeded_abi_check_available
            && seeded_abi_contract_available
            && seeded_mirror_failover_available
            && seeded_signature_policy_available
            && seeded_checksum_policy_available
            && seeded_installer_policy_available
            && seeded_retry_timeout_available
            && seeded_apt_keyring_list_available
            && seeded_pacman_keyring_path_available
            && seeded_apt_capability_manifest_available
            && seeded_flutter_closure_audit_available
            && elf_so_runtime_contract_ok
    } else {
        package_stack_foundation_ok
    };

    let flutter_runtime_available = has_flutter_runtime || seeded_flutter_runtime_available;

    let desktop_app_stack_foundation_ok = wayland_probe_ok
        && x11_probe_ok
        && (has_desktop_session_runtime || desktop_install_capable)
        && (flutter_runtime_available || desktop_install_capable)
        && file_contains_all(
            &cargo_toml,
            &[
                "linux_userspace_wayland",
                "linux_userspace_x11",
                "linux_compat_vfs",
                "ipc_dbus",
                "network_https",
                "posix_thread",
                "posix_process",
            ],
        );

    let desktop_app_stack_ok = if require_desktop_app_stack {
        desktop_app_stack_foundation_ok && seeded_flutter_runtime_available
    } else {
        desktop_app_stack_foundation_ok
    };

    let parity_module_count = count_occurrences(&kernel_tests_mod, "_parity;");
    let syscall_semantic_parity_ok = parity_module_count >= 8
        && file_contains_all(
            &kernel_tests_mod,
            constants::SYSCALL_SEMANTIC_PARITY_TESTS,
        )
        && file_contains_all(
            &syscall_stats_api,
            &[
                "SyscallHealthReport",
                "evaluate_syscall_health",
                "recommended_syscall_health_action",
            ],
        );

    let gpu_ioctl_coverage_ok = file_contains_all(
        &linux_shim_dispatch,
        &["linux_nr::IOCTL", "sys_linux_ioctl"],
    ) && file_contains_all(
        &linux_compat_io,
        &["pub fn sys_linux_ioctl", "ioctl", "posix::fs::ioctl"],
    ) && file_contains_all(
        &gpu_mod,
        &["GpuBackend", "VirtIoGpu", "gpu_stack_snapshot"],
    ) && file_contains_all(
        &gpu_tests,
        &[
            "gpu_desktop_path_readiness_requires_non_none_backend",
            "gpu_health_detects_stale_heartbeat",
        ],
    ) && file_contains_all(
        &gpu_ioctl_inventory,
        constants::GPU_IOCTL_COVERAGE_REQS,
    );

    let linux_host_e2e_pipeline_ok = file_contains_all(
        &linux_host_e2e_script,
        &[
            "#!/usr/bin/env bash",
            "cargo run -p xtask -- build apt-iso",
            "cargo run -p xtask -- ops qemu smoke",
            "cargo run -p xtask -- test linux-app-compat --strict --ci",
            "reports/linux_host_e2e_proof",
        ],
    ) && file_contains_all(
        &linux_host_e2e_doc,
        &[
            "Linux host only",
            "qemu smoke",
            "strict linux-app-compat",
            "Acceptance Criteria",
        ],
    );

    let cross_layer_health_surface_ok = file_contains_all(
        &vfs_health,
        &[
            "VfsHealthSummary",
            "current_mount_health_summary",
            "summarize_mount_health",
        ],
    ) && file_contains_all(
        &network_metrics_snapshot,
        &["runtime_health_report", "recommended_runtime_health_action"],
    ) && file_contains_all(
        &syscall_stats_api,
        &[
            "SyscallHealthReport",
            "evaluate_syscall_health",
            "recommended_syscall_health_action",
        ],
    ) && file_contains_all(
        &gpu_mod,
        &[
            "GpuHealthReport",
            "evaluate_gpu_health",
            "recommended_gpu_health_action",
        ],
    );

    let snapshot = ProbeSnapshot {
        wayland_probe_ok,
        x11_probe_ok,
        fs_stack_ok,
        package_stack_ok,
        desktop_app_stack_ok,
        has_any_system_pkg_manager,
        seeded_system_pkg_manager_any,
        seeded_apt_available,
        seeded_pacman_available,
        seeded_diskfs_setup_available,
        seeded_pivot_root_setup_available,
        diskfs_bootstrap_telemetry_ok,
        seeded_abi_check_available,
        seeded_abi_contract_available,
        seeded_mirror_failover_available,
        seeded_signature_policy_available,
        seeded_checksum_policy_available,
        seeded_installer_policy_available,
        seeded_retry_timeout_available,
        seeded_apt_keyring_list_available,
        seeded_pacman_keyring_path_available,
        seeded_flutter_closure_audit_available,
        seeded_apt_capability_manifest_available,
        seeded_apt_host_limitation_note_available,
        has_min_dev_pkg_stack,
        language_pkg_manager_count,
        has_desktop_session_runtime,
        has_flutter_runtime,
        seeded_flutter_runtime_available,
        desktop_install_capable,
        elf_so_runtime_contract_ok,
        wayland_x11_depth_ok,
        syscall_semantic_parity_ok,
        gpu_ioctl_coverage_ok,
        linux_host_e2e_pipeline_ok,
        cross_layer_health_surface_ok,
        parity_module_count,
    };

    maybe_print_package_stack_diagnostics(&snapshot, require_package_stack);

    let requirements = ProbeRequirements {
        require_wayland,
        require_x11,
        require_fs_stack,
        require_package_stack,
        require_desktop_app_stack,
        strict,
        quick,
    };
    run_probe_suite(compat, totals, &requirements, &snapshot);

    build_desktop_probes(&snapshot)
}
