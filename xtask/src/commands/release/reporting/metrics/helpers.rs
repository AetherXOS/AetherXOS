use anyhow::{Context, Result};
use serde_json::Value;
use std::fs;
use std::path::Path;

use crate::commands::release::preflight::models::{
    PerfEngineeringReportDoc, PerfThresholdConfig, PerfWaiverConfig, ScoreNormalizeDoc,
    TrendDashboardDoc,
};

pub(crate) fn is_perf_relevant_check(id: &str) -> bool {
    matches!(
        id,
        "abi_drift"
            | "linux_abi_semantic_matrix"
            | "linux_abi_trend_dashboard"
            | "linux_abi_workload_catalog"
            | "linux_abi_workload_trend"
            | "glibc_compat_split"
    )
}

pub(crate) fn select_scoring_checks(checks: &[Value]) -> Vec<Value> {
    let perf_checks: Vec<Value> = checks
        .iter()
        .filter(|check| {
            check
                .get("id")
                .and_then(|v| v.as_str())
                .map(is_perf_relevant_check)
                .unwrap_or(false)
        })
        .cloned()
        .collect();

    if perf_checks.is_empty() {
        checks.to_vec()
    } else {
        perf_checks
    }
}

pub(crate) fn failed_check_count(checks: &[Value]) -> usize {
    checks
        .iter()
        .filter(|check| !check.get("ok").and_then(|v| v.as_bool()).unwrap_or(false))
        .count()
}

pub(crate) fn completion_pct(checks: &[Value], failed_count: usize) -> f64 {
    if checks.is_empty() {
        100.0
    } else {
        (((checks.len() - failed_count) as f64 / checks.len() as f64) * 1000.0).round() / 10.0
    }
}

pub(crate) fn render_trend_dashboard_md(doc: &TrendDashboardDoc) -> String {
    let mut md = String::new();
    md.push_str("# Trend Dashboard\n\n");
    md.push_str(&format!("- generated_utc: {}\n", doc.generated_utc));
    md.push_str(&format!("- strict: {}\n", doc.strict));
    md.push_str(&format!("- latest_overall_ok: {}\n", doc.latest_overall_ok));
    md.push_str(&format!(
        "- latest_failed_count: {}\n",
        doc.latest_failed_count
    ));
    md.push_str(&format!(
        "- regression_detected: {}\n\n",
        doc.regression_detected
    ));
    md.push_str("## Points\n\n");
    for point in &doc.points {
        md.push_str(&format!(
            "- {} :: overall_ok={} failed_count={} completion_pct={:.1}\n",
            point.generated_utc, point.overall_ok, point.failed_count, point.completion_pct
        ));
    }
    md
}

pub(crate) fn render_score_normalize_md(doc: &ScoreNormalizeDoc) -> String {
    let mut md = String::new();
    md.push_str("# Score Normalize\n\n");
    md.push_str(&format!("- generated_utc: {}\n", doc.generated_utc));
    md.push_str(&format!("- strict: {}\n", doc.strict));
    md.push_str(&format!("- overall_ok: {}\n", doc.overall_ok));
    md.push_str(&format!("- host_os: {}\n", doc.host_os));
    md.push_str(&format!("- host_arch: {}\n", doc.host_arch));
    md.push_str(&format!(
        "- raw_completion_pct: {:.1}\n",
        doc.raw_completion_pct
    ));
    md.push_str(&format!(
        "- normalized_score: {:.1}\n",
        doc.normalized_score
    ));
    md.push_str(&format!("- failed_checks: {}\n", doc.failed_checks));
    md
}

pub(crate) fn render_perf_report_md(doc: &PerfEngineeringReportDoc) -> String {
    let mut md = String::new();
    md.push_str("# Performance Engineering Report\n\n");
    md.push_str(&format!("- generated_utc: {}\n", doc.generated_utc));
    md.push_str(&format!("- strict: {}\n", doc.strict));
    md.push_str(&format!("- overall_ok: {}\n", doc.overall_ok));
    md.push_str(&format!(
        "- perf_engineering_score: {:.1}\n",
        doc.perf_engineering_score
    ));
    md.push_str(&format!(
        "- gate_completion_pct: {:.1}\n",
        doc.gate_completion_pct
    ));
    md.push_str(&format!(
        "- normalized_gate_score: {:.1}\n",
        doc.normalized_gate_score
    ));
    md.push_str(&format!("- failed_checks: {}\n", doc.failed_checks));
    md.push_str(&format!("- linux_abi_score: {:.1}\n", doc.linux_abi_score));
    md.push_str(&format!(
        "- release_regression_detected: {}\n",
        doc.release_regression_detected
    ));
    md.push_str("\n## Thresholds\n\n");
    md.push_str(&format!(
        "- threshold_min_perf_score: {:.1}\n",
        doc.threshold_min_perf_score
    ));
    md.push_str(&format!(
        "- threshold_min_normalized_gate_score: {:.1}\n",
        doc.threshold_min_normalized_gate_score
    ));
    md.push_str(&format!(
        "- threshold_max_failed_checks: {}\n",
        doc.threshold_max_failed_checks
    ));
    md.push_str(&format!("- threshold_source: {}\n", doc.threshold_source));
    md.push_str("\n## Waiver\n\n");
    md.push_str(&format!(
        "- waiver_allow_regression: {}\n",
        doc.waiver_allow_regression
    ));
    md.push_str(&format!(
        "- waiver_allow_below_min_score: {}\n",
        doc.waiver_allow_below_min_score
    ));
    md.push_str(&format!("- waiver_source: {}\n", doc.waiver_source));
    md
}

pub(crate) fn load_or_create_perf_thresholds(path: &Path) -> Result<PerfThresholdConfig> {
    if path.exists() {
        let text = fs::read_to_string(path)
            .with_context(|| format!("failed reading perf thresholds: {}", path.display()))?;
        return serde_json::from_str(&text)
            .with_context(|| format!("failed parsing perf thresholds: {}", path.display()));
    }

    let default_cfg = PerfThresholdConfig::default();
    let text = serde_json::to_string_pretty(&default_cfg)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed creating threshold dir: {}", parent.display()))?;
    }
    fs::write(path, text)
        .with_context(|| format!("failed writing default perf thresholds: {}", path.display()))?;
    Ok(default_cfg)
}

pub(crate) fn load_perf_waiver(path: &Path) -> Result<PerfWaiverConfig> {
    if !path.exists() {
        return Ok(PerfWaiverConfig::default());
    }
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed reading perf waiver file: {}", path.display()))?;
    serde_json::from_str(&text)
        .with_context(|| format!("failed parsing perf waiver file: {}", path.display()))
}

#[cfg(test)]
#[path = "helpers_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "render_tests.rs"]
mod render_tests;
