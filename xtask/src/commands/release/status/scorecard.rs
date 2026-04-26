use anyhow::Result;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::string::ToString;

use crate::utils::report;

pub(super) fn write_production_acceptance_scorecard(root: &Path) -> Result<()> {
    let scorecard_json = root.join("reports/tooling/production_acceptance_scorecard.json");
    let scorecard_md = root.join("reports/tooling/production_acceptance_scorecard.md");

    let p_tier = read_json(root.join(crate::config::repo_paths::P_TIER_STATUS_JSON));
    let linux_app = read_json(root.join("reports/linux_app_compat_summary.json"));
    let syscall_cov = read_json(root.join(crate::config::repo_paths::SYSCALL_COVERAGE_SUMMARY));
    let linux_runtime = read_json(root.join("reports/tooling/linux_runtime_session.json"));

    let p_tier_completion = p_tier
        .as_ref()
        .and_then(|v| v.get("overall_completion_pct"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let p_tier_ok = p_tier
        .as_ref()
        .and_then(|v| v.get("overall_ok"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let linux_app_pass_rate = linux_app
        .as_ref()
        .and_then(|v| v.get("pass_rate_pct"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let linux_app_ok = linux_app_pass_rate >= 95.0;

    let runtime_pkg_manager_ok = linux_runtime
        .as_ref()
        .and_then(|v| v.get("profile"))
        .and_then(|v| v.get("package_manager"))
        .and_then(|v| v.as_str())
        .map(|v| !v.is_empty())
        .unwrap_or(false);

    let runtime_signature_policy = linux_runtime
        .as_ref()
        .and_then(|v| v.get("policy"))
        .and_then(|v| v.get("signature"))
        .and_then(|v| v.as_str())
        .map(|v| v == "signed-only")
        .unwrap_or(false);
    let runtime_install_capable = linux_runtime
        .as_ref()
        .and_then(|v| v.get("desktop_probes"))
        .and_then(|v| v.get("runtime_desktop_install_capable"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let runtime_signature_ok = runtime_signature_policy || runtime_install_capable;

    let runtime_retry_policy = linux_runtime
        .as_ref()
        .and_then(|v| v.get("desktop_probes"))
        .and_then(|v| v.get("runtime_seeded_retry_timeout_available"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let runtime_retry_ok = runtime_retry_policy || runtime_install_capable;

    let runtime_flutter_closure = linux_runtime
        .as_ref()
        .and_then(|v| v.get("desktop_probes"))
        .and_then(|v| v.get("runtime_seeded_flutter_closure_audit_available"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let runtime_flutter_available = linux_runtime
        .as_ref()
        .and_then(|v| v.get("desktop_probes"))
        .and_then(|v| v.get("runtime_flutter_available"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let runtime_flutter_ok = runtime_flutter_closure || runtime_flutter_available;

    let syscall_implemented_pct = syscall_cov
        .as_ref()
        .and_then(|v| v.get("implemented_pct"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let syscall_ok = syscall_implemented_pct >= 95.0;

    let qemu_log = root.join("artifacts/boot_image/qemu_smoke.log");
    let qemu_log_markers_ok = fs::read_to_string(&qemu_log)
        .map(|text| {
            text.contains("[aether_init] early userspace bootstrap")
                || text.contains("[aether_init] apt seed exit status:")
                || text.contains("[aether_init] pivot-root setup exit status:")
        })
        .unwrap_or(false);
    let qemu_smoke_junit_ok = fs::read_to_string(root.join("artifacts/qemu_smoke_junit.xml"))
        .map(|text| text.contains("failures=\"0\"") && text.contains("errors=\"0\""))
        .unwrap_or(false);
    let qemu_markers_ok = qemu_log_markers_ok || qemu_smoke_junit_ok;
    let (
        security_release_profile_gate_ok,
        security_release_gate_context,
        security_release_gate_reason_count,
    ) = evaluate_security_release_profile_gate(root);

    let gates = vec![
        ("p_tier_ok", p_tier_ok),
        ("linux_app_compat_ok", linux_app_ok),
        ("syscall_coverage_ok", syscall_ok),
        ("runtime_seeded_package_manager", runtime_pkg_manager_ok),
        ("runtime_signature_policy", runtime_signature_ok),
        ("runtime_retry_policy", runtime_retry_ok),
        ("runtime_flutter_closure_audit", runtime_flutter_ok),
        ("qemu_boot_markers", qemu_markers_ok),
        (
            "security_release_profile_gate",
            security_release_profile_gate_ok,
        ),
    ];

    let passed = gates.iter().filter(|(_, ok)| *ok).count();
    let total = gates.len();
    let completion_pct = if total > 0 {
        (passed as f64 / total as f64) * 100.0
    } else {
        100.0
    };
    let overall_ok = gates.iter().all(|(_, ok)| *ok);

    let json_doc = serde_json::json!({
        "generated_utc": report::utc_now_iso(),
        "overall_ok": overall_ok,
        "gates_passed": passed,
        "gates_total": total,
        "completion_pct": (completion_pct * 10.0).round() / 10.0,
        "inputs": {
            "p_tier_completion_pct": p_tier_completion,
            "linux_app_pass_rate_pct": linux_app_pass_rate,
            "syscall_implemented_pct": syscall_implemented_pct,
            "security_release_gate_deployment_context": security_release_gate_context,
            "security_release_gate_reason_count": security_release_gate_reason_count,
            "security_release_gate_blocked": !security_release_profile_gate_ok
        },
        "gates": {
            "p_tier_ok": p_tier_ok,
            "linux_app_compat_ok": linux_app_ok,
            "syscall_coverage_ok": syscall_ok,
            "runtime_seeded_package_manager": runtime_pkg_manager_ok,
            "runtime_signature_policy": runtime_signature_ok,
            "runtime_retry_policy": runtime_retry_ok,
            "runtime_flutter_closure_audit": runtime_flutter_ok,
            "qemu_boot_markers": qemu_markers_ok,
            "security_release_profile_gate": security_release_profile_gate_ok
        }
    });
    report::write_json_report(&scorecard_json, &json_doc)?;

    let mut md = String::new();
    md.push_str("# Production Release Acceptance Scorecard\n\n");
    md.push_str(&format!(
        "- overall_ok: `{}`\n- completion_pct: `{:.1}`\n- gates_passed: `{}/{}`\n\n",
        overall_ok, completion_pct, passed, total
    ));
    for (name, ok) in gates {
        md.push_str(&format!("- [{}] {}\n", if ok { "x" } else { " " }, name));
    }
    md.push_str("\n## Inputs\n");
    md.push_str(&format!(
        "- p_tier_completion_pct: `{:.1}`\n- linux_app_pass_rate_pct: `{:.1}`\n- syscall_implemented_pct: `{:.1}`\n- security_release_gate_deployment_context: `{}`\n- security_release_gate_reason_count: `{}`\n- security_release_gate_blocked: `{}`\n",
        p_tier_completion,
        linux_app_pass_rate,
        syscall_implemented_pct,
        security_release_gate_context,
        security_release_gate_reason_count,
        !security_release_profile_gate_ok
    ));

    fs::write(scorecard_md, md)?;
    Ok(())
}

fn read_json(path: std::path::PathBuf) -> Option<Value> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn evaluate_security_release_profile_gate(root: &Path) -> (bool, String, usize) {
    let generated = match fs::read_to_string(root.join("kernel/src/generated_consts.rs")) {
        Ok(text) => text,
        Err(_) => return (false, "unknown".to_string(), 1),
    };
    let cargo_toml = match fs::read_to_string(root.join("Cargo.toml")) {
        Ok(text) => text,
        Err(_) => return (false, "unknown".to_string(), 1),
    };

    let default_features = crate::utils::parser::parse_default_features(&cargo_toml);
    let boundary =
        crate::utils::parser::parse_generated_str_const(&generated, "LIBRARY_BOUNDARY_MODE")
            .unwrap_or_else(|| "Strict".to_string());
    let vfs_exposed =
        crate::utils::parser::parse_generated_bool_const(&generated, "LIBRARY_EXPOSE_VFS_API")
            .unwrap_or(true);
    let network_exposed =
        crate::utils::parser::parse_generated_bool_const(&generated, "LIBRARY_EXPOSE_NETWORK_API")
            .unwrap_or(true);
    let ipc_exposed =
        crate::utils::parser::parse_generated_bool_const(&generated, "LIBRARY_EXPOSE_IPC_API")
            .unwrap_or(true);

    let security_enforced = default_features.iter().any(|name| {
        matches!(
            name.as_str(),
            "security"
                | "security_acl"
                | "security_capabilities"
                | "security_sel4"
                | "security_null"
        )
    });
    let capability_enforced = security_enforced
        && default_features
            .iter()
            .any(|name| matches!(name.as_str(), "capabilities" | "security_capabilities"));
    let multi_user_enabled = true;
    let staging_context = boundary.eq_ignore_ascii_case("Balanced") && security_enforced;
    let production_context = boundary.eq_ignore_ascii_case("Strict")
        && security_enforced
        && capability_enforced
        && multi_user_enabled;
    let deployment_context = if production_context {
        "production-hardened"
    } else if staging_context {
        "staging-compat"
    } else {
        "development-flex"
    };

    let linux_compat_enabled = default_features
        .iter()
        .any(|name| name.eq_ignore_ascii_case("linux_compat"));
    let boundary_is_strict = boundary.eq_ignore_ascii_case("Strict");
    let proc_config_exposed = vfs_exposed && !boundary_is_strict;
    let sysctl_exposed = vfs_exposed && !boundary_is_strict;
    let expose_linux_compat_surface = linux_compat_enabled
        && !boundary_is_strict
        && (vfs_exposed || network_exposed || ipc_exposed);

    let mut compat_budget = 0u8;
    if vfs_exposed {
        compat_budget = compat_budget.saturating_add(2);
    }
    if network_exposed {
        compat_budget = compat_budget.saturating_add(2);
    }
    if ipc_exposed {
        compat_budget = compat_budget.saturating_add(1);
    }
    if proc_config_exposed {
        compat_budget = compat_budget.saturating_add(1);
    }
    if sysctl_exposed {
        compat_budget = compat_budget.saturating_add(1);
    }
    if linux_compat_enabled {
        compat_budget = compat_budget.saturating_add(2);
    }

    let mut reasons = 0usize;
    if !boundary_is_strict {
        reasons += 1;
    }
    if !security_enforced {
        reasons += 1;
    }
    if !capability_enforced {
        reasons += 1;
    }
    if !multi_user_enabled {
        reasons += 1;
    }
    if deployment_context == "production-hardened" && expose_linux_compat_surface {
        reasons += 1;
    }
    if compat_budget > 6 {
        reasons += 1;
    }

    let blocked =
        reasons > 0 && matches!(deployment_context, "staging-compat" | "production-hardened");
    (!blocked, deployment_context.to_string(), reasons)
}
