use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::config;
use crate::utils::logging;
use crate::utils::{paths, report};

#[path = "status/scorecard.rs"]
mod scorecard;

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct TierCheck {
    pub id: String,
    pub ok: bool,
    pub required: bool,
    pub detail: String,
    pub path: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct TierSummary {
    pub ok: bool,
    pub score_pct: f64,
    pub required_total: usize,
    pub required_passed: usize,
    pub required_remaining: usize,
    pub checks: Vec<TierCheck>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct TrendTier {
    pub prev_score_pct: Option<f64>,
    pub delta_score_pct: f64,
    pub regression: bool,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Trend {
    pub overall_regression: bool,
    pub tiers: HashMap<String, TrendTier>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PTierStatusRepo {
    pub generated_utc: String,
    pub overall_ok: bool,
    pub overall_completion_pct: f64,
    pub required_total: usize,
    pub required_passed: usize,
    pub required_remaining: usize,
    pub blockers: Vec<String>,
    pub trend: Trend,
    pub tiers: HashMap<String, TierSummary>,
}

pub fn run() -> Result<()> {
    logging::info(
        "release::status",
        "Generating aggregated readiness status",
        &[("tiers", "baseline/expansion/maturity")],
    );

    let root = paths::repo_root();
    let out_json = root.join(config::repo_paths::P_TIER_STATUS_JSON);
    let out_md = root.join(config::repo_paths::P_TIER_STATUS_MD);
    paths::ensure_dir(out_json.parent().unwrap())?;

    // Baseline checks
    let p0_checks = vec![
            bool_check(
                &root,
                "health_score",
                vec!["reports/tooling/health_report.json"],
                true,
                |d| {
                    let score = d.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    (score >= 60.0, format!("score={score}"))
                },
                "missing health_report",
            ),
            bool_check(
                &root,
                "policy_gate",
                vec!["reports/tooling/policy_gate.json"],
                true,
                |d| {
                    (
                        d.get("ok").and_then(|v| v.as_bool()).unwrap_or(false),
                        "policy_gate.ok".to_string(),
                    )
                },
                "missing policy gate",
            ),
            bool_check(
                &root,
                "syscall_default",
                vec![config::repo_paths::SYSCALL_COVERAGE_SUMMARY],
                true,
                |d| {
                    let pct = d
                        .get("implemented_pct")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    (pct >= 100.0, format!("implemented_pct={pct}"))
                },
                "missing syscall coverage summary",
            ),
            bool_check(
                &root,
                "syscall_linux_compat",
                vec!["reports/syscall_coverage_linux_compat_summary.json"],
                true,
                |d| {
                    let pct = d
                        .get("implemented_pct")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    (pct >= 100.0, format!("implemented_pct={pct}"))
                },
                "missing linux_compat syscall summary",
            ),
        ];

        // Expansion checks
        let p1_checks = vec![
            bool_check(
                &root,
                "posix_conformance",
                vec![config::repo_paths::POSIX_CONFORMANCE_SUMMARY],
                true,
                |d| {
                    let ok = d
                        .get("summary")
                        .and_then(|v| v.get("ok"))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    (ok, "summary.ok".to_string())
                },
                "missing posix conformance summary",
            ),
            bool_check(
                &root,
                "soak_stress_chaos",
                vec!["reports/soak_stress_chaos.json"],
                true,
                |d| {
                    let ok = d
                        .get("summary")
                        .and_then(|v| v.get("ok"))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    (ok, "summary.ok".to_string())
                },
                "missing soak/stress summary",
            ),
        ];

        // Maturity checks
        let p2_checks = vec![bool_check(
            &root,
            "p2_gap_gate",
            vec!["reports/p2_gap/gate_summary.json"],
            true,
            |d| {
                let ok = d
                    .get("summary")
                    .and_then(|v| v.get("ok"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                (ok, "summary.ok".to_string())
            },
            "missing p2 gap gate summary",
        )];

        let mut tiers = HashMap::new();
        tiers.insert("p0".to_string(), summarize_tier(p0_checks));
        tiers.insert("p1".to_string(), summarize_tier(p1_checks));
        tiers.insert("p2".to_string(), summarize_tier(p2_checks));

        let overall_ok = tiers.values().all(|t| t.ok);
        let mut blockers = Vec::new();
        for (name, tier) in &tiers {
            for c in &tier.checks {
                if c.required && !c.ok {
                    blockers.push(format!("{name}:{}: {}", c.id, c.detail));
                }
            }
        }

        let required_total: usize = tiers.values().map(|t| t.required_total).sum();
        let required_passed: usize = tiers.values().map(|t| t.required_passed).sum();
        let overall_completion_pct = if required_total > 0 {
            (required_passed as f64 / required_total as f64) * 100.0
        } else {
            100.0
        };

        // Previous status for trend
        let prev_status: Option<PTierStatusRepo> = fs::read_to_string(&out_json)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok());

        let trend = build_trend(&tiers, prev_status);

        let report = PTierStatusRepo {
            generated_utc: report::utc_now_iso(),
            overall_ok,
            overall_completion_pct: (overall_completion_pct * 10.0).round() / 10.0,
            required_total,
            required_passed,
            required_remaining: required_total.saturating_sub(required_passed),
            blockers: blockers.clone(),
            trend,
            tiers: tiers.clone(),
        };

        report::write_json_report(&out_json, &report)?;

        // Generator markdown
        let mut md = format!(
            "# Baseline/Expansion/Maturity Status\n\n- generated_utc: `{}`\n- overall_ok: `{}`\n- overall_completion_pct: {:.1}%\n- required_passed: {}/{}\n- blockers: {}\n\n",
            report.generated_utc,
            overall_ok,
            report.overall_completion_pct,
            report.required_passed,
            report.required_total,
            blockers.len()
        );

        for tier_name in &["p0", "p1", "p2"] {
            if let Some(t) = tiers.get(*tier_name) {
                md.push_str(&format!(
                    "## {} - {}\n",
                    tier_name.to_uppercase(),
                    if t.ok { "OK" } else { "FAIL" }
                ));
                md.push_str(&format!("- score_pct: {:.1}%\n", t.score_pct));
                md.push_str(&format!(
                    "- required_passed: {}/{}\n\n",
                    t.required_passed, t.required_total
                ));
                for c in &t.checks {
                    md.push_str(&format!(
                        "- [{}] `{}` ({}) - {}\n",
                        if c.ok { "x" } else { " " },
                        c.id,
                        if c.required { "required" } else { "optional" },
                        c.detail
                    ));
                }
                md.push('\n');
            }
        }

        fs::write(&out_md, md)?;

        scorecard::write_production_acceptance_scorecard(&root)?;

        logging::ready(
            "release::status",
            "Readiness status generated",
            &[
                ("completion", &format!("{:.1}%", overall_completion_pct)),
                ("blockers", &blockers.len().to_string()),
            ],
        );
        Ok(())
    }

fn bool_check<F>(
    root: &Path,
    id: &str,
    patterns: Vec<&str>,
    required: bool,
    predicate: F,
    missing_detail: &str,
) -> TierCheck
where
    F: Fn(&serde_json::Value) -> (bool, String),
{
    for pattern in patterns {
        let path = root.join(pattern);
        if path.exists() {
            if let Ok(text) = fs::read_to_string(&path) {
                if let Ok(json) = serde_json::from_str(&text) {
                    let (ok, detail) = predicate(&json);
                    return TierCheck {
                        id: id.to_string(),
                        ok,
                        required,
                        detail,
                        path: pattern.to_string(),
                    };
                }
            }
        }
    }
    TierCheck {
        id: id.to_string(),
        ok: !required,
        required,
        detail: missing_detail.to_string(),
        path: String::new(),
    }
}

fn summarize_tier(checks: Vec<TierCheck>) -> TierSummary {
    let req_total = checks.iter().filter(|c| c.required).count();
    let req_passed = checks.iter().filter(|c| c.required && c.ok).count();
    let score = if req_total > 0 {
        (req_passed as f64 / req_total as f64) * 100.0
    } else {
        100.0
    };
    TierSummary {
        ok: req_passed == req_total,
        score_pct: (score * 10.0).round() / 10.0,
        required_total: req_total,
        required_passed: req_passed,
        required_remaining: req_total.saturating_sub(req_passed),
        checks,
    }
}

fn build_trend(cur: &HashMap<String, TierSummary>, prev: Option<PTierStatusRepo>) -> Trend {
    let mut trend = Trend::default();
    for name in &["p0".to_string(), "p1".to_string(), "p2".to_string()] {
        let cur_tier = cur.get(name).unwrap();
        let mut t = TrendTier::default();
        if let Some(p) = &prev {
            if let Some(pt) = p.tiers.get(name) {
                t.prev_score_pct = Some(pt.score_pct);
                t.delta_score_pct = (cur_tier.score_pct - pt.score_pct).round();
                t.regression = t.delta_score_pct < 0.0;
                if t.regression {
                    trend.overall_regression = true;
                }
            }
        }
        trend.tiers.insert(name.clone(), t);
    }
    trend
}
