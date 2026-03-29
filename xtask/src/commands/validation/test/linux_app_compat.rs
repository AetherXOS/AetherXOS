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
    Command::new("sh").args(["-c", command]).output()
}

fn command_exists(cmd: &str) -> bool {
    shell_cmd(&format!("command -v {} >/dev/null 2>&1", cmd))
        .map(|out| out.status.success())
        .unwrap_or(false)
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

fn run_optional(layer: &mut Layer, totals: &mut Totals, name: &str, check_cmd: &str, cmd: &str, required: bool) {
    let present = shell_cmd(check_cmd).map(|o| o.status.success()).unwrap_or(false);
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
        println!("[test::linux-app-compat] Desktop smoke profile enabled (Wayland/X11 probes required)");
    }

    let mut totals = Totals::default();
    let mut host = Layer::default();
    let mut integration = Layer::default();
    let mut compat = Layer::default();
    let mut kernel = Layer::default();
    let mut qemu_layer = Layer::default();

    println!("\nPhase 1: Host Shell and Linux Primitive Smoke");
    let _ = run_case(&mut host, &mut totals, "Process creation", "exit 0");
    let _ = run_case(&mut host, &mut totals, "File read/write", "echo test >/tmp/hc_test.txt; cat /tmp/hc_test.txt | grep test");
    let _ = run_case(&mut host, &mut totals, "Pipe chaining", "echo hello | cat | grep hello");
    let _ = run_case(&mut host, &mut totals, "procfs available", "ls /proc >/dev/null");
    if !quick {
        let _ = run_case(&mut host, &mut totals, "Loop execution", "for i in 1 2 3; do echo $i; done | wc -l | grep '^3$'");
    }

    println!("\nPhase 1b: App Integration");
    let _ = run_case(&mut integration, &mut totals, "awk aggregation", "printf 'a 1\na 2\n' | awk '$1==\"a\" {s+=$2} END{print s}' | grep '^3$'");
    let _ = run_case(&mut integration, &mut totals, "sed transform", "printf 'linux-compat\n' | sed 's/linux/hyper/' | grep '^hyper-compat$'");
    let _ = run_case(&mut integration, &mut totals, "tar round-trip", "mkdir -p /tmp/hc_tar/src; echo payload >/tmp/hc_tar/src/a.txt; tar -cf /tmp/hc_tar/a.tar -C /tmp/hc_tar/src .; mkdir -p /tmp/hc_tar/out; tar -xf /tmp/hc_tar/a.tar -C /tmp/hc_tar/out; cat /tmp/hc_tar/out/a.txt | grep payload");

    println!("\nPhase 1c: Runtime Probe");
    run_optional(&mut compat, &mut totals, "busybox availability", "command -v busybox >/dev/null 2>&1", "busybox --help >/dev/null", require_busybox);
    run_optional(&mut compat, &mut totals, "busybox applet smoke", "command -v busybox >/dev/null 2>&1", "busybox ls / >/dev/null", require_busybox);
    run_optional(&mut compat, &mut totals, "glibc detection", "getconf GNU_LIBC_VERSION >/dev/null 2>&1", "getconf GNU_LIBC_VERSION | grep -i glibc", require_glibc);

    let root = paths::repo_root();
    let wayland_mod = root.join("src/modules/userspace_graphics/wayland/mod.rs");
    let wayland_proto = root.join("src/modules/userspace_graphics/wayland/protocol.rs");
    let x11_mod = root.join("src/modules/userspace_graphics/x11/mod.rs");
    let x11_proto = root.join("src/modules/userspace_graphics/x11/protocol.rs");
    let vfs_mod = root.join("src/modules/vfs/mod.rs");
    let linux_mount_setup = root.join("src/modules/vfs/linux_mount_setup.rs");
    let mount_table = root.join("src/modules/vfs/mount_table.rs");
    let disk_fs = root.join("src/modules/vfs/disk_fs.rs");
    let posix_lifecycle = root.join("src/modules/posix/fs/lifecycle_support.rs");
    let cargo_toml = root.join("Cargo.toml");

    let wayland_probe_ok =
        file_contains(&wayland_mod, "validate_client_handshake_prefix")
        && file_contains(&wayland_mod, "validate_registry_advertisement_path")
        && file_contains(&wayland_mod, "validate_registry_bind_prefix")
        && file_contains(&wayland_proto, "parse_wire_header")
        && file_contains(&wayland_proto, "is_complete_frame");

    let x11_probe_ok =
        file_contains(&x11_mod, "validate_client_setup_request")
        && file_contains(&x11_mod, "validate_server_reply_prefix")
        && file_contains(&x11_mod, "validate_core_opcode_dispatch_prefix")
        && file_contains(&x11_proto, "parse_setup_prefix")
        && file_contains(&x11_proto, "parse_server_packet_prefix");

    let system_pkg_managers = ["apt-get", "dnf", "pacman", "apk", "zypper"];
    let has_any_system_pkg_manager = system_pkg_managers
        .iter()
        .any(|pm| command_exists(pm));

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
    let desktop_install_capable = has_any_system_pkg_manager && has_min_dev_pkg_stack;

    let fs_stack_ok =
        file_contains(&posix_lifecycle, "mount_devfs")
        && file_contains(&posix_lifecycle, "mount_ramfs")
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
            &["FsType", "Ext4", "Fat32", "Overlay", "Tmpfs", "Procfs", "Sysfs"],
        )
        && file_contains_all(&disk_fs, &["DiskFsMode", "Fat", "Little", "Ext4", "with_backend"])
        && file_contains_all(&vfs_mod, &["pub mod tmpfs", "pub mod procfs", "pub mod sysfs"]);

    let package_stack_ok =
        file_contains_all(
            &cargo_toml,
            &[
                "linux_compat = [",
                "posix_process",
                "posix_pipe",
                "posix_mman",
                "posix_fs",
                "posix_net",
            ],
        )
        && has_any_system_pkg_manager
        && has_min_dev_pkg_stack
        && file_contains_all(&disk_fs, &["pub fn write_all", "pub fn read_all"])
        && file_contains_all(&linux_mount_setup, &["/tmp", "/run", "/dev/shm"]);

    let desktop_app_stack_ok =
        wayland_probe_ok
            && x11_probe_ok
            && (has_desktop_session_runtime || desktop_install_capable)
            && (has_flutter_runtime || desktop_install_capable)
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

    println!("\nPhase 2: Kernel Gates");
    print!("[GATE] cargo test --features linux_compat --no-run");
    let build_ok = Command::new("cargo")
        .args(["test", "--features", "linux_compat", "--no-run"])
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
    let _ = crate::commands::validation::syscall_coverage::execute(true, "md", &Some("reports/linux_app_compat_syscall_coverage.md".to_string()));
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
    let pass_rate = if executed == 0 { 100.0 } else { ((totals.passed as f64 / executed as f64) * 1000.0).round() / 10.0 };

    let host_rate = rate(&host);
    let integration_rate = rate(&integration);
    let compat_rate = rate(&compat);
    let kernel_rate = rate(&kernel);
    let qemu_rate = if qemu || strict { rate(&qemu_layer) } else { 100.0 };
    let overall = ((host_rate * 0.25) + (integration_rate * 0.20) + (compat_rate * 0.10) + (kernel_rate * 0.25) + (qemu_rate * 0.20)).round();

    let ci_ok = totals.failed == 0;
    let out = Scorecard {
        generated_utc: report::utc_now_iso(),
        profile: if strict { "strict".to_string() } else { "standard".to_string() },
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
    report::write_json_report(&reports.join("linux_app_compat_validation_scorecard.json"), &out)?;
    report::write_json_report(
        &reports.join("linux_app_runtime_probe_report.json"),
        &serde_json::json!({
            "generated_utc": report::utc_now_iso(),
            "busybox_required": require_busybox,
            "glibc_required": require_glibc,
            "desktop_smoke": desktop_smoke,
            "wayland_required": require_wayland,
            "x11_required": require_x11,
            "fs_stack_required": require_fs_stack,
            "package_stack_required": require_package_stack,
            "desktop_app_stack_required": require_desktop_app_stack,
            "layer_counts": compat,
            "desktop_probes": {
                "wayland_probe_ok": wayland_probe_ok,
                "x11_probe_ok": x11_probe_ok,
                "fs_stack_ok": fs_stack_ok,
                "package_stack_ok": package_stack_ok,
                "desktop_app_stack_ok": desktop_app_stack_ok,
                    "runtime_system_package_manager_any": has_any_system_pkg_manager,
                    "runtime_dev_package_stack_ok": has_min_dev_pkg_stack,
                    "runtime_language_package_manager_count": language_pkg_manager_count,
                    "runtime_desktop_session_available": has_desktop_session_runtime,
                    "runtime_flutter_available": has_flutter_runtime,
                    "runtime_desktop_install_capable": desktop_install_capable,
            }
        }),
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
