use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;

use crate::commands::release::preflight;
use crate::config;
use crate::utils::{paths, report};

#[derive(Serialize)]
struct GateReportDoc {
    generated_utc: String,
    strict: bool,
    baseline_path: String,
    baseline_created: bool,
    current_overall_ok: bool,
    regressions: Vec<String>,
    improvements: Vec<String>,
}

#[derive(Serialize)]
struct ExplainFailureDoc {
    generated_utc: String,
    strict: bool,
    overall_ok: bool,
    issue_count: usize,
    action_plan: Vec<String>,
}

#[derive(Serialize)]
struct SupportCheck {
    id: String,
    ok: bool,
    detail: String,
}

#[derive(Serialize)]
struct SupportDiagnosticsDoc {
    generated_utc: String,
    strict: bool,
    overall_ok: bool,
    commands: Vec<String>,
    status: Vec<SupportCheck>,
}

pub(crate) fn gate_report(prev: Option<&str>, strict: bool) -> Result<()> {
    println!("[release::gate-report] Generating CI gate delta report");
    let root = paths::repo_root();

    preflight::ci_bundle(false)?;
    let current_path = root.join(config::repo_paths::CI_BUNDLE_JSON);
    let current_text = fs::read_to_string(&current_path)
        .with_context(|| format!("failed reading current CI bundle: {}", current_path.display()))?;
    let current_doc: Value = serde_json::from_str(&current_text)
        .with_context(|| format!("failed parsing current CI bundle: {}", current_path.display()))?;

    let baseline_rel = prev.unwrap_or("reports/tooling/ci_bundle_prev.json");
    let baseline_path = root.join(baseline_rel);
    let baseline_exists = baseline_path.exists();
    if !baseline_exists {
        fs::write(&baseline_path, &current_text)
            .with_context(|| format!("failed creating CI baseline: {}", baseline_path.display()))?;
    }

    let baseline_text = fs::read_to_string(&baseline_path)
        .with_context(|| format!("failed reading CI baseline: {}", baseline_path.display()))?;
    let baseline_doc: Value = serde_json::from_str(&baseline_text)
        .with_context(|| format!("failed parsing CI baseline: {}", baseline_path.display()))?;

    let current_checks = current_doc
        .get("checks")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let baseline_checks = baseline_doc
        .get("checks")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut baseline_map = BTreeMap::new();
    for check in baseline_checks {
        if let Some(id) = check.get("id").and_then(|v| v.as_str()) {
            let ok = check.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
            baseline_map.insert(id.to_string(), ok);
        }
    }

    let mut regressions = Vec::new();
    let mut improvements = Vec::new();
    for check in current_checks {
        let id = check
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let current_ok = check.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        if let Some(prev_ok) = baseline_map.get(id) {
            if *prev_ok && !current_ok {
                regressions.push(id.to_string());
            }
            if !*prev_ok && current_ok {
                improvements.push(id.to_string());
            }
        }
    }

    let current_overall_ok = current_doc
        .get("overall_ok")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let report_obj = GateReportDoc {
        generated_utc: report::utc_now_iso(),
        strict,
        baseline_path: baseline_rel.to_string(),
        baseline_created: !baseline_exists,
        current_overall_ok,
        regressions,
        improvements,
    };

    let out_json = root.join(config::repo_paths::CI_GATE_REPORT_JSON);
    let out_md = root.join(config::repo_paths::CI_GATE_REPORT_MD);
    report::write_json_report(&out_json, &report_obj)?;
    report::write_text_report(&out_md, &render_gate_report_md(&report_obj))?;

    if strict && !report_obj.regressions.is_empty() {
        bail!(
            "strict gate-report failed: regressions={}. See {}",
            report_obj.regressions.len(),
            out_json.display()
        );
    }

    println!("[release::gate-report] PASS");
    Ok(())
}

pub(crate) fn explain_failure(strict: bool) -> Result<()> {
    println!("[release::explain-failure] Building actionable remediation plan");
    let root = paths::repo_root();

    preflight::release_diagnostics(false)?;
    let diag_path = root.join(config::repo_paths::RELEASE_DIAGNOSTICS_JSON);
    let diag_text = fs::read_to_string(&diag_path)
        .with_context(|| format!("failed reading diagnostics report: {}", diag_path.display()))?;
    let diag_doc: Value = serde_json::from_str(&diag_text)
        .with_context(|| format!("failed parsing diagnostics report: {}", diag_path.display()))?;

    let overall_ok = diag_doc
        .get("overall_ok")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let issues = diag_doc
        .get("issues")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut plan = Vec::new();
    plan.push("1) Run `cargo run -p xtask -- release gate-fixup` to refresh all gate artifacts.".to_string());
    for issue in &issues {
        let id = issue
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown_issue");
        let remediation = issue
            .get("remediation")
            .and_then(|v| v.as_str())
            .unwrap_or("inspect related report and fix root cause");
        plan.push(format!("- [{id}] {remediation}"));
    }
    plan.push("N) Re-run strict gates: `release doctor --strict`, `release ci-bundle --strict`, `release export-junit --strict`.".to_string());

    let doc = ExplainFailureDoc {
        generated_utc: report::utc_now_iso(),
        strict,
        overall_ok,
        issue_count: issues.len(),
        action_plan: plan,
    };

    let out_json = root.join(config::repo_paths::EXPLAIN_FAILURE_JSON);
    let out_md = root.join(config::repo_paths::EXPLAIN_FAILURE_MD);
    report::write_json_report(&out_json, &doc)?;
    report::write_text_report(&out_md, &render_explain_failure_md(&doc))?;

    if strict && !doc.overall_ok {
        bail!(
            "strict explain-failure gate: unresolved issues remain ({}). See {}",
            doc.issue_count,
            out_json.display()
        );
    }

    println!("[release::explain-failure] PASS");
    Ok(())
}

pub(crate) fn release_notes(out: Option<&str>) -> Result<()> {
    println!("[release::notes] Generating release notes from current gate state");
    let root = paths::repo_root();

    preflight::ci_bundle(false)?;
    explain_failure(false)?;

    let ci_path = root.join(config::repo_paths::CI_BUNDLE_JSON);
    let ci_text = fs::read_to_string(&ci_path)
        .with_context(|| format!("failed reading CI bundle: {}", ci_path.display()))?;
    let ci_doc: Value = serde_json::from_str(&ci_text)
        .with_context(|| format!("failed parsing CI bundle: {}", ci_path.display()))?;

    let diag_path = root.join(config::repo_paths::EXPLAIN_FAILURE_JSON);
    let diag_text = fs::read_to_string(&diag_path).with_context(|| {
        format!(
            "failed reading explain-failure report: {}",
            diag_path.display()
        )
    })?;
    let diag_doc: Value = serde_json::from_str(&diag_text).with_context(|| {
        format!(
            "failed parsing explain-failure report: {}",
            diag_path.display()
        )
    })?;

    let overall_ok = ci_doc
        .get("overall_ok")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let checks = ci_doc
        .get("checks")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut notes = String::new();
    notes.push_str("# Release Notes (Auto)\n\n");
    notes.push_str(&format!("- generated_utc: {}\n", report::utc_now_iso()));
    notes.push_str(&format!("- overall_ok: {}\n\n", overall_ok));
    notes.push_str("## Gate Summary\n\n");
    for check in checks {
        let id = check.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
        let ok = check.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        notes.push_str(&format!("- [{}] {}\n", if ok { "x" } else { " " }, id));
    }

    notes.push_str("\n## Remediation Plan\n\n");
    if let Some(plan) = diag_doc.get("action_plan").and_then(|v| v.as_array()) {
        for line in plan {
            if let Some(step) = line.as_str() {
                notes.push_str(&format!("- {}\n", step));
            }
        }
    }

    let out_rel = out.unwrap_or(config::repo_paths::RELEASE_NOTES_MD);
    let out_path = root.join(out_rel);
    report::write_text_report(&out_path, &notes)?;

    println!("[release::notes] PASS");
    Ok(())
}

pub(crate) fn support_diagnostics(strict: bool) -> Result<()> {
    println!("[release::support-diagnostics] Building support diagnostics bundle");
    let root = paths::repo_root();

    preflight::release_doctor(false)?;
    gate_report(None, false)?;
    preflight::export_junit(None, false)?;

    let specs = [
        ("doctor", config::repo_paths::RELEASE_DOCTOR_JSON),
        ("diagnostics", config::repo_paths::RELEASE_DIAGNOSTICS_JSON),
        ("ci_bundle", config::repo_paths::CI_BUNDLE_JSON),
        ("gate_report", config::repo_paths::CI_GATE_REPORT_JSON),
    ];

    let mut status = Vec::new();
    for (id, rel) in specs {
        let path = root.join(rel);
        if !path.exists() {
            status.push(SupportCheck {
                id: id.to_string(),
                ok: false,
                detail: format!("missing {}", rel),
            });
            continue;
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed reading support input: {}", path.display()))?;
        let doc: Value = serde_json::from_str(&text)
            .with_context(|| format!("failed parsing support input: {}", path.display()))?;
        let ok = doc
            .get("overall_ok")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        status.push(SupportCheck {
            id: id.to_string(),
            ok,
            detail: if ok {
                "overall_ok=true".to_string()
            } else {
                "overall_ok=false".to_string()
            },
        });
    }

    let commands = vec![
        "cargo run -p xtask -- release gate-fixup".to_string(),
        "cargo run -p xtask -- release doctor --strict".to_string(),
        "cargo run -p xtask -- release ci-bundle --strict".to_string(),
        "cargo run -p xtask -- release explain-failure".to_string(),
    ];
    let overall_ok = status.iter().all(|item| item.ok);

    let doc = SupportDiagnosticsDoc {
        generated_utc: report::utc_now_iso(),
        strict,
        overall_ok,
        commands,
        status,
    };

    let out_json = root.join(config::repo_paths::SUPPORT_DIAGNOSTICS_JSON);
    let out_md = root.join(config::repo_paths::SUPPORT_DIAGNOSTICS_MD);
    report::write_json_report(&out_json, &doc)?;
    report::write_text_report(&out_md, &render_support_diagnostics_md(&doc))?;

    if strict && !doc.overall_ok {
        bail!(
            "strict support-diagnostics failed; unresolved checks remain. See {}",
            out_json.display()
        );
    }

    println!("[release::support-diagnostics] PASS");
    Ok(())
}

fn render_gate_report_md(report_obj: &GateReportDoc) -> String {
    let mut md = String::new();
    md.push_str("# CI Gate Report\n\n");
    md.push_str(&format!("- generated_utc: {}\n", report_obj.generated_utc));
    md.push_str(&format!("- strict: {}\n", report_obj.strict));
    md.push_str(&format!("- baseline_path: {}\n", report_obj.baseline_path));
    md.push_str(&format!("- baseline_created: {}\n", report_obj.baseline_created));
    md.push_str(&format!("- current_overall_ok: {}\n", report_obj.current_overall_ok));
    md.push_str(&format!("- regressions: {}\n", report_obj.regressions.len()));
    md.push_str(&format!("- improvements: {}\n\n", report_obj.improvements.len()));

    if !report_obj.regressions.is_empty() {
        md.push_str("## Regressions\n\n");
        for item in &report_obj.regressions {
            md.push_str(&format!("- {}\n", item));
        }
        md.push('\n');
    }
    if !report_obj.improvements.is_empty() {
        md.push_str("## Improvements\n\n");
        for item in &report_obj.improvements {
            md.push_str(&format!("- {}\n", item));
        }
    }
    md
}

fn render_explain_failure_md(doc: &ExplainFailureDoc) -> String {
    let mut md = String::new();
    md.push_str("# Explain Failure\n\n");
    md.push_str(&format!("- generated_utc: {}\n", doc.generated_utc));
    md.push_str(&format!("- strict: {}\n", doc.strict));
    md.push_str(&format!("- overall_ok: {}\n", doc.overall_ok));
    md.push_str(&format!("- issue_count: {}\n\n", doc.issue_count));
    md.push_str("## Action Plan\n\n");
    for step in &doc.action_plan {
        md.push_str(&format!("- {}\n", step));
    }
    md
}

fn render_support_diagnostics_md(doc: &SupportDiagnosticsDoc) -> String {
    let mut md = String::new();
    md.push_str("# Support Diagnostics\n\n");
    md.push_str(&format!("- generated_utc: {}\n", doc.generated_utc));
    md.push_str(&format!("- strict: {}\n", doc.strict));
    md.push_str(&format!("- overall_ok: {}\n\n", doc.overall_ok));
    md.push_str("## Status\n\n");
    for item in &doc.status {
        md.push_str(&format!(
            "- [{}] {} ({})\n",
            if item.ok { "x" } else { " " },
            item.id,
            item.detail
        ));
    }
    md.push_str("\n## Suggested Commands\n\n");
    for cmd in &doc.commands {
        md.push_str(&format!("- {}\n", cmd));
    }
    md
}
