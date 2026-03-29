use anyhow::Result;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::utils::{paths, report};

#[derive(Debug, Serialize)]
struct DesktopStackAuditReport {
    schema_version: u32,
    generated_utc: String,
    host_runtime: HostRuntime,
    target_readiness: TargetReadiness,
    checks: Vec<AuditCheck>,
    critical_gaps: Vec<AuditGap>,
    next_steps: Vec<String>,
}

#[derive(Debug, Serialize)]
struct HostRuntime {
    package_managers: BTreeMap<String, bool>,
    has_any_system_package_manager: bool,
    has_apt: bool,
    has_apt_get: bool,
}

#[derive(Debug, Serialize)]
struct TargetReadiness {
    readiness_score_pct: f64,
    apt_bootstrap_ready: bool,
    xfce_install_path_ready: bool,
    gnome_install_path_ready: bool,
    flutter_desktop_path_ready: bool,
}

#[derive(Debug, Serialize)]
struct AuditCheck {
    id: String,
    title: String,
    area: String,
    ok: bool,
    severity: String,
    evidence: String,
    recommendation: String,
}

#[derive(Debug, Serialize)]
struct AuditGap {
    id: String,
    severity: String,
    impact: String,
    blocking_for: Vec<String>,
    recommendation: String,
}

pub fn run() -> Result<()> {
    println!("[linux-abi::desktop-stack-audit] Running APT + desktop stack readiness audit");

    let root = paths::repo_root();
    let out_dir = paths::resolve("reports/linux_desktop_stack_audit");
    paths::ensure_dir(&out_dir)?;

    let cargo_toml = root.join("Cargo.toml");
    let linux_dispatch = root.join("src/kernel/syscalls/linux_shim/dispatch.rs");
    let linux_path_ops = root.join("src/kernel/syscalls/linux_shim/fs/io/path_ops.rs");
    let linux_path_ops_tests = root.join("src/kernel/syscalls/linux_shim/fs/io/path_ops/tests.rs");
    let linux_fs_mod = root.join("src/kernel/syscalls/linux_shim/fs/mod.rs");
    let linux_fs_integration_tests = root.join("src/kernel/syscalls/linux_shim/fs/integration_tests.rs");
    let disk_fs = root.join("src/modules/vfs/disk_fs.rs");
    let writable_fs = root.join("src/modules/vfs/writable_fs.rs");
    let library_backends = root.join("src/modules/vfs/library_backends.rs");
    let wayland_mod = root.join("src/modules/userspace_graphics/wayland/mod.rs");
    let x11_mod = root.join("src/modules/userspace_graphics/x11/mod.rs");
    let procfs_gen = root.join("src/modules/vfs/procfs/generators.rs");
    let dbus_mod = root.join("src/modules/ipc/dbus.rs");

    let package_managers = collect_package_manager_status();
    let has_apt = *package_managers.get("apt").unwrap_or(&false);
    let has_apt_get = *package_managers.get("apt-get").unwrap_or(&false);
    let has_any_system_package_manager = package_managers.values().any(|v| *v);

    let mut checks = Vec::new();

    let linux_feature_surface_ok = file_contains_all(
        &cargo_toml,
        &[
            "linux_compat = [",
            "posix_process",
            "posix_fs",
            "posix_mman",
            "posix_net",
            "linux_compat_vfs",
            "network_https",
            "ipc_dbus",
        ],
    );
    checks.push(AuditCheck {
        id: "feature_surface".to_string(),
        title: "Linux compatibility feature surface".to_string(),
        area: "kernel-build".to_string(),
        ok: linux_feature_surface_ok,
        severity: "high".to_string(),
        evidence: "Cargo features include linux_compat, posix_*, linux_compat_vfs, network_https, ipc_dbus".to_string(),
        recommendation: "Keep linux_compat profile default in desktop ISO build and enforce in CI gates".to_string(),
    });

    let apt_host_ready = has_apt || has_apt_get || has_any_system_package_manager;
    checks.push(AuditCheck {
        id: "host_apt_presence".to_string(),
        title: "Host shell has package manager for desktop bootstrap".to_string(),
        area: "host-runtime".to_string(),
        ok: apt_host_ready,
        severity: "medium".to_string(),
        evidence: format!(
            "apt={}, apt-get={}, any_system_pm={}",
            has_apt, has_apt_get, has_any_system_package_manager
        ),
        recommendation: "Prefer Debian/Ubuntu host for apt-path smoke tests; non-apt package managers remain acceptable for generic bootstrap validation".to_string(),
    });

    let dispatch_has_core_pkg_ops = file_contains_all(
        &linux_dispatch,
        &[
            "linux_nr::OPENAT",
            "linux_nr::OPENAT2",
            "linux_nr::UNLINKAT",
            "linux_nr::RENAMEAT2",
            "linux_nr::FCHMODAT",
            "linux_nr::FCHOWNAT",
            "linux_nr::STATX",
            "linux_nr::FTRUNCATE",
            "linux_nr::FSYNC",
        ],
    );
    checks.push(AuditCheck {
        id: "pkg_core_fs_syscalls".to_string(),
        title: "Core package file syscalls present".to_string(),
        area: "kernel-syscalls".to_string(),
        ok: dispatch_has_core_pkg_ops,
        severity: "high".to_string(),
        evidence: "dispatch includes openat/openat2/unlinkat/renameat2/fchmodat/fchownat/statx/ftruncate/fsync".to_string(),
        recommendation: "Add syscall-level integration tests using dpkg-style rename/fsync/chown/chmod patterns".to_string(),
    });

    let renameat2_noreplace_ready = file_contains(&linux_dispatch, "linux_nr::RENAMEAT2")
        && file_contains(&linux_path_ops, "RENAME_NOREPLACE")
        && file_contains(&linux_path_ops, "EEXIST");
    checks.push(AuditCheck {
        id: "pkg_renameat2_noreplace_semantics".to_string(),
        title: "renameat2 noreplace semantic coverage".to_string(),
        area: "kernel-syscalls".to_string(),
        ok: renameat2_noreplace_ready,
        severity: "high".to_string(),
        evidence: "dispatch routes RENAMEAT2 and path_ops handles RENAME_NOREPLACE with EEXIST behavior".to_string(),
        recommendation: "Keep extending renameat2 compatibility for exchange/whiteout where filesystem semantics are available".to_string(),
    });

    let renameat2_exchange_ready = file_contains(&linux_dispatch, "linux_nr::RENAMEAT2")
        && file_contains(&linux_path_ops, "RENAME_EXCHANGE")
        && (file_contains(&linux_path_ops, "renameat2_exchange_swaps_paths")
            || file_contains(&linux_path_ops_tests, "renameat2_exchange_swaps_paths"));
    checks.push(AuditCheck {
        id: "pkg_renameat2_exchange_semantics".to_string(),
        title: "renameat2 exchange semantic coverage".to_string(),
        area: "kernel-syscalls".to_string(),
        ok: renameat2_exchange_ready,
        severity: "high".to_string(),
        evidence: "dispatch routes RENAMEAT2 and path_ops includes RENAME_EXCHANGE behavior with swap-path test coverage".to_string(),
        recommendation: "Keep strengthening rename-exchange behavior with atomicity and rollback assertions across filesystem backends".to_string(),
    });

    let dispatch_has_link_symlink = file_contains(&linux_dispatch, "linux_nr::LINKAT")
        && file_contains(&linux_dispatch, "linux_nr::SYMLINKAT");
    checks.push(AuditCheck {
        id: "pkg_link_symlink_syscalls".to_string(),
        title: "linkat and symlinkat in Linux shim dispatch".to_string(),
        area: "kernel-syscalls".to_string(),
        ok: dispatch_has_link_symlink,
        severity: "high".to_string(),
        evidence: "Package extraction and maintainer scripts often rely on hard/symbolic link syscalls".to_string(),
        recommendation: "Implement and wire LINKAT/SYMLINKAT handlers in linux_shim/fs path ops".to_string(),
    });

    let dpkg_flow_tests_ready = file_contains(&linux_fs_mod, "dpkg_style_file_ops_sequence_succeeds")
        || file_contains(
            &linux_fs_integration_tests,
            "dpkg_style_file_ops_sequence_succeeds",
        );
    checks.push(AuditCheck {
        id: "pkg_dpkg_sequence_tests".to_string(),
        title: "dpkg-style syscall sequence test coverage".to_string(),
        area: "kernel-tests".to_string(),
        ok: dpkg_flow_tests_ready,
        severity: "high".to_string(),
        evidence: "linux_shim fs integration test covers link/symlink/rename/chmod/chown/fsync sequence".to_string(),
        recommendation: "Keep extending package lifecycle tests with failure/recovery branches and maintainer-script edge cases".to_string(),
    });

    let persistent_fs_ready = file_contains(&writable_fs, "WritableOverlayFs")
        && file_contains(&writable_fs, "BlockWritebackSink")
        && file_contains(&library_backends, "overlay-writeback adapter")
        && !file_contains(&disk_fs, "read-only pending")
        && !file_contains(&disk_fs, "pending read-only image mapping");
    checks.push(AuditCheck {
        id: "persistent_pkg_db_fs".to_string(),
        title: "Persistent writable filesystem for package DB".to_string(),
        area: "storage".to_string(),
        ok: persistent_fs_ready,
        severity: "critical".to_string(),
        evidence: "WritableOverlayFs + BlockWritebackSink and overlay-writeback backend descriptors are present".to_string(),
        recommendation: "Keep exercising reboot durability via writeback+journal soak tests for package DB paths".to_string(),
    });

    let seccomp_runtime_visible = file_contains(&procfs_gen, "NoNewPrivs:\\t{}")
        && file_contains(&procfs_gen, "Seccomp:\\t{}")
        && !file_contains(&procfs_gen, "Seccomp:\\t0");
    checks.push(AuditCheck {
        id: "seccomp_runtime".to_string(),
        title: "Non-stub seccomp runtime visibility".to_string(),
        area: "security".to_string(),
        ok: seccomp_runtime_visible,
        severity: "medium".to_string(),
        evidence: "procfs status includes dynamic NoNewPrivs/Seccomp fields sourced from PRCTL task state".to_string(),
        recommendation: "Expose real seccomp mode/state in procfs and align with Linux ABI expectations".to_string(),
    });

    let graphics_server_semantics_ready = file_contains(&wayland_mod, "wayland_protocol_semantics_supported")
        && file_contains(&wayland_mod, "validate_surface_commit_prefix")
        && file_contains(&x11_mod, "x11_reply_event_semantics_supported")
        && file_contains(&x11_mod, "validate_core_opcode_dispatch_prefix");
    checks.push(AuditCheck {
        id: "graphics_server_semantics".to_string(),
        title: "Wayland/X11 server semantics beyond frame parsing".to_string(),
        area: "graphics".to_string(),
        ok: graphics_server_semantics_ready,
        severity: "critical".to_string(),
        evidence: "Wayland/X11 modules expose semantic-path helpers for registry/surface commit and reply/event routing".to_string(),
        recommendation: "Continue incrementally replacing prefix validators with full object lifecycle and server-state transitions".to_string(),
    });

    let gpu_mod = root.join("src/modules/drivers/gpu/mod.rs");
    let gpu_stack_present = root.join("src/modules/drivers/gpu").exists()
        && file_contains(&gpu_mod, "GpuBackend")
        && file_contains(&gpu_mod, "mark_kms_ready")
        && file_contains(&gpu_mod, "is_desktop_session_ready");
    checks.push(AuditCheck {
        id: "gpu_drm_kms_stack".to_string(),
        title: "DRM/KMS GPU stack presence".to_string(),
        area: "graphics".to_string(),
        ok: gpu_stack_present,
        severity: "critical".to_string(),
        evidence: "GPU module exposes backend + KMS/input readiness primitives for desktop-session gating".to_string(),
        recommendation: "Add basic framebuffer path first, then DRM/KMS + input stack for desktop sessions".to_string(),
    });

    let session_daemon_stack_ready = root.join("src/modules/ipc/dbus.rs").exists()
        && file_contains(&dbus_mod, "register_session_service")
        && file_contains(&dbus_mod, "mark_session_service_ready")
        && file_contains(&dbus_mod, "list_session_services");
    checks.push(AuditCheck {
        id: "session_bus_and_daemons".to_string(),
        title: "Desktop session daemon substrate (dbus/logind/udev)".to_string(),
        area: "desktop-runtime".to_string(),
        ok: session_daemon_stack_ready,
        severity: "high".to_string(),
        evidence: "IPC DBus module exposes session service registration/readiness/heartbeat supervision primitives".to_string(),
        recommendation: "Provide userspace dbus broker + service supervision and defer GNOME until logind-equivalent stabilizes".to_string(),
    });

    let passed = checks.iter().filter(|c| c.ok).count();
    let total = checks.len();
    let readiness_score_pct = if total == 0 {
        100.0
    } else {
        ((passed as f64 / total as f64) * 1000.0).round() / 10.0
    };

    let apt_bootstrap_ready = linux_feature_surface_ok
        && dispatch_has_core_pkg_ops
        && renameat2_noreplace_ready
        && renameat2_exchange_ready
        && dispatch_has_link_symlink
        && dpkg_flow_tests_ready
        && persistent_fs_ready;

    let xfce_install_path_ready = apt_bootstrap_ready
        && graphics_server_semantics_ready
        && gpu_stack_present;

    let gnome_install_path_ready = xfce_install_path_ready && session_daemon_stack_ready;

    let flutter_desktop_path_ready = xfce_install_path_ready && has_any_system_package_manager;

    let critical_gaps = checks
        .iter()
        .filter(|c| !c.ok)
        .map(|c| AuditGap {
            id: c.id.clone(),
            severity: c.severity.clone(),
            impact: format!("{} is not ready", c.title),
            blocking_for: blocking_targets_for(&c.id),
            recommendation: c.recommendation.clone(),
        })
        .collect::<Vec<_>>();

    let next_steps = derive_next_steps(&critical_gaps);

    let report_data = DesktopStackAuditReport {
        schema_version: 1,
        generated_utc: report::utc_now_iso(),
        host_runtime: HostRuntime {
            package_managers,
            has_any_system_package_manager,
            has_apt,
            has_apt_get,
        },
        target_readiness: TargetReadiness {
            readiness_score_pct,
            apt_bootstrap_ready,
            xfce_install_path_ready,
            gnome_install_path_ready,
            flutter_desktop_path_ready,
        },
        checks,
        critical_gaps,
        next_steps,
    };

    report::write_json_report(&out_dir.join("summary.json"), &report_data)?;
    fs::write(out_dir.join("summary.md"), render_markdown(&report_data))?;

    println!(
        "[linux-abi::desktop-stack-audit] readiness={:.1}% apt_ready={} xfce_ready={} gnome_ready={} flutter_ready={}",
        report_data.target_readiness.readiness_score_pct,
        report_data.target_readiness.apt_bootstrap_ready,
        report_data.target_readiness.xfce_install_path_ready,
        report_data.target_readiness.gnome_install_path_ready,
        report_data.target_readiness.flutter_desktop_path_ready,
    );
    println!(
        "[linux-abi::desktop-stack-audit] wrote {}",
        out_dir.join("summary.json").display()
    );

    Ok(())
}

fn shell_cmd(command: &str) -> std::io::Result<std::process::Output> {
    if cfg!(windows) {
        Command::new("cmd").args(["/C", command]).output()
    } else {
        Command::new("sh").args(["-c", command]).output()
    }
}

fn package_manager_probe_command(cmd: &str) -> String {
    if cfg!(windows) {
        format!("where {} >NUL 2>&1", cmd)
    } else {
        format!("command -v {} >/dev/null 2>&1", cmd)
    }
}

fn command_exists(cmd: &str) -> bool {
    shell_cmd(&package_manager_probe_command(cmd))
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn collect_package_manager_status() -> BTreeMap<String, bool> {
    let mut out = BTreeMap::new();
    for pm in ["apt", "apt-get", "dnf", "pacman", "apk", "zypper"] {
        out.insert(pm.to_string(), command_exists(pm));
    }
    out
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

fn blocking_targets_for(check_id: &str) -> Vec<String> {
    match check_id {
        "feature_surface" => vec!["apt".to_string(), "xfce".to_string(), "gnome".to_string(), "flutter".to_string()],
        "host_apt_presence" => vec!["host apt smoke".to_string()],
        "pkg_core_fs_syscalls" => vec!["apt".to_string(), "dpkg".to_string()],
        "pkg_renameat2_noreplace_semantics" => vec!["apt".to_string(), "dpkg".to_string()],
        "pkg_renameat2_exchange_semantics" => vec!["apt".to_string(), "dpkg".to_string()],
        "pkg_link_symlink_syscalls" => vec!["apt".to_string(), "dpkg".to_string()],
        "pkg_dpkg_sequence_tests" => vec!["apt".to_string(), "dpkg".to_string()],
        "persistent_pkg_db_fs" => vec!["apt".to_string(), "xfce".to_string(), "gnome".to_string(), "flutter".to_string()],
        "seccomp_runtime" => vec!["hardening".to_string()],
        "graphics_server_semantics" => vec!["xfce".to_string(), "gnome".to_string(), "flutter".to_string()],
        "gpu_drm_kms_stack" => vec!["xfce".to_string(), "gnome".to_string(), "flutter".to_string()],
        "session_bus_and_daemons" => vec!["gnome".to_string()],
        _ => vec!["desktop".to_string()],
    }
}

fn step_for_gap(gap_id: &str) -> Option<&'static str> {
    match gap_id {
        "host_apt_presence" => Some("Run distro-native package bootstrap smoke (apt/dnf/pacman/apk) and keep host-specific path in CI matrix."),
        "pkg_core_fs_syscalls" => Some("Extend syscall behavior tests for package operations with negative/failure recovery paths."),
        "pkg_renameat2_noreplace_semantics" => Some("Implement and validate RENAME_EXCHANGE/RENAME_WHITEOUT when filesystem semantics are available."),
        "pkg_renameat2_exchange_semantics" => Some("Increase rename-exchange robustness with backend-specific atomicity/rollback conformance tests."),
        "pkg_link_symlink_syscalls" => Some("Expand link/symlink coverage to include relative dirfd and permission edge cases."),
        "pkg_dpkg_sequence_tests" => Some("Add maintainer-script style integration cases with rollback/retry branches."),
        "persistent_pkg_db_fs" => Some("Run writeback+journal soak + reboot durability suite for package DB paths."),
        "seccomp_runtime" => Some("Validate seccomp visibility against real filter mode transitions and add procfs conformance assertions."),
        "graphics_server_semantics" => Some("Promote protocol prefix validators into full stateful lifecycle dispatch handlers."),
        "gpu_drm_kms_stack" => Some("Integrate real DRM/KMS device path and input event plumbing on top of GPU readiness primitives."),
        "session_bus_and_daemons" => Some("Add userspace broker/service supervision loop and logind-equivalent session contracts."),
        _ => None,
    }
}

fn derive_next_steps(gaps: &[AuditGap]) -> Vec<String> {
    let mut steps = Vec::new();
    for gap in gaps {
        if let Some(step) = step_for_gap(&gap.id) {
            if !steps.iter().any(|s| s == step) {
                steps.push(step.to_string());
            }
        }
    }
    if steps.is_empty() {
        steps.push("Keep regression gates green with nightly ABI/desktop audits and soak tests.".to_string());
    }
    steps
}

fn render_markdown(report: &DesktopStackAuditReport) -> String {
    let mut md = String::new();
    md.push_str("# Linux Desktop Stack Audit\n\n");
    md.push_str(&format!(
        "- readiness_score_pct: `{:.1}`\n- apt_bootstrap_ready: `{}`\n- xfce_install_path_ready: `{}`\n- gnome_install_path_ready: `{}`\n- flutter_desktop_path_ready: `{}`\n\n",
        report.target_readiness.readiness_score_pct,
        report.target_readiness.apt_bootstrap_ready,
        report.target_readiness.xfce_install_path_ready,
        report.target_readiness.gnome_install_path_ready,
        report.target_readiness.flutter_desktop_path_ready,
    ));

    md.push_str("## Host Package Managers\n\n");
    for (pm, present) in &report.host_runtime.package_managers {
        md.push_str(&format!("- {}: `{}`\n", pm, present));
    }

    md.push_str("\n## Checks\n\n");
    for check in &report.checks {
        md.push_str(&format!(
            "- [{}] {} ({}) :: {}\n  - evidence: {}\n  - recommendation: {}\n",
            if check.ok { "ok" } else { "fail" },
            check.title,
            check.area,
            check.severity,
            check.evidence,
            check.recommendation,
        ));
    }

    md.push_str("\n## Critical Gaps\n\n");
    for gap in &report.critical_gaps {
        md.push_str(&format!(
            "- [{}] {} :: blocks {:?}\n  - impact: {}\n  - recommendation: {}\n",
            gap.severity, gap.id, gap.blocking_for, gap.impact, gap.recommendation,
        ));
    }

    md
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_manager_probe_command_matches_platform_shell() {
        let probe = package_manager_probe_command("apt");
        if cfg!(windows) {
            assert!(probe.starts_with("where apt"));
            assert!(probe.contains(">NUL 2>&1"));
        } else {
            assert!(probe.starts_with("command -v apt"));
            assert!(probe.contains(">/dev/null 2>&1"));
        }
    }
}
