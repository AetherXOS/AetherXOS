use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_json::Value;
use std::fs;

use crate::commands::release::preflight;
use crate::config;
use crate::utils::{paths, report};

#[derive(Serialize)]
pub(crate) struct ExplainFailureDoc {
    pub(crate) generated_utc: String,
    pub(crate) strict: bool,
    pub(crate) overall_ok: bool,
    pub(crate) issue_count: usize,
    pub(crate) action_plan: Vec<String>,
}

pub(crate) fn execute(strict: bool) -> Result<()> {
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

#[cfg(test)]
#[path = "explain_failure_tests.rs"]
mod tests;
