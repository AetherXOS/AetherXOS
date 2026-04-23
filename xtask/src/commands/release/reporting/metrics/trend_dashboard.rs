use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::fs;

use super::helpers::render_trend_dashboard_md;
use crate::commands::release::preflight::ci_bundle;
use crate::commands::release::preflight::models::{TrendDashboardDoc, TrendPoint};
use crate::config;
use crate::utils::{paths, report};

pub(crate) fn execute(limit: usize, strict: bool) -> Result<()> {
    println!("[release::trend-dashboard] Updating trend history and dashboard");
    let root = paths::repo_root();

    ci_bundle(false)?;
    let ci_path = root.join(config::repo_paths::CI_BUNDLE_JSON);
    let ci_text = fs::read_to_string(&ci_path)
        .with_context(|| format!("failed reading CI bundle report: {}", ci_path.display()))?;
    let ci_doc: Value = serde_json::from_str(&ci_text)
        .with_context(|| format!("failed parsing CI bundle report: {}", ci_path.display()))?;

    let checks = ci_doc
        .get("checks")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let failed_count = checks
        .iter()
        .filter(|check| !check.get("ok").and_then(|v| v.as_bool()).unwrap_or(false))
        .count();
    let completion_pct = if checks.is_empty() {
        100.0
    } else {
        (((checks.len() - failed_count) as f64 / checks.len() as f64) * 1000.0).round() / 10.0
    };
    let current = TrendPoint {
        generated_utc: report::utc_now_iso(),
        overall_ok: ci_doc
            .get("overall_ok")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        failed_count,
        completion_pct,
    };

    let history_path = root.join(config::repo_paths::TREND_HISTORY_JSON);
    let mut history: Vec<TrendPoint> = if history_path.exists() {
        let text = fs::read_to_string(&history_path)
            .with_context(|| format!("failed reading trend history: {}", history_path.display()))?;
        serde_json::from_str(&text).unwrap_or_default()
    } else {
        Vec::new()
    };
    history.push(current.clone());
    if history.len() > limit {
        let drop_count = history.len().saturating_sub(limit);
        history.drain(0..drop_count);
    }
    report::write_json_report(&history_path, &history)?;

    let regression_detected = if history.len() >= 2 {
        let prev = &history[history.len() - 2];
        current.failed_count > prev.failed_count
    } else {
        false
    };

    let dashboard = TrendDashboardDoc {
        generated_utc: report::utc_now_iso(),
        strict,
        points: history,
        latest_overall_ok: current.overall_ok,
        latest_failed_count: current.failed_count,
        regression_detected,
    };

    let out_json = root.join(config::repo_paths::TREND_DASHBOARD_JSON);
    let out_md = root.join(config::repo_paths::TREND_DASHBOARD_MD);
    report::write_json_report(&out_json, &dashboard)?;
    report::write_text_report(&out_md, &render_trend_dashboard_md(&dashboard))?;

    if strict && dashboard.regression_detected {
        bail!(
            "strict trend-dashboard failed: regression detected. See {}",
            out_json.display()
        );
    }

    println!("[release::trend-dashboard] PASS");
    Ok(())
}
