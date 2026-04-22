use anyhow::{Context, Result, bail};
use regex::Regex;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;

use crate::config;
use crate::utils::{paths, report};

use super::ci::{build_file_entry, capture_command_output, ci_bundle};
use super::models::{
    FreezeCheckDoc, PerfEngineeringReportDoc, PerfThresholdConfig, PerfWaiverConfig,
    ReleaseManifestDoc, SbomAuditDoc, ScoreNormalizeDoc, TrendDashboardDoc,
};
use crate::commands::release::reporting::metrics::helpers as metrics_helpers;

pub fn gate_report(prev: Option<&str>, strict: bool) -> Result<()> {
    crate::commands::release::reporting::gates::gate_report(prev, strict)
}

pub fn export_junit(out: Option<&str>, strict: bool) -> Result<()> {
    println!("[release::export-junit] Exporting release gate summary to JUnit XML");
    let root = paths::repo_root();

    ci_bundle(false)?;
    let ci_bundle_path = root.join(config::repo_paths::CI_BUNDLE_JSON);
    let ci_bundle_text = fs::read_to_string(&ci_bundle_path)
        .with_context(|| format!("failed reading CI bundle for junit export: {}", ci_bundle_path.display()))?;
    let ci_bundle_doc: Value = serde_json::from_str(&ci_bundle_text)
        .with_context(|| format!("failed parsing CI bundle for junit export: {}", ci_bundle_path.display()))?;

    let overall_ok = ci_bundle_doc
        .get("overall_ok")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let mut failed_checks = Vec::new();
    if let Some(checks) = ci_bundle_doc.get("checks").and_then(|v| v.as_array()) {
        for check in checks {
            let ok = check.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
            if !ok {
                let id = check.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
                failed_checks.push(id.to_string());
            }
        }
    }

    let junit_rel = out.unwrap_or(config::repo_paths::RELEASE_GATES_JUNIT_XML);
    let junit_path = root.join(junit_rel);
    let stdout = if failed_checks.is_empty() {
        "all release gate checks are green".to_string()
    } else {
        format!("failed checks: {}", failed_checks.join(", "))
    };
    let failure_message = if overall_ok {
        None
    } else {
        Some("release gate bundle has failing checks")
    };

    report::write_junit_single_case(
        &junit_path,
        &report::JunitSingleCaseReport {
            suite_name: "release_gates",
            case_name: "ci_bundle",
            class_name: "xtask.release",
            duration_secs: 0.0,
            passed: overall_ok,
            failure_message,
            stdout: &stdout,
            stderr: "",
        },
    )?;

    if strict && !overall_ok {
        bail!(
            "strict export-junit failed because ci bundle is not green. See {}",
            ci_bundle_path.display()
        );
    }

    println!("[release::export-junit] PASS");
    Ok(())
}

pub fn explain_failure(strict: bool) -> Result<()> {
    crate::commands::release::reporting::gates::explain_failure(strict)
}

pub fn trend_dashboard(limit: usize, strict: bool) -> Result<()> {
    crate::commands::release::reporting::metrics::trend_dashboard(limit, strict)
}

pub fn render_trend_dashboard_md(doc: &TrendDashboardDoc) -> String {
    metrics_helpers::render_trend_dashboard_md(doc)
}

pub fn freeze_check(strict: bool, allow_dirty: bool) -> Result<()> {
    println!("[release::freeze-check] Running branch/worktree freeze checks");
    let root = paths::repo_root();

    let branch = capture_command_output("git", &["rev-parse", "--abbrev-ref", "HEAD"])
        .unwrap_or_else(|| "unknown".to_string());
    let status = capture_command_output("git", &["status", "--porcelain"]).unwrap_or_default();
    let worktree_clean = status.trim().is_empty();
    let branch_ok = branch == "main" || branch.starts_with("release/") || branch.starts_with("hotfix/");

    let overall_ok = branch_ok && (worktree_clean || allow_dirty);
    let detail = format!(
        "branch_ok={} worktree_clean={} allow_dirty={}",
        branch_ok, worktree_clean, allow_dirty
    );

    let doc = FreezeCheckDoc {
        generated_utc: report::utc_now_iso(),
        strict,
        overall_ok,
        branch,
        worktree_clean,
        detail,
    };

    let out_json = root.join(config::repo_paths::FREEZE_CHECK_JSON);
    let out_md = root.join(config::repo_paths::FREEZE_CHECK_MD);
    report::write_json_report(&out_json, &doc)?;
    report::write_text_report(&out_md, &render_freeze_check_md(&doc))?;

    if strict && !doc.overall_ok {
        bail!(
            "strict freeze-check failed. See {}",
            out_json.display()
        );
    }

    println!("[release::freeze-check] PASS");
    Ok(())
}

fn render_freeze_check_md(doc: &FreezeCheckDoc) -> String {
    let mut md = String::new();
    md.push_str("# Freeze Check\n\n");
    md.push_str(&format!("- generated_utc: {}\n", doc.generated_utc));
    md.push_str(&format!("- strict: {}\n", doc.strict));
    md.push_str(&format!("- overall_ok: {}\n", doc.overall_ok));
    md.push_str(&format!("- branch: {}\n", doc.branch));
    md.push_str(&format!("- worktree_clean: {}\n", doc.worktree_clean));
    md.push_str(&format!("- detail: {}\n", doc.detail));
    md
}

pub fn sbom_audit(strict: bool) -> Result<()> {
    println!("[release::sbom-audit] Auditing Cargo.lock package inventory");
    let root = paths::repo_root();
    let lock_path = root.join("Cargo.lock");
    let text = fs::read_to_string(&lock_path)
        .with_context(|| format!("failed reading Cargo.lock: {}", lock_path.display()))?;

    let name_re = Regex::new(r#"name\s*=\s*\"([^\"]+)\""#)?;
    let mut names = Vec::new();
    for cap in name_re.captures_iter(&text) {
        if let Some(name) = cap.get(1) {
            names.push(name.as_str().to_string());
        }
    }

    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for name in &names {
        *counts.entry(name.clone()).or_insert(0) += 1;
    }
    let duplicate_name_count = counts.values().filter(|count| **count > 1).count();
    let mut top_package_names = counts.keys().take(20).cloned().collect::<Vec<_>>();
    top_package_names.sort();

    let doc = SbomAuditDoc {
        generated_utc: report::utc_now_iso(),
        strict,
        overall_ok: !names.is_empty(),
        package_count: names.len(),
        duplicate_name_count,
        top_package_names,
    };

    let out_json = root.join(config::repo_paths::SBOM_AUDIT_JSON);
    let out_md = root.join(config::repo_paths::SBOM_AUDIT_MD);
    report::write_json_report(&out_json, &doc)?;
    report::write_text_report(&out_md, &render_sbom_audit_md(&doc))?;

    if strict && !doc.overall_ok {
        bail!(
            "strict sbom-audit failed. See {}",
            out_json.display()
        );
    }

    println!("[release::sbom-audit] PASS");
    Ok(())
}

fn render_sbom_audit_md(doc: &SbomAuditDoc) -> String {
    let mut md = String::new();
    md.push_str("# SBOM Audit\n\n");
    md.push_str(&format!("- generated_utc: {}\n", doc.generated_utc));
    md.push_str(&format!("- strict: {}\n", doc.strict));
    md.push_str(&format!("- overall_ok: {}\n", doc.overall_ok));
    md.push_str(&format!("- package_count: {}\n", doc.package_count));
    md.push_str(&format!(
        "- duplicate_name_count: {}\n\n",
        doc.duplicate_name_count
    ));
    md.push_str("## Packages (sample)\n\n");
    for item in &doc.top_package_names {
        md.push_str(&format!("- {}\n", item));
    }
    md
}

pub fn score_normalize(strict: bool) -> Result<()> {
    crate::commands::release::reporting::metrics::score_normalize(strict)
}

pub fn perf_report(strict: bool) -> Result<()> {
    crate::commands::release::reporting::metrics::perf_report(strict)
}

pub fn render_perf_report_md(doc: &PerfEngineeringReportDoc) -> String {
    metrics_helpers::render_perf_report_md(doc)
}

pub fn load_or_create_perf_thresholds(path: &std::path::Path) -> Result<PerfThresholdConfig> {
    metrics_helpers::load_or_create_perf_thresholds(path)
}

pub fn load_perf_waiver(path: &std::path::Path) -> Result<PerfWaiverConfig> {
    metrics_helpers::load_perf_waiver(path)
}

pub fn render_score_normalize_md(doc: &ScoreNormalizeDoc) -> String {
    metrics_helpers::render_score_normalize_md(doc)
}

pub fn release_notes(out: Option<&str>) -> Result<()> {
    crate::commands::release::reporting::gates::release_notes(out)
}

pub fn release_manifest(strict: bool) -> Result<()> {
    println!("[release::manifest] Generating machine-readable release manifest");
    let root = paths::repo_root();

    super::ci::gate_fixup(false)?;
    super::abi::abi_drift_report(None, false)?;

    let required_paths = [
        config::repo_paths::P_TIER_STATUS_JSON,
        config::repo_paths::PRODUCTION_ACCEPTANCE_SCORECARD_JSON,
        config::repo_paths::RELEASE_EVIDENCE_BUNDLE_JSON,
        config::repo_paths::RELEASE_DIAGNOSTICS_JSON,
        config::repo_paths::CI_BUNDLE_JSON,
        config::repo_paths::ABI_DRIFT_REPORT_JSON,
        config::repo_paths::HOST_TOOL_VERIFY_JSON,
    ];

    let mut required_files = Vec::new();
    for rel in required_paths {
        required_files.push(build_file_entry(&root, rel, true)?);
    }
    let required_missing = required_files.iter().filter(|f| !f.exists).count();
    let overall_ok = required_missing == 0;

    let manifest = ReleaseManifestDoc {
        generated_utc: report::utc_now_iso(),
        strict,
        overall_ok,
        git_commit: capture_command_output("git", &["rev-parse", "HEAD"]),
        host_os: std::env::consts::OS.to_string(),
        host_arch: std::env::consts::ARCH.to_string(),
        required_missing,
        required_files,
    };

    let out_json = root.join(config::repo_paths::RELEASE_MANIFEST_JSON);
    let out_md = root.join(config::repo_paths::RELEASE_MANIFEST_MD);
    report::write_json_report(&out_json, &manifest)?;
    report::write_text_report(&out_md, &render_release_manifest_md(&manifest))?;

    if strict && !manifest.overall_ok {
        bail!(
            "strict release-manifest failed: required_missing={}. See {}",
            manifest.required_missing,
            out_json.display()
        );
    }

    println!("[release::manifest] PASS");
    Ok(())
}

pub fn support_diagnostics(strict: bool) -> Result<()> {
    crate::commands::release::reporting::gates::support_diagnostics(strict)
}

fn render_release_manifest_md(doc: &ReleaseManifestDoc) -> String {
    let mut md = String::new();
    md.push_str("# Release Manifest\n\n");
    md.push_str(&format!("- generated_utc: {}\n", doc.generated_utc));
    md.push_str(&format!("- strict: {}\n", doc.strict));
    md.push_str(&format!("- overall_ok: {}\n", doc.overall_ok));
    md.push_str(&format!("- git_commit: {}\n", doc.git_commit.clone().unwrap_or_else(|| "unknown".to_string())));
    md.push_str(&format!("- host_os: {}\n", doc.host_os));
    md.push_str(&format!("- host_arch: {}\n", doc.host_arch));
    md.push_str(&format!("- required_missing: {}\n\n", doc.required_missing));
    md.push_str("## Required Files\n\n");
    for file in &doc.required_files {
        md.push_str(&format!(
            "- [{}] {}\n",
            if file.exists { "x" } else { " " },
            file.path
        ));
    }
    md
}
