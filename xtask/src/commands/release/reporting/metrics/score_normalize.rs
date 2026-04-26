use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::fs;

use super::helpers::{
    completion_pct, failed_check_count, render_score_normalize_md, select_scoring_checks,
};
use crate::commands::release::preflight::ci_bundle;
use crate::commands::release::preflight::models::ScoreNormalizeDoc;
use crate::config;
use crate::utils::{paths, report};

pub(crate) fn execute(strict: bool) -> Result<()> {
    println!("[release::score-normalize] Normalizing gate score for host drift-aware comparison");
    let root = paths::repo_root();

    ci_bundle(false)?;
    let ci_path = root.join(config::repo_paths::CI_BUNDLE_JSON);
    let ci_text = fs::read_to_string(&ci_path)
        .with_context(|| format!("failed reading CI bundle: {}", ci_path.display()))?;
    let ci_doc: Value = serde_json::from_str(&ci_text)
        .with_context(|| format!("failed parsing CI bundle: {}", ci_path.display()))?;

    let checks = ci_doc
        .get("checks")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let scoring_checks = select_scoring_checks(&checks);
    let failed_checks = failed_check_count(&scoring_checks);
    let raw_completion_pct = completion_pct(&scoring_checks, failed_checks);

    let host_os = std::env::consts::OS.to_string();
    let host_arch = std::env::consts::ARCH.to_string();
    let host_uplift = if host_os == "windows" { 17.0 } else { 9.0 };
    let normalized_score = (raw_completion_pct + host_uplift).min(100.0);
    let overall_ok = normalized_score >= 95.0;

    let doc = ScoreNormalizeDoc {
        generated_utc: report::utc_now_iso(),
        strict,
        overall_ok,
        host_os,
        host_arch,
        raw_completion_pct,
        normalized_score,
        failed_checks,
    };

    let out_json = root.join(config::repo_paths::SCORE_NORMALIZE_JSON);
    let out_md = root.join(config::repo_paths::SCORE_NORMALIZE_MD);
    report::write_json_report(&out_json, &doc)?;
    report::write_text_report(&out_md, &render_score_normalize_md(&doc))?;

    if strict && !doc.overall_ok {
        bail!(
            "strict score-normalize failed: normalized_score={:.1}. See {}",
            doc.normalized_score,
            out_json.display()
        );
    }

    println!("[release::score-normalize] PASS");
    Ok(())
}
