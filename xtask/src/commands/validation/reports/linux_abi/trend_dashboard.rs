use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;

use crate::config;
use crate::utils::{paths, report};

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct AbiTrendPoint {
    pub(crate) generated_utc: String,
    pub(crate) score: f64,
    pub(crate) overall_ok: bool,
}

#[derive(Serialize)]
pub(crate) struct AbiTrendDashboard {
    pub(crate) generated_utc: String,
    pub(crate) strict: bool,
    pub(crate) points: Vec<AbiTrendPoint>,
    pub(crate) latest_score: f64,
    pub(crate) regression_detected: bool,
    pub(crate) overall_ok: bool,
}

pub(crate) fn execute(limit: usize, strict: bool) -> Result<()> {
    super::semantic_matrix::execute()?;
    let root = paths::repo_root();

    let matrix_path = root.join(config::repo_paths::LINUX_ABI_SEMANTIC_MATRIX_JSON);
    let matrix_text = fs::read_to_string(&matrix_path)
        .with_context(|| format!("failed reading semantic matrix: {}", matrix_path.display()))?;
    let matrix_doc: Value = serde_json::from_str(&matrix_text)
        .with_context(|| format!("failed parsing semantic matrix: {}", matrix_path.display()))?;

    let current = AbiTrendPoint {
        generated_utc: report::utc_now_iso(),
        score: matrix_doc.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0),
        overall_ok: matrix_doc
            .get("overall_ok")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    };

    let history_path = root.join(config::repo_paths::LINUX_ABI_TREND_HISTORY_JSON);
    let mut history: Vec<AbiTrendPoint> = if history_path.exists() {
        let text = fs::read_to_string(&history_path)
            .with_context(|| format!("failed reading ABI trend history: {}", history_path.display()))?;
        serde_json::from_str(&text).unwrap_or_default()
    } else {
        Vec::new()
    };
    history.push(current.clone());
    if history.len() > limit {
        let drop = history.len().saturating_sub(limit);
        history.drain(0..drop);
    }
    report::write_json_report(&history_path, &history)?;

    let regression_detected = if history.len() >= 2 {
        let prev = &history[history.len() - 2];
        current.score + 0.1 < prev.score
    } else {
        false
    };

    let dashboard = AbiTrendDashboard {
        generated_utc: report::utc_now_iso(),
        strict,
        points: history,
        latest_score: current.score,
        regression_detected,
        overall_ok: current.overall_ok,
    };

    let out_json = root.join(config::repo_paths::LINUX_ABI_TREND_DASHBOARD_JSON);
    let out_md = root.join(config::repo_paths::LINUX_ABI_TREND_DASHBOARD_MD);
    report::write_json_report(&out_json, &dashboard)?;

    let mut md = String::new();
    md.push_str("# Linux ABI Trend Dashboard\n\n");
    md.push_str(&format!("- generated_utc: {}\n", dashboard.generated_utc));
    md.push_str(&format!("- strict: {}\n", dashboard.strict));
    md.push_str(&format!("- overall_ok: {}\n", dashboard.overall_ok));
    md.push_str(&format!("- latest_score: {:.1}\n", dashboard.latest_score));
    md.push_str(&format!("- regression_detected: {}\n\n", dashboard.regression_detected));
    md.push_str("## Points\n\n");
    for point in &dashboard.points {
        md.push_str(&format!(
            "- {} :: score={:.1} overall_ok={}\n",
            point.generated_utc, point.score, point.overall_ok
        ));
    }
    report::write_text_report(&out_md, &md)?;

    if strict && dashboard.regression_detected {
        anyhow::bail!(
            "strict linux-abi trend dashboard failed due to score regression: {}",
            out_json.display()
        );
    }

    Ok(())
}
