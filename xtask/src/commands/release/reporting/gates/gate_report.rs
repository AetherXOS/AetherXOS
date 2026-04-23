use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;

use crate::commands::release::preflight;
use crate::config;
use crate::utils::{paths, report};

#[derive(Serialize)]
pub(crate) struct GateReportDoc {
    pub(crate) generated_utc: String,
    pub(crate) strict: bool,
    pub(crate) baseline_path: String,
    pub(crate) baseline_created: bool,
    pub(crate) current_overall_ok: bool,
    pub(crate) regressions: Vec<String>,
    pub(crate) improvements: Vec<String>,
}

pub(crate) fn execute(prev: Option<&str>, strict: bool) -> Result<()> {
    println!("[release::gate-report] Generating CI gate delta report");
    let root = paths::repo_root();

    preflight::ci_bundle(false)?;
    let current_path = root.join(config::repo_paths::CI_BUNDLE_JSON);
    let current_text = fs::read_to_string(&current_path).with_context(|| {
        format!(
            "failed reading current CI bundle: {}",
            current_path.display()
        )
    })?;
    let current_doc: Value = serde_json::from_str(&current_text).with_context(|| {
        format!(
            "failed parsing current CI bundle: {}",
            current_path.display()
        )
    })?;

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

fn render_gate_report_md(report_obj: &GateReportDoc) -> String {
    let mut md = String::new();
    md.push_str("# CI Gate Report\n\n");
    md.push_str(&format!("- generated_utc: {}\n", report_obj.generated_utc));
    md.push_str(&format!("- strict: {}\n", report_obj.strict));
    md.push_str(&format!("- baseline_path: {}\n", report_obj.baseline_path));
    md.push_str(&format!(
        "- baseline_created: {}\n",
        report_obj.baseline_created
    ));
    md.push_str(&format!(
        "- current_overall_ok: {}\n",
        report_obj.current_overall_ok
    ));
    md.push_str(&format!(
        "- regressions: {}\n",
        report_obj.regressions.len()
    ));
    md.push_str(&format!(
        "- improvements: {}\n\n",
        report_obj.improvements.len()
    ));

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
