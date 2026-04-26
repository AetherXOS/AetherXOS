use anyhow::{Context, Result};
use serde_json::Value;
use std::fs;

use crate::commands::release::preflight;
use crate::config;
use crate::utils::{paths, report};

pub(crate) fn execute(out: Option<&str>) -> Result<()> {
    println!("[release::notes] Generating release notes from current gate state");
    let root = paths::repo_root();

    preflight::ci_bundle(false)?;
    super::explain_failure::execute(false)?;

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
    let plan = diag_doc
        .get("action_plan")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let notes = render_release_notes_md(&checks, &plan, overall_ok);

    let out_rel = out.unwrap_or(config::repo_paths::RELEASE_NOTES_MD);
    let out_path = root.join(out_rel);
    report::write_text_report(&out_path, &notes)?;

    println!("[release::notes] PASS");
    Ok(())
}

pub(crate) fn render_release_notes_md(
    checks: &[Value],
    plan: &[Value],
    overall_ok: bool,
) -> String {
    let mut notes = String::new();
    notes.push_str("# Release Notes (Auto)\n\n");
    notes.push_str(&format!("- generated_utc: {}\n", report::utc_now_iso()));
    notes.push_str(&format!("- overall_ok: {}\n\n", overall_ok));
    notes.push_str("## Gate Summary\n\n");
    for check in checks {
        let id = check
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let ok = check.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        notes.push_str(&format!("- [{}] {}\n", if ok { "x" } else { " " }, id));
    }

    notes.push_str("\n## Remediation Plan\n\n");
    for line in plan {
        if let Some(step) = line.as_str() {
            notes.push_str(&format!("- {}\n", step));
        }
    }

    notes
}

#[cfg(test)]
#[path = "release_notes_tests.rs"]
mod tests;
