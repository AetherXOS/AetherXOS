use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::fs;
use std::path::Path;

use crate::config;
use crate::utils::{paths, report};

use crate::commands::release::preflight::models::{
    ReleaseDiagnosticIssue, ReleaseDiagnosticsReport,
};

pub fn execute(strict: bool) -> Result<()> {
    println!("[release::diagnostics] Generating release diagnostics");
    let root = paths::repo_root();

    let mut issues = Vec::new();
    collect_scorecard_issues(&root, &mut issues)?;
    collect_p_tier_issues(&root, &mut issues)?;
    collect_evidence_bundle_issues(&root, &mut issues)?;
    collect_overall_ok_issue(
        &root,
        &mut issues,
        "host_tool_verify_failed",
        "high",
        config::repo_paths::HOST_TOOL_VERIFY_JSON,
        "Host tool verification is not green",
        "Run: cargo run -p xtask -- release host-tool-verify --strict",
    )?;
    collect_overall_ok_issue(
        &root,
        &mut issues,
        "critical_policy_guard_failed",
        "high",
        config::repo_paths::CRITICAL_POLICY_GUARD_JSON,
        "Critical policy guard is not green",
        "Run: cargo run -p xtask -- release policy-guard --strict",
    )?;
    collect_overall_ok_issue(
        &root,
        &mut issues,
        "warning_audit_failed",
        "high",
        config::repo_paths::WARNING_AUDIT_JSON,
        "Warning audit is not green",
        "Run: cargo run -p xtask -- release warning-audit --strict",
    )?;

    let overall_ok = issues.is_empty();
    let report_obj = ReleaseDiagnosticsReport {
        generated_utc: report::utc_now_iso(),
        overall_ok,
        strict,
        issue_count: issues.len(),
        issues,
    };

    let out_json = root.join(config::repo_paths::RELEASE_DIAGNOSTICS_JSON);
    let out_md = root.join(config::repo_paths::RELEASE_DIAGNOSTICS_MD);
    report::write_json_report(&out_json, &report_obj)?;
    report::write_text_report(&out_md, &render_release_diagnostics_md(&report_obj))?;

    if strict && !report_obj.overall_ok {
        bail!(
            "strict release diagnostics failed: issue_count={}. See {}",
            report_obj.issue_count,
            out_json.display()
        );
    }

    println!("[release::diagnostics] PASS");
    Ok(())
}

fn collect_scorecard_issues(root: &Path, out: &mut Vec<ReleaseDiagnosticIssue>) -> Result<()> {
    let path = root.join(config::repo_paths::PRODUCTION_ACCEPTANCE_SCORECARD_JSON);
    if !path.exists() {
        out.push(ReleaseDiagnosticIssue {
            id: "scorecard_missing".to_string(),
            severity: "critical".to_string(),
            source: config::repo_paths::PRODUCTION_ACCEPTANCE_SCORECARD_JSON.to_string(),
            detail: "production acceptance scorecard report does not exist".to_string(),
            remediation: "Run: cargo run -p xtask -- release preflight".to_string(),
        });
        return Ok(());
    }

    let text = fs::read_to_string(&path)
        .with_context(|| format!("failed reading scorecard: {}", path.display()))?;
    let doc: Value = serde_json::from_str(&text)
        .with_context(|| format!("failed parsing scorecard: {}", path.display()))?;

    let overall_ok = doc
        .get("overall_ok")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !overall_ok {
        let completion = doc
            .get("completion_pct")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        out.push(ReleaseDiagnosticIssue {
            id: "scorecard_gate_failed".to_string(),
            severity: "critical".to_string(),
            source: config::repo_paths::PRODUCTION_ACCEPTANCE_SCORECARD_JSON.to_string(),
            detail: format!("overall_ok=false completion_pct={completion:.1}"),
            remediation: "Run: cargo run -p xtask -- release p0-p1-nightly ; then inspect missing gates in production_release_acceptance_scorecard.json".to_string(),
        });
    }
    Ok(())
}

fn collect_p_tier_issues(root: &Path, out: &mut Vec<ReleaseDiagnosticIssue>) -> Result<()> {
    let path = root.join(config::repo_paths::P_TIER_STATUS_JSON);
    if !path.exists() {
        out.push(ReleaseDiagnosticIssue {
            id: "p_tier_missing".to_string(),
            severity: "critical".to_string(),
            source: config::repo_paths::P_TIER_STATUS_JSON.to_string(),
            detail: "p-tier status report does not exist".to_string(),
            remediation: "Run: cargo run -p xtask -- release p1-nightly".to_string(),
        });
        return Ok(());
    }

    let text = fs::read_to_string(&path)
        .with_context(|| format!("failed reading p-tier status: {}", path.display()))?;
    let doc: Value = serde_json::from_str(&text)
        .with_context(|| format!("failed parsing p-tier status: {}", path.display()))?;

    let overall_ok = doc
        .get("overall_ok")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !overall_ok {
        out.push(ReleaseDiagnosticIssue {
            id: "p_tier_failed".to_string(),
            severity: "critical".to_string(),
            source: config::repo_paths::P_TIER_STATUS_JSON.to_string(),
            detail: "overall_ok=false".to_string(),
            remediation: "Inspect blockers[] in p_tier_status.json and rerun impacted xtask validation commands".to_string(),
        });
    }

    let regression = doc
        .get("trend")
        .and_then(|v| v.get("overall_regression"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if regression {
        out.push(ReleaseDiagnosticIssue {
            id: "p_tier_regression".to_string(),
            severity: "high".to_string(),
            source: config::repo_paths::P_TIER_STATUS_JSON.to_string(),
            detail: "trend.overall_regression=true".to_string(),
            remediation: "Compare current and previous tier scores, then restore failing required checks before release".to_string(),
        });
    }

    Ok(())
}

fn collect_evidence_bundle_issues(
    root: &Path,
    out: &mut Vec<ReleaseDiagnosticIssue>,
) -> Result<()> {
    let path = root.join(config::repo_paths::RELEASE_EVIDENCE_BUNDLE_JSON);
    if !path.exists() {
        out.push(ReleaseDiagnosticIssue {
            id: "evidence_bundle_missing".to_string(),
            severity: "high".to_string(),
            source: config::repo_paths::RELEASE_EVIDENCE_BUNDLE_JSON.to_string(),
            detail: "release evidence bundle does not exist".to_string(),
            remediation: "Run: cargo run -p xtask -- release evidence-bundle".to_string(),
        });
        return Ok(());
    }

    let text = fs::read_to_string(&path)
        .with_context(|| format!("failed reading evidence bundle: {}", path.display()))?;
    let doc: Value = serde_json::from_str(&text)
        .with_context(|| format!("failed parsing evidence bundle: {}", path.display()))?;

    let required_missing = doc
        .get("required_missing")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let required_gate_failures = doc
        .get("required_gate_failures")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    if required_missing > 0 || required_gate_failures > 0 {
        out.push(ReleaseDiagnosticIssue {
            id: "evidence_bundle_not_green".to_string(),
            severity: "critical".to_string(),
            source: config::repo_paths::RELEASE_EVIDENCE_BUNDLE_JSON.to_string(),
            detail: format!(
                "required_missing={} required_gate_failures={}",
                required_missing, required_gate_failures
            ),
            remediation: "Run: cargo run -p xtask -- release evidence-bundle --strict and address failing_required_gates[]".to_string(),
        });
    }
    Ok(())
}

fn render_release_diagnostics_md(report_obj: &ReleaseDiagnosticsReport) -> String {
    let mut md = String::new();
    md.push_str("# Release Diagnostics\n\n");
    md.push_str(&format!("- generated_utc: {}\n", report_obj.generated_utc));
    md.push_str(&format!("- overall_ok: {}\n", report_obj.overall_ok));
    md.push_str(&format!("- strict: {}\n", report_obj.strict));
    md.push_str(&format!("- issue_count: {}\n\n", report_obj.issue_count));

    if report_obj.issues.is_empty() {
        md.push_str("No blocking issues found.\n");
        return md;
    }

    md.push_str("## Issues\n\n");
    for issue in &report_obj.issues {
        md.push_str(&format!("- id: {}\n", issue.id));
        md.push_str(&format!("  - severity: {}\n", issue.severity));
        md.push_str(&format!("  - source: {}\n", issue.source));
        md.push_str(&format!("  - detail: {}\n", issue.detail));
        md.push_str(&format!("  - remediation: {}\n", issue.remediation));
    }

    md
}

fn collect_overall_ok_issue(
    root: &Path,
    out: &mut Vec<ReleaseDiagnosticIssue>,
    issue_id: &str,
    severity: &str,
    rel_path: &str,
    detail_text: &str,
    remediation: &str,
) -> Result<()> {
    let path = root.join(rel_path);
    if !path.exists() {
        out.push(ReleaseDiagnosticIssue {
            id: format!("{}_missing", issue_id),
            severity: severity.to_string(),
            source: rel_path.to_string(),
            detail: "report does not exist".to_string(),
            remediation: remediation.to_string(),
        });
        return Ok(());
    }

    let text = fs::read_to_string(&path)
        .with_context(|| format!("failed reading report: {}", path.display()))?;
    let doc: Value = serde_json::from_str(&text)
        .with_context(|| format!("failed parsing report: {}", path.display()))?;

    let overall_ok = doc
        .get("overall_ok")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    if !overall_ok {
        out.push(ReleaseDiagnosticIssue {
            id: issue_id.to_string(),
            severity: severity.to_string(),
            source: rel_path.to_string(),
            detail: detail_text.to_string(),
            remediation: remediation.to_string(),
        });
    }

    Ok(())
}
