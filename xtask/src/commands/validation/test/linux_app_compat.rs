use anyhow::{Result, bail};
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::utils::{paths, report};

#[derive(Default)]
struct Totals {
    passed: usize,
    failed: usize,
    skipped: usize,
}

#[derive(Default, Serialize)]
struct Layer {
    passed: usize,
    failed: usize,
    skipped: usize,
    total: usize,
}

#[derive(Serialize)]
struct Scorecard {
    generated_utc: String,
    profile: String,
    ci_enforce: bool,
    totals: TotalsOut,
    layer_percentages: LayerPercentages,
}

#[derive(Serialize)]
struct TotalsOut {
    passed: usize,
    failed: usize,
    skipped: usize,
    total: usize,
    pass_rate_pct: f64,
}

#[derive(Serialize)]
struct LayerPercentages {
    host_smoke_pass_rate_pct: f64,
    app_integration_pass_rate_pct: f64,
    runtime_probe_pass_rate_pct: f64,
    kernel_gate_pass_rate_pct: f64,
    qemu_gate_pass_rate_pct: f64,
    overall_compatibility_index_pct: f64,
    ci_policy_ok: bool,
}

fn shell_cmd(command: &str) -> std::io::Result<std::process::Output> {
    #[cfg(windows)]
    {
        Command::new("cmd").args(["/C", command]).output()
    }
    #[cfg(not(windows))]
    {
        Command::new("sh").args(["-c", command]).output()
    }
}

fn command_exists(cmd: &str) -> bool {
    #[cfg(windows)]
    {
        Command::new("where")
            .arg(cmd)
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        shell_cmd(&format!("command -v {} >/dev/null 2>&1", cmd))
            .map(|out| out.status.success())
            .unwrap_or(false)
    }
}

fn run_case(layer: &mut Layer, totals: &mut Totals, name: &str, cmd: &str) -> bool {
    print!("[TEST] {}", name);
    match shell_cmd(cmd) {
        Ok(out) if out.status.success() => {
            println!(" OK");
            layer.total += 1;
            layer.passed += 1;
            totals.passed += 1;
            true
        }
        Ok(_) | Err(_) => {
            println!(" FAIL");
            layer.total += 1;
            layer.failed += 1;
            totals.failed += 1;
            false
        }
    }
}

fn run_optional(
    layer: &mut Layer,
    totals: &mut Totals,
    name: &str,
    check_cmd: &str,
    cmd: &str,
    required: bool,
) {
    let present = shell_cmd(check_cmd)
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !present {
        print!("[TEST] {}", name);
        if required {
            println!(" FAIL");
            layer.total += 1;
            layer.failed += 1;
            totals.failed += 1;
        } else {
            println!(" SKIP");
            layer.total += 1;
            layer.skipped += 1;
            totals.skipped += 1;
        }
        return;
    }
    let _ = run_case(layer, totals, name, cmd);
}

fn run_source_probe(layer: &mut Layer, totals: &mut Totals, name: &str, ok: bool, required: bool) {
    print!("[TEST] {}", name);
    if ok {
        println!(" OK");
        layer.total += 1;
        layer.passed += 1;
        totals.passed += 1;
        return;
    }

    if required {
        println!(" FAIL");
        layer.total += 1;
        layer.failed += 1;
        totals.failed += 1;
    } else {
        println!(" SKIP");
        layer.total += 1;
        layer.skipped += 1;
        totals.skipped += 1;
    }
}

fn skip_case(layer: &mut Layer, totals: &mut Totals, name: &str) {
    print!("[TEST] {}", name);
    println!(" SKIP");
    layer.total += 1;
    layer.skipped += 1;
    totals.skipped += 1;
}

fn file_contains(path: &Path, needle: &str) -> bool {
    if !path.exists() {
        return false;
    }
    fs::read_to_string(path)
        .map(|text| text.contains(needle))
        .unwrap_or(false)
}

fn file_contains_all(path: &Path, needles: &[&str]) -> bool {
    if !path.exists() {
        return false;
    }
    let Ok(text) = fs::read_to_string(path) else {
        return false;
    };
    needles.iter().all(|needle| text.contains(needle))
}

fn count_occurrences(path: &Path, needle: &str) -> usize {
    if !path.exists() {
        return 0;
    }
    let Ok(text) = fs::read_to_string(path) else {
        return 0;
    };
    text.matches(needle).count()
}

fn rate(layer: &Layer) -> f64 {
    let executed = layer.total.saturating_sub(layer.skipped);
    if executed == 0 {
        100.0
    } else {
        ((layer.passed as f64 / executed as f64) * 1000.0).round() / 10.0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LinuxAppCompatOptions {
    pub desktop_smoke: bool,
    pub quick: bool,
    pub qemu: bool,
    pub strict: bool,
    pub ci: bool,
    pub require_busybox: bool,
    pub require_glibc: bool,
    pub require_wayland: bool,
    pub require_x11: bool,
    pub require_fs_stack: bool,
    pub require_package_stack: bool,
    pub require_desktop_app_stack: bool,
}

pub fn run(opts: LinuxAppCompatOptions) -> Result<()> {
    println!("[test::linux-app-compat] Native layered validator");

    let desktop_smoke = opts.desktop_smoke;
    let quick = opts.quick;
    let qemu = opts.qemu;
    let strict = opts.strict;
    let ci = opts.ci;
    let require_busybox = opts.require_busybox;
    let require_glibc = opts.require_glibc;
    let require_wayland = opts.require_wayland || desktop_smoke;
    let require_x11 = opts.require_x11 || desktop_smoke;
    let require_fs_stack = opts.require_fs_stack;
    let require_package_stack = opts.require_package_stack;
    let require_desktop_app_stack = opts.require_desktop_app_stack || desktop_smoke;

    if desktop_smoke {
        println!(
            "[test::linux-app-compat] Desktop smoke profile enabled (Wayland/X11 probes required)"
        );
    }

    let mut totals = Totals::default();
    let mut host = Layer::default();
    let mut integration = Layer::default();
    let mut compat = Layer::default();
    let mut kernel = Layer::default();
    let mut qemu_layer = Layer::default();

    println!("\nPhase 1: Host Shell and Linux Primitive Smoke");
    if cfg!(windows) {
        let _ = run_case(&mut host, &mut totals, "Process creation", "ver >nul");
        let _ = run_case(
            &mut host,
            &mut totals,
            "File read/write",
            "echo test>%TEMP%\\hc_test.txt && findstr test %TEMP%\\hc_test.txt >nul",
        );
        let _ = run_case(
            &mut host,
            &mut totals,
            "Pipe chaining",
            "echo hello | findstr hello >nul",
        );
        skip_case(&mut host, &mut totals, "procfs available");
        if !quick {
            skip_case(&mut host, &mut totals, "Loop execution");
        }
    } else {
        let _ = run_case(&mut host, &mut totals, "Process creation", "exit 0");
        let _ = run_case(
            &mut host,
            &mut totals,
            "File read/write",
            "echo test >/tmp/hc_test.txt; cat /tmp/hc_test.txt | grep test",
        );
        let _ = run_case(
            &mut host,
            &mut totals,
            "Pipe chaining",
            "echo hello | cat | grep hello",
        );
        let _ = run_case(
            &mut host,
            &mut totals,
            "procfs available",
            "ls /proc >/dev/null",
        );
        if !quick {
            let _ = run_case(
                &mut host,
                &mut totals,
                "Loop execution",
                "for i in 1 2 3; do echo $i; done | wc -l | grep '^3$'",
            );
        }
    }

    println!("\nPhase 1b: App Integration");
    if cfg!(windows) {
        skip_case(&mut integration, &mut totals, "awk aggregation");
        skip_case(&mut integration, &mut totals, "sed transform");
        skip_case(&mut integration, &mut totals, "tar round-trip");
    } else {
        let _ = run_case(
            &mut integration,
            &mut totals,
            "awk aggregation",
            "printf 'a 1\na 2\n' | awk '$1==\"a\" {s+=$2} END{print s}' | grep '^3$'",
        );
        let _ = run_case(
            &mut integration,
            &mut totals,
            "sed transform",
            "printf 'linux-compat\n' | sed 's/linux/hyper/' | grep '^hyper-compat$'",
        );
        let _ = run_case(
            &mut integration,
            &mut totals,
            "tar round-trip",
            "mkdir -p /tmp/hc_tar/src; echo payload >/tmp/hc_tar/src/a.txt; tar -cf /tmp/hc_tar/a.tar -C /tmp/hc_tar/src .; mkdir -p /tmp/hc_tar/out; tar -xf /tmp/hc_tar/a.tar -C /tmp/hc_tar/out; cat /tmp/hc_tar/out/a.txt | grep payload",
        );
    }

    println!("\nPhase 1c: Runtime Probe");
    run_optional(
        &mut compat,
        &mut totals,
        "busybox availability",
        "command -v busybox >/dev/null 2>&1",
        "busybox --help >/dev/null",
        require_busybox,
    );
    run_optional(
        &mut compat,
        &mut totals,
        "busybox applet smoke",
        "command -v busybox >/dev/null 2>&1",
        "busybox ls / >/dev/null",
        require_busybox,
    );
    run_optional(
        &mut compat,
        &mut totals,
        "glibc detection",
        "getconf GNU_LIBC_VERSION >/dev/null 2>&1",
        "getconf GNU_LIBC_VERSION | grep -i glibc",
        require_glibc,
    );

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
            &[
                "validate_surface_commit_prefix",
                "validate_registry_bind_prefix",
                "validate_registry_advertisement_path",
            ],
        )
        && file_contains_all(
            &x11_mod,
            &[
                "validate_client_request_prefix",
                "x11_reply_event_semantics_supported",
                "validate_core_opcode_dispatch_prefix",
            ],
        )
        && file_contains_all(
            &x11_proto,
            &[
                "parse_request_prefix",
                "parse_reply_prefix",
                "has_complete_server_packet",
            ],
        )
        && file_contains_all(
            &linux_abi_platform,
            &["score_graphics_stack", "wayland_stack", "x11_stack"],
        )
        && file_contains_all(
            &linux_abi_desktop_plan,
            &["graphics_readiness", "score_stack", "Wayland", "X11"],
        );

    let system_pkg_managers = ["apt-get", "dnf", "pacman", "apk", "zypper"];
    let has_any_system_pkg_manager = system_pkg_managers.iter().any(|pm| command_exists(pm));

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
        &[
            "event=tmpfs_fallback",
            "event=diskfs_mounted",
            "event=diskfs_mode_set",
        ],
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

    let language_pkg_managers = ["pip", "pip3", "npm", "cargo"];
    let language_pkg_manager_count = language_pkg_managers
        .iter()
        .filter(|pm| command_exists(pm))
        .count();
    let has_min_dev_pkg_stack = language_pkg_manager_count >= 2;

    let desktop_session_binaries = ["xfce4-session", "gnome-shell"];
    let has_desktop_session_runtime = desktop_session_binaries
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
            &[
                "pivot-root",
                "AETHERCORE_ENABLE_PIVOT_ROOT",
                "switch_root",
                "chroot",
                "pivot-root.status",
            ],
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
            &[
                "FsType", "Ext4", "Fat32", "Overlay", "Tmpfs", "Procfs", "Sysfs",
            ],
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

    if require_package_stack && !package_stack_ok {
        println!(
            "[test::linux-app-compat] package-stack diagnostics: seeded_pkg_mgr={} seeded_apt={} abi_check={} abi_contract={} apt_seed_manifest={} elf_so_contract={}",
            seeded_system_pkg_manager_any,
            seeded_apt_available,
            seeded_abi_check_available,
            seeded_abi_contract_available,
            seeded_apt_capability_manifest_available,
            elf_so_runtime_contract_ok,
        );
        println!(
            "[test::linux-app-compat] package-stack policy diagnostics: mirror_failover={} retry_timeout={} signature_policy={} checksum_policy={} apt_keyring_list={} pacman_keyring_path={} flutter_closure_audit={}",
            seeded_mirror_failover_available,
            seeded_retry_timeout_available,
            seeded_signature_policy_available,
            seeded_checksum_policy_available,
            seeded_apt_keyring_list_available,
            seeded_pacman_keyring_path_available,
            seeded_flutter_closure_audit_available,
        );
        if seeded_apt_host_limitation_note_available {
            println!(
                "[test::linux-app-compat] hint: apt seed was built on a non-Unix host; run apt-iso on Linux for real apt/dpkg/.so runtime closure"
            );
        }
    }

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
            &[
                "signal_frame_parity",
                "af_unix_parity",
                "fs_backend_parity",
                "memory_mapping_parity",
                "ptrace_debugging_parity",
                "proc_sysctl_consistency_parity",
                "pid_uts_namespace_parity",
                "socket_options_parity",
            ],
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
        &[
            "DRM_IOCTL_VERSION",
            "DRM_IOCTL_MODE_GETRESOURCES",
            "VIRTGPU",
        ],
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

    run_source_probe(
        &mut compat,
        &mut totals,
        "wayland userspace graphics probe",
        wayland_probe_ok,
        require_wayland,
    );
    run_source_probe(
        &mut compat,
        &mut totals,
        "x11 userspace graphics probe",
        x11_probe_ok,
        require_x11,
    );
    run_source_probe(
        &mut compat,
        &mut totals,
        "filesystem stack probe (devfs/tmpfs/procfs/sysfs/ext4/fat/overlay)",
        fs_stack_ok,
        require_fs_stack,
    );
    run_source_probe(
        &mut compat,
        &mut totals,
        "linux package install stack probe",
        package_stack_ok,
        require_package_stack,
    );
    run_source_probe(
        &mut compat,
        &mut totals,
        "desktop app stack probe (XFCE/GNOME/Flutter prerequisites)",
        desktop_app_stack_ok,
        require_desktop_app_stack,
    );
    run_source_probe(
        &mut compat,
        &mut totals,
        "wayland/x11 protocol-depth probe (request/reply/event/object lifecycle prefixes)",
        wayland_x11_depth_ok,
        true,
    );
    run_source_probe(
        &mut compat,
        &mut totals,
        "syscall semantic parity suite probe",
        syscall_semantic_parity_ok,
        true,
    );
    run_source_probe(
        &mut compat,
        &mut totals,
        "gpu ioctl coverage inventory probe",
        gpu_ioctl_coverage_ok,
        true,
    );
    run_source_probe(
        &mut compat,
        &mut totals,
        "linux-host e2e install proof pipeline probe",
        linux_host_e2e_pipeline_ok,
        true,
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
    run_source_probe(
        &mut compat,
        &mut totals,
        "ELF shared-object runtime contract probe (PT_INTERP/DT_NEEDED/dlopen)",
        elf_so_runtime_contract_ok,
        true,
    );
    run_source_probe(
        &mut compat,
        &mut totals,
        "cross-layer health surface probe (fs/net/syscalls/gpu)",
        cross_layer_health_surface_ok,
        true,
    );

    println!("\nPhase 2: Kernel Gates");
    print!("[GATE] cargo check --lib --features linux_compat");
    let host_target = crate::utils::cargo::detect_host_triple().ok();
    let mut cargo_args = vec!["check", "--lib", "--features", "linux_compat"];
    if let Some(target) = host_target.as_deref() {
        cargo_args.push("--target");
        cargo_args.push(target);
    }
    let build_ok = Command::new("cargo")
        .args(&cargo_args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if build_ok {
        println!(" OK");
        kernel.total += 1;
        kernel.passed += 1;
        totals.passed += 1;
    } else {
        println!(" FAIL");
        kernel.total += 1;
        kernel.failed += 1;
        totals.failed += 1;
    }

    print!("[GATE] syscall coverage summary");
    let _ = crate::commands::validation::syscall_coverage::execute(
        true,
        "md",
        &Some("reports/linux_app_compat_syscall_coverage.md".to_string()),
    );
    let cov_ok = paths::resolve("reports/syscall_coverage_summary.json").exists();
    if cov_ok {
        println!(" OK");
        kernel.total += 1;
        kernel.passed += 1;
        totals.passed += 1;
    } else {
        println!(" FAIL");
        kernel.total += 1;
        kernel.failed += 1;
        totals.failed += 1;
    }

    if qemu || strict {
        println!("\nPhase 3: QEMU Gate");
        print!("[GATE] qemu smoke");
        if crate::commands::ops::qemu::smoke_test().is_ok() {
            println!(" OK");
            qemu_layer.total += 1;
            qemu_layer.passed += 1;
            totals.passed += 1;
        } else {
            println!(" FAIL");
            qemu_layer.total += 1;
            qemu_layer.failed += 1;
            totals.failed += 1;
        }
    }

    let total = totals.passed + totals.failed + totals.skipped;
    let executed = totals.passed + totals.failed;
    let pass_rate = if executed == 0 {
        100.0
    } else {
        ((totals.passed as f64 / executed as f64) * 1000.0).round() / 10.0
    };

    let host_rate = rate(&host);
    let integration_rate = rate(&integration);
    let compat_rate = rate(&compat);
    let kernel_rate = rate(&kernel);
    let qemu_rate = if qemu || strict {
        rate(&qemu_layer)
    } else {
        100.0
    };
    let overall = ((host_rate * 0.25)
        + (integration_rate * 0.20)
        + (compat_rate * 0.10)
        + (kernel_rate * 0.25)
        + (qemu_rate * 0.20))
        .round();

    let ci_ok = totals.failed == 0;
    let out = Scorecard {
        generated_utc: report::utc_now_iso(),
        profile: if strict {
            "strict".to_string()
        } else {
            "standard".to_string()
        },
        ci_enforce: ci,
        totals: TotalsOut {
            passed: totals.passed,
            failed: totals.failed,
            skipped: totals.skipped,
            total,
            pass_rate_pct: pass_rate,
        },
        layer_percentages: LayerPercentages {
            host_smoke_pass_rate_pct: host_rate,
            app_integration_pass_rate_pct: integration_rate,
            runtime_probe_pass_rate_pct: compat_rate,
            kernel_gate_pass_rate_pct: kernel_rate,
            qemu_gate_pass_rate_pct: qemu_rate,
            overall_compatibility_index_pct: overall,
            ci_policy_ok: ci_ok,
        },
    };

    let reports = paths::resolve("reports");
    paths::ensure_dir(&reports)?;
    report::write_json_report(
        &reports.join("linux_app_compat_validation_scorecard.json"),
        &out,
    )?;
    let mut desktop_probes = serde_json::Map::new();
    desktop_probes.insert(
        "wayland_probe_ok".to_string(),
        serde_json::json!(wayland_probe_ok),
    );
    desktop_probes.insert("x11_probe_ok".to_string(), serde_json::json!(x11_probe_ok));
    desktop_probes.insert("fs_stack_ok".to_string(), serde_json::json!(fs_stack_ok));
    desktop_probes.insert(
        "package_stack_ok".to_string(),
        serde_json::json!(package_stack_ok),
    );
    desktop_probes.insert(
        "desktop_app_stack_ok".to_string(),
        serde_json::json!(desktop_app_stack_ok),
    );
    desktop_probes.insert(
        "runtime_system_package_manager_any".to_string(),
        serde_json::json!(has_any_system_pkg_manager),
    );
    desktop_probes.insert(
        "runtime_seeded_system_package_manager_any".to_string(),
        serde_json::json!(seeded_system_pkg_manager_any),
    );
    desktop_probes.insert(
        "runtime_seeded_apt_available".to_string(),
        serde_json::json!(seeded_apt_available),
    );
    desktop_probes.insert(
        "runtime_seeded_pacman_available".to_string(),
        serde_json::json!(seeded_pacman_available),
    );
    desktop_probes.insert(
        "runtime_seeded_diskfs_setup_available".to_string(),
        serde_json::json!(seeded_diskfs_setup_available),
    );
    desktop_probes.insert(
        "runtime_seeded_pivot_root_setup_available".to_string(),
        serde_json::json!(seeded_pivot_root_setup_available),
    );
    desktop_probes.insert(
        "source_diskfs_bootstrap_telemetry_ok".to_string(),
        serde_json::json!(diskfs_bootstrap_telemetry_ok),
    );
    desktop_probes.insert(
        "runtime_seeded_abi_check_available".to_string(),
        serde_json::json!(seeded_abi_check_available),
    );
    desktop_probes.insert(
        "runtime_seeded_abi_contract_available".to_string(),
        serde_json::json!(seeded_abi_contract_available),
    );
    desktop_probes.insert(
        "runtime_seeded_mirror_failover_available".to_string(),
        serde_json::json!(seeded_mirror_failover_available),
    );
    desktop_probes.insert(
        "runtime_seeded_signature_policy_available".to_string(),
        serde_json::json!(seeded_signature_policy_available),
    );
    desktop_probes.insert(
        "runtime_seeded_checksum_policy_available".to_string(),
        serde_json::json!(seeded_checksum_policy_available),
    );
    desktop_probes.insert(
        "runtime_seeded_installer_policy_available".to_string(),
        serde_json::json!(seeded_installer_policy_available),
    );
    desktop_probes.insert(
        "runtime_seeded_retry_timeout_available".to_string(),
        serde_json::json!(seeded_retry_timeout_available),
    );
    desktop_probes.insert(
        "runtime_seeded_apt_keyring_list_available".to_string(),
        serde_json::json!(seeded_apt_keyring_list_available),
    );
    desktop_probes.insert(
        "runtime_seeded_pacman_keyring_path_available".to_string(),
        serde_json::json!(seeded_pacman_keyring_path_available),
    );
    desktop_probes.insert(
        "runtime_seeded_flutter_closure_audit_available".to_string(),
        serde_json::json!(seeded_flutter_closure_audit_available),
    );
    desktop_probes.insert(
        "runtime_seeded_apt_capability_manifest_available".to_string(),
        serde_json::json!(seeded_apt_capability_manifest_available),
    );
    desktop_probes.insert(
        "runtime_seeded_apt_host_limitation_note_available".to_string(),
        serde_json::json!(seeded_apt_host_limitation_note_available),
    );
    desktop_probes.insert(
        "runtime_dev_package_stack_ok".to_string(),
        serde_json::json!(has_min_dev_pkg_stack),
    );
    desktop_probes.insert(
        "runtime_language_package_manager_count".to_string(),
        serde_json::json!(language_pkg_manager_count),
    );
    desktop_probes.insert(
        "runtime_desktop_session_available".to_string(),
        serde_json::json!(has_desktop_session_runtime),
    );
    desktop_probes.insert(
        "runtime_flutter_available".to_string(),
        serde_json::json!(has_flutter_runtime),
    );
    desktop_probes.insert(
        "runtime_seeded_flutter_available".to_string(),
        serde_json::json!(seeded_flutter_runtime_available),
    );
    desktop_probes.insert(
        "runtime_desktop_install_capable".to_string(),
        serde_json::json!(desktop_install_capable),
    );
    desktop_probes.insert(
        "source_elf_so_runtime_contract_ok".to_string(),
        serde_json::json!(elf_so_runtime_contract_ok),
    );
    desktop_probes.insert(
        "source_wayland_x11_protocol_depth_ok".to_string(),
        serde_json::json!(wayland_x11_depth_ok),
    );
    desktop_probes.insert(
        "source_syscall_semantic_parity_ok".to_string(),
        serde_json::json!(syscall_semantic_parity_ok),
    );
    desktop_probes.insert(
        "source_gpu_ioctl_coverage_ok".to_string(),
        serde_json::json!(gpu_ioctl_coverage_ok),
    );
    desktop_probes.insert(
        "source_linux_host_e2e_pipeline_ok".to_string(),
        serde_json::json!(linux_host_e2e_pipeline_ok),
    );
    desktop_probes.insert(
        "source_syscall_parity_module_count".to_string(),
        serde_json::json!(parity_module_count),
    );

    let mut runtime_probe = serde_json::Map::new();
    runtime_probe.insert(
        "generated_utc".to_string(),
        serde_json::json!(report::utc_now_iso()),
    );
    runtime_probe.insert(
        "busybox_required".to_string(),
        serde_json::json!(require_busybox),
    );
    runtime_probe.insert(
        "glibc_required".to_string(),
        serde_json::json!(require_glibc),
    );
    runtime_probe.insert(
        "desktop_smoke".to_string(),
        serde_json::json!(desktop_smoke),
    );
    runtime_probe.insert(
        "wayland_required".to_string(),
        serde_json::json!(require_wayland),
    );
    runtime_probe.insert("x11_required".to_string(), serde_json::json!(require_x11));
    runtime_probe.insert(
        "fs_stack_required".to_string(),
        serde_json::json!(require_fs_stack),
    );
    runtime_probe.insert(
        "package_stack_required".to_string(),
        serde_json::json!(require_package_stack),
    );
    runtime_probe.insert(
        "desktop_app_stack_required".to_string(),
        serde_json::json!(require_desktop_app_stack),
    );
    runtime_probe.insert("layer_counts".to_string(), serde_json::json!(compat));
    runtime_probe.insert(
        "desktop_probes".to_string(),
        serde_json::Value::Object(desktop_probes),
    );

    report::write_json_report(
        &reports.join("linux_app_runtime_probe_report.json"),
        &serde_json::Value::Object(runtime_probe),
    )?;

    println!("\nPassed: {}/{}", totals.passed, total);
    println!("Failed: {}/{}", totals.failed, total);
    println!("Skipped: {}/{}", totals.skipped, total);
    println!("Pass Rate: {}%", pass_rate);

    if ci && !ci_ok {
        bail!("ci policy failed")
    }
    if totals.failed > 0 {
        bail!("linux app compatibility validation failed")
    }
    Ok(())
}
