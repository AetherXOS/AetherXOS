use anyhow::{Context, Result, bail};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

use crate::cli::LinuxAbiAction;
use crate::commands::validation;
use crate::config;
use crate::utils::{paths, report};

use super::abi::abi_drift_report;
use super::diagnostics::{
    critical_policy_guard, release_diagnostics, seed_release_support_reports, warning_audit,
};
use super::evidence_bundle;
use super::host_tools::host_tool_verify;
use super::models::{
    BundleCheck, CiBundleDoc, EvidenceFileEntry, ReleaseEvidenceBundle, ReproducibleBuildEvidence,
};

pub fn parse_csv_lower(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|item| item.trim().to_ascii_lowercase())
        .filter(|item| !item.is_empty())
        .collect()
}

pub fn relative_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

pub fn reproducible_evidence() -> Result<()> {
    println!("[release::reproducible-evidence] Generating reproducible build evidence");

    let root = paths::repo_root();
    let candidates = [
        ("Cargo.toml", true),
        ("Cargo.lock", true),
        ("xtask/Cargo.toml", true),
        (config::repo_paths::P_TIER_STATUS_JSON, true),
        (
            config::repo_paths::PRODUCTION_ACCEPTANCE_SCORECARD_JSON,
            true,
        ),
        (config::repo_paths::SYSCALL_COVERAGE_SUMMARY, true),
        ("artifacts/boot_image/stage/boot/aethercore.elf", false),
        ("artifacts/boot_image/stage/boot/initramfs.cpio.gz", false),
        ("artifacts/boot_image/qemu_smoke.log", false),
    ];

    let mut files = Vec::with_capacity(candidates.len());
    for (path, required) in candidates {
        files.push(build_file_entry(&root, path, required)?);
    }

    let evidence = ReproducibleBuildEvidence {
        generated_utc: report::utc_now_iso(),
        git_commit: capture_command_output("git", &["rev-parse", "HEAD"]),
        rustc_version: capture_command_output("rustc", &["-V"])
            .unwrap_or_else(|| "rustc/unknown".to_string()),
        cargo_version: capture_command_output("cargo", &["-V"])
            .unwrap_or_else(|| "cargo/unknown".to_string()),
        host_os: std::env::consts::OS.to_string(),
        host_arch: std::env::consts::ARCH.to_string(),
        missing_files: files.iter().filter(|entry| !entry.exists).count(),
        files,
    };

    let out_json = root.join(config::repo_paths::REPRO_BUILD_EVIDENCE_JSON);
    let out_md = root.join(config::repo_paths::REPRO_BUILD_EVIDENCE_MD);
    report::write_json_report(&out_json, &evidence)?;
    report::write_text_report(&out_md, &render_reproducible_md(&evidence))?;

    println!("[release::reproducible-evidence] PASS");
    Ok(())
}

pub fn build_file_entry(root: &Path, rel_path: &str, required: bool) -> Result<EvidenceFileEntry> {
    let abs = root.join(rel_path);
    if !abs.exists() {
        return Ok(EvidenceFileEntry {
            path: rel_path.to_string(),
            required,
            exists: false,
            size_bytes: None,
            sha256: None,
            modified_utc: None,
            gate_ok: None,
            gate_detail: None,
        });
    }

    let metadata = fs::metadata(&abs)
        .with_context(|| format!("failed reading metadata for {}", abs.display()))?;
    let size_bytes = metadata.len();
    let modified_utc = metadata.modified().ok().map(|t| {
        let dt: chrono::DateTime<chrono::Utc> = chrono::DateTime::<chrono::Utc>::from(t);
        dt.to_rfc3339()
    });

    Ok(EvidenceFileEntry {
        path: rel_path.to_string(),
        required,
        exists: true,
        size_bytes: Some(size_bytes),
        sha256: Some(hash_file_sha256(&abs)?),
        modified_utc,
        gate_ok: None,
        gate_detail: None,
    })
}

fn hash_file_sha256(path: &Path) -> Result<String> {
    let bytes = fs::read(path)
        .with_context(|| format!("failed to read file for hashing: {}", path.display()))?;
    let digest = Sha256::digest(&bytes);
    Ok(format!("{:x}", digest))
}

pub fn capture_command_output(program: &str, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new(program)
        .args(args)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub fn evaluate_gate(path: &str, doc: &Value) -> Option<(bool, String)> {
    if path == config::repo_paths::P_TIER_STATUS_JSON
        || path == config::repo_paths::PRODUCTION_ACCEPTANCE_SCORECARD_JSON
    {
        let ok = doc
            .get("overall_ok")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        return Some((ok, "overall_ok".to_string()));
    }

    if path == config::repo_paths::SYSCALL_COVERAGE_SUMMARY {
        let pct = doc
            .get("implemented_pct")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        return Some((pct >= 95.0, format!("implemented_pct={pct:.1}")));
    }

    if path == config::repo_paths::RELEASE_DIAGNOSTICS_JSON {
        let ok = doc
            .get("overall_ok")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        return Some((ok, "overall_ok".to_string()));
    }

    if path == config::repo_paths::HOST_TOOL_VERIFY_JSON
        || path == config::repo_paths::CRITICAL_POLICY_GUARD_JSON
        || path == config::repo_paths::WARNING_AUDIT_JSON
    {
        let ok = doc
            .get("overall_ok")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        return Some((ok, "overall_ok".to_string()));
    }

    None
}

fn render_reproducible_md(evidence: &ReproducibleBuildEvidence) -> String {
    let mut md = String::new();
    md.push_str("# Reproducible Build Evidence\n\n");
    md.push_str(&format!("- generated_utc: {}\n", evidence.generated_utc));
    md.push_str(&format!(
        "- git_commit: {}\n",
        evidence.git_commit.as_deref().unwrap_or("unknown")
    ));
    md.push_str(&format!("- rustc_version: {}\n", evidence.rustc_version));
    md.push_str(&format!("- cargo_version: {}\n", evidence.cargo_version));
    md.push_str(&format!("- host_os: {}\n", evidence.host_os));
    md.push_str(&format!("- host_arch: {}\n", evidence.host_arch));
    md.push_str(&format!("- missing_files: {}\n\n", evidence.missing_files));
    md.push_str("## Evidence Files\n\n");
    for file in &evidence.files {
        md.push_str(&format!(
            "- [{}] {} (required={})\n",
            if file.exists { "x" } else { " " },
            file.path,
            file.required
        ));
        if let Some(hash) = &file.sha256 {
            md.push_str(&format!("  - sha256: {}\n", hash));
        }
        if let Some(size) = file.size_bytes {
            md.push_str(&format!("  - size_bytes: {}\n", size));
        }
    }
    md
}

pub fn render_bundle_md(bundle: &ReleaseEvidenceBundle) -> String {
    let mut md = String::new();
    md.push_str("# Release Evidence Bundle\n\n");
    md.push_str(&format!("- generated_utc: {}\n", bundle.generated_utc));
    md.push_str(&format!("- strict: {}\n", bundle.strict));
    md.push_str(&format!("- overall_ok: {}\n", bundle.overall_ok));
    md.push_str(&format!(
        "- required_missing: {}\n",
        bundle.required_missing
    ));
    md.push_str(&format!(
        "- required_gate_failures: {}\n\n",
        bundle.required_gate_failures
    ));
    md.push_str("## Entries\n\n");
    for entry in &bundle.entries {
        md.push_str(&format!(
            "- [{}] {} (required={})\n",
            if entry.exists { "x" } else { " " },
            entry.path,
            entry.required
        ));
        if let Some(gate_ok) = entry.gate_ok {
            md.push_str(&format!("  - gate_ok: {}\n", gate_ok));
        }
        if let Some(detail) = &entry.gate_detail {
            md.push_str(&format!("  - gate_detail: {}\n", detail));
        }
    }
    md
}

pub fn gate_fixup(strict: bool) -> Result<()> {
    println!("[release::gate-fixup] Regenerating release gates and evidence artifacts");
    host_tool_verify(false)?;
    critical_policy_guard(false)?;
    warning_audit(false, None)?;
    seed_release_support_reports()?;
    crate::commands::release::status::run()?;
    reproducible_evidence()?;
    evidence_bundle::run(false)?;
    release_diagnostics(false)?;

    if strict {
        evidence_bundle::run(true)?;
        release_diagnostics(true)?;
    }

    println!("[release::gate-fixup] PASS");
    Ok(())
}

pub fn ci_bundle(strict: bool) -> Result<()> {
    println!("[release::ci-bundle] Building consolidated CI bundle report");
    let root = paths::repo_root();

    gate_fixup(false)?;
    abi_drift_report(None, false)?;
    validation::linux_abi::execute(&LinuxAbiAction::SemanticMatrix)?;
    validation::linux_abi::execute(&LinuxAbiAction::TrendDashboard {
        limit: 60,
        strict: false,
    })?;
    validation::linux_abi::execute(&LinuxAbiAction::WorkloadCatalog {
        limit: 60,
        strict: false,
    })?;
    validation::glibc::execute(&crate::cli::GlibcAction::CompatibilitySplit { strict: false })?;

    let specs = [
        ("p_tier", config::repo_paths::P_TIER_STATUS_JSON),
        (
            "prod_scorecard",
            config::repo_paths::PRODUCTION_ACCEPTANCE_SCORECARD_JSON,
        ),
        (
            "evidence_bundle",
            config::repo_paths::RELEASE_EVIDENCE_BUNDLE_JSON,
        ),
        ("diagnostics", config::repo_paths::RELEASE_DIAGNOSTICS_JSON),
        (
            "host_tool_verify",
            config::repo_paths::HOST_TOOL_VERIFY_JSON,
        ),
        (
            "critical_policy_guard",
            config::repo_paths::CRITICAL_POLICY_GUARD_JSON,
        ),
        ("warning_audit", config::repo_paths::WARNING_AUDIT_JSON),
        ("abi_drift", config::repo_paths::ABI_DRIFT_REPORT_JSON),
        (
            "linux_abi_semantic_matrix",
            config::repo_paths::LINUX_ABI_SEMANTIC_MATRIX_JSON,
        ),
        (
            "linux_abi_trend_dashboard",
            config::repo_paths::LINUX_ABI_TREND_DASHBOARD_JSON,
        ),
        (
            "linux_abi_workload_catalog",
            config::repo_paths::LINUX_ABI_WORKLOAD_CATALOG_JSON,
        ),
        (
            "linux_abi_workload_trend",
            config::repo_paths::LINUX_ABI_WORKLOAD_TREND_JSON,
        ),
        (
            "glibc_compat_split",
            config::repo_paths::GLIBC_COMPAT_SPLIT_JSON,
        ),
    ];

    let mut checks = Vec::new();
    for (id, rel) in specs {
        let path = root.join(rel);
        if !path.exists() {
            checks.push(BundleCheck {
                id: id.to_string(),
                ok: false,
                detail: format!("missing report {}", rel),
            });
            continue;
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed reading CI bundle input: {}", path.display()))?;
        let doc: Value = serde_json::from_str(&text)
            .with_context(|| format!("failed parsing CI bundle input: {}", path.display()))?;
        let ok = doc
            .get("overall_ok")
            .and_then(|value| value.as_bool())
            .unwrap_or(true);
        checks.push(BundleCheck {
            id: id.to_string(),
            ok,
            detail: if ok {
                "overall_ok=true".to_string()
            } else {
                "overall_ok=false".to_string()
            },
        });
    }

    let overall_ok = checks.iter().all(|check| check.ok);
    let bundle = CiBundleDoc {
        generated_utc: report::utc_now_iso(),
        strict,
        overall_ok,
        checks,
    };

    let out_json = root.join(config::repo_paths::CI_BUNDLE_JSON);
    let out_md = root.join(config::repo_paths::CI_BUNDLE_MD);
    report::write_json_report(&out_json, &bundle)?;
    report::write_text_report(&out_md, &render_ci_bundle_md(&bundle))?;

    if strict && !bundle.overall_ok {
        bail!(
            "strict CI bundle failed: one or more checks are not green. See {}",
            out_json.display()
        );
    }

    println!("[release::ci-bundle] PASS");
    Ok(())
}

fn render_ci_bundle_md(bundle: &CiBundleDoc) -> String {
    let mut md = String::new();
    md.push_str("# CI Bundle\n\n");
    md.push_str(&format!("- generated_utc: {}\n", bundle.generated_utc));
    md.push_str(&format!("- strict: {}\n", bundle.strict));
    md.push_str(&format!("- overall_ok: {}\n\n", bundle.overall_ok));
    md.push_str("## Checks\n\n");
    for check in &bundle.checks {
        md.push_str(&format!(
            "- [{}] {} ({})\n",
            if check.ok { "x" } else { " " },
            check.id,
            check.detail
        ));
    }
    md
}
