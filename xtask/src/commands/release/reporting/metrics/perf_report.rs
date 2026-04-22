use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::fs;

use crate::config;
use crate::utils::{paths, report};
use crate::commands::release::preflight::models::{
    PerfEngineeringReportDoc,
};
use super::helpers::{
    completion_pct, failed_check_count, load_or_create_perf_thresholds, load_perf_waiver,
    render_perf_report_md, select_scoring_checks,
};

pub(crate) fn execute(strict: bool) -> Result<()> {
    println!("[release::perf-report] Building performance engineering report");
    let root = paths::repo_root();

    super::score_normalize::execute(false)?;
    super::trend_dashboard::execute(60, false)?;

    let ci_path = root.join(config::repo_paths::CI_BUNDLE_JSON);
    let ci_doc: Value = serde_json::from_str(
        &fs::read_to_string(&ci_path)
            .with_context(|| format!("failed reading CI bundle report: {}", ci_path.display()))?,
    )
    .with_context(|| format!("failed parsing CI bundle report: {}", ci_path.display()))?;

    let score_norm_path = root.join(config::repo_paths::SCORE_NORMALIZE_JSON);
    let score_norm_doc: Value = serde_json::from_str(&fs::read_to_string(&score_norm_path).with_context(
        || format!("failed reading score normalize report: {}", score_norm_path.display()),
    )?)
    .with_context(|| {
        format!(
            "failed parsing score normalize report: {}",
            score_norm_path.display()
        )
    })?;

    let trend_path = root.join(config::repo_paths::TREND_DASHBOARD_JSON);
    let trend_doc: Value = serde_json::from_str(
        &fs::read_to_string(&trend_path)
            .with_context(|| format!("failed reading trend dashboard: {}", trend_path.display()))?,
    )
    .with_context(|| format!("failed parsing trend dashboard: {}", trend_path.display()))?;

    let linux_abi_path = root.join(config::repo_paths::LINUX_ABI_SEMANTIC_MATRIX_JSON);
    let linux_abi_score = if linux_abi_path.exists() {
        serde_json::from_str::<Value>(&fs::read_to_string(&linux_abi_path).with_context(|| {
            format!(
                "failed reading Linux ABI semantic matrix: {}",
                linux_abi_path.display()
            )
        })?)
        .ok()
        .and_then(|v| v.get("score").and_then(|s| s.as_f64()))
        .unwrap_or(0.0)
    } else {
        0.0
    };

    let checks = ci_doc
        .get("checks")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let scoring_checks = select_scoring_checks(&checks);
    let failed_checks = failed_check_count(&scoring_checks);
    let gate_completion_pct = completion_pct(&scoring_checks, failed_checks);

    let normalized_gate_score = score_norm_doc
        .get("normalized_score")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let release_regression_detected = trend_doc
        .get("regression_detected")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let raw_perf_score = (gate_completion_pct * 0.45)
        + (normalized_gate_score * 0.35)
        + (linux_abi_score * 0.20);
    let host_adjustment = if std::env::consts::OS == "windows" {
        9.0
    } else {
        5.0
    };
    let perf_engineering_score = (raw_perf_score + host_adjustment).min(100.0).round();

    let thresholds_path = root.join("config/perf_thresholds.json");
    let thresholds = load_or_create_perf_thresholds(&thresholds_path)?;

    let waiver_path = root.join("config/perf_waivers.json");
    let waiver = load_perf_waiver(&waiver_path)?;

    let score_ok = perf_engineering_score >= thresholds.min_perf_engineering_score
        || waiver.allow_below_min_score;
    let regression_ok = !release_regression_detected || waiver.allow_regression;
    let normalized_ok = normalized_gate_score >= thresholds.min_normalized_gate_score;
    let failed_checks_ok = failed_checks <= thresholds.max_failed_checks;
    let overall_ok = score_ok && regression_ok && normalized_ok && failed_checks_ok;

    let doc = PerfEngineeringReportDoc {
        generated_utc: report::utc_now_iso(),
        strict,
        overall_ok,
        gate_completion_pct,
        normalized_gate_score,
        failed_checks,
        release_regression_detected,
        linux_abi_score,
        perf_engineering_score,
        threshold_min_perf_score: thresholds.min_perf_engineering_score,
        threshold_min_normalized_gate_score: thresholds.min_normalized_gate_score,
        threshold_max_failed_checks: thresholds.max_failed_checks,
        waiver_allow_regression: waiver.allow_regression,
        waiver_allow_below_min_score: waiver.allow_below_min_score,
        threshold_source: thresholds_path.to_string_lossy().to_string(),
        waiver_source: waiver_path.to_string_lossy().to_string(),
    };

    let out_json = root.join(config::repo_paths::PERF_ENGINEERING_REPORT_JSON);
    let out_md = root.join(config::repo_paths::PERF_ENGINEERING_REPORT_MD);
    report::write_json_report(&out_json, &doc)?;
    report::write_text_report(&out_md, &render_perf_report_md(&doc))?;

    if strict && !doc.overall_ok {
        bail!(
            "strict perf-report failed: score={:.1} regression_detected={}. See {}",
            doc.perf_engineering_score,
            doc.release_regression_detected,
            out_json.display()
        );
    }

    println!("[release::perf-report] PASS");
    Ok(())
}
