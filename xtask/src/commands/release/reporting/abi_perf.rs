use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_json::Value;
use std::fs;

use crate::cli::LinuxAbiAction;
use crate::commands::release::reporting::metrics;
use crate::commands::validation;
use crate::config;
use crate::utils::{paths, report};

#[derive(Serialize)]
struct AbiPerfGateDoc {
    generated_utc: String,
    strict: bool,
    overall_ok: bool,
    linux_abi_semantic_ok: bool,
    linux_abi_score: f64,
    linux_abi_trend_regression: bool,
    linux_abi_workload_ok: bool,
    linux_abi_workload_pass_rate_pct: f64,
    perf_report_ok: bool,
    perf_engineering_score: f64,
    perf_regression_detected: bool,
    failures: Vec<String>,
}

pub(crate) fn abi_perf_gate(strict: bool) -> Result<()> {
    println!("[release::abi-perf-gate] Evaluating ABI and performance production gates");
    let root = paths::repo_root();

    validation::linux_abi::execute(&LinuxAbiAction::SemanticMatrix)?;
    validation::linux_abi::execute(&LinuxAbiAction::TrendDashboard {
        limit: 60,
        strict: false,
    })?;
    validation::linux_abi::execute(&LinuxAbiAction::WorkloadCatalog {
        limit: 60,
        strict: false,
    })?;
    metrics::perf_report(false)?;

    let semantic = read_json(root.join(config::repo_paths::LINUX_ABI_SEMANTIC_MATRIX_JSON))?;
    let trend = read_json(root.join(config::repo_paths::LINUX_ABI_TREND_DASHBOARD_JSON))?;
    let workload = read_json(root.join(config::repo_paths::LINUX_ABI_WORKLOAD_TREND_JSON))?;
    let perf = read_json(root.join(config::repo_paths::PERF_ENGINEERING_REPORT_JSON))?;

    let linux_abi_semantic_ok = semantic
        .get("overall_ok")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let linux_abi_score = semantic
        .get("score")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let linux_abi_trend_regression = trend
        .get("regression_detected")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let linux_abi_workload_ok = workload
        .get("overall_ok")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let linux_abi_workload_pass_rate_pct = workload
        .get("latest_pass_rate_pct")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let perf_report_ok = perf
        .get("overall_ok")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let perf_engineering_score = perf
        .get("perf_engineering_score")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let perf_regression_detected = perf
        .get("release_regression_detected")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let mut failures = Vec::new();
    if !linux_abi_semantic_ok {
        failures.push(format!(
            "linux_abi_semantic_matrix overall_ok=false score={:.1}",
            linux_abi_score
        ));
    }
    if linux_abi_trend_regression {
        failures.push("linux_abi_trend_dashboard regression_detected=true".to_string());
    }
    if !linux_abi_workload_ok {
        failures.push(format!(
            "linux_abi_workload_trend overall_ok=false latest_pass_rate_pct={:.1}",
            linux_abi_workload_pass_rate_pct
        ));
    }
    if !perf_report_ok {
        failures.push(format!(
            "perf_engineering_report overall_ok=false score={:.1} regression_detected={}",
            perf_engineering_score, perf_regression_detected
        ));
    }

    let overall_ok = failures.is_empty();
    let doc = AbiPerfGateDoc {
        generated_utc: report::utc_now_iso(),
        strict,
        overall_ok,
        linux_abi_semantic_ok,
        linux_abi_score,
        linux_abi_trend_regression,
        linux_abi_workload_ok,
        linux_abi_workload_pass_rate_pct,
        perf_report_ok,
        perf_engineering_score,
        perf_regression_detected,
        failures,
    };

    let out_json = root.join(config::repo_paths::ABI_PERF_GATE_JSON);
    let out_md = root.join(config::repo_paths::ABI_PERF_GATE_MD);
    report::write_json_report(&out_json, &doc)?;
    report::write_text_report(&out_md, &render_abi_perf_gate_md(&doc))?;

    if strict && !doc.overall_ok {
        bail!(
            "strict abi-perf gate failed: {}. See {}",
            doc.failures.join("; "),
            out_json.display()
        );
    }

    println!("[release::abi-perf-gate] PASS");
    Ok(())
}

fn read_json(path: std::path::PathBuf) -> Result<Value> {
    let text = fs::read_to_string(&path)
        .with_context(|| format!("failed reading JSON report: {}", path.display()))?;
    serde_json::from_str(&text)
        .with_context(|| format!("failed parsing JSON report: {}", path.display()))
}

fn render_abi_perf_gate_md(doc: &AbiPerfGateDoc) -> String {
    let mut md = String::new();
    md.push_str("# ABI + Performance Gate\n\n");
    md.push_str(&format!("- generated_utc: {}\n", doc.generated_utc));
    md.push_str(&format!("- strict: {}\n", doc.strict));
    md.push_str(&format!("- overall_ok: {}\n", doc.overall_ok));
    md.push_str(&format!(
        "- linux_abi_semantic_ok: {} (score={:.1})\n",
        doc.linux_abi_semantic_ok, doc.linux_abi_score
    ));
    md.push_str(&format!(
        "- linux_abi_trend_regression: {}\n",
        doc.linux_abi_trend_regression
    ));
    md.push_str(&format!(
        "- linux_abi_workload_ok: {} (latest_pass_rate_pct={:.1})\n",
        doc.linux_abi_workload_ok, doc.linux_abi_workload_pass_rate_pct
    ));
    md.push_str(&format!(
        "- perf_report_ok: {} (score={:.1}, regression_detected={})\n\n",
        doc.perf_report_ok, doc.perf_engineering_score, doc.perf_regression_detected
    ));

    if doc.failures.is_empty() {
        md.push_str("No failures detected.\n");
        return md;
    }

    md.push_str("## Failures\n\n");
    for failure in &doc.failures {
        md.push_str(&format!("- {}\n", failure));
    }
    md
}
