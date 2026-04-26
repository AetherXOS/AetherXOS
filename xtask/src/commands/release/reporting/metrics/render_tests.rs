use super::*;
use crate::commands::release::preflight::models::{
    PerfEngineeringReportDoc, ScoreNormalizeDoc, TrendDashboardDoc, TrendPoint,
};

#[test]
fn render_score_normalize_md_exposes_gate_fields() {
    let doc = ScoreNormalizeDoc {
        generated_utc: "2026-04-23T00:00:00Z".to_string(),
        strict: true,
        overall_ok: false,
        host_os: "windows".to_string(),
        host_arch: "x86_64".to_string(),
        raw_completion_pct: 88.8,
        normalized_score: 95.8,
        failed_checks: 2,
    };

    let md = render_score_normalize_md(&doc);

    assert!(md.contains("# Score Normalize"));
    assert!(md.contains("generated_utc: 2026-04-23T00:00:00Z"));
    assert!(md.contains("strict: true"));
    assert!(md.contains("overall_ok: false"));
    assert!(md.contains("normalized_score: 95.8"));
}

#[test]
fn render_trend_dashboard_md_lists_points_in_order() {
    let doc = TrendDashboardDoc {
        generated_utc: "2026-04-23T00:00:00Z".to_string(),
        strict: false,
        points: vec![
            TrendPoint {
                generated_utc: "2026-04-22T00:00:00Z".to_string(),
                overall_ok: false,
                failed_count: 3,
                completion_pct: 76.9,
            },
            TrendPoint {
                generated_utc: "2026-04-23T00:00:00Z".to_string(),
                overall_ok: true,
                failed_count: 0,
                completion_pct: 100.0,
            },
        ],
        latest_overall_ok: true,
        latest_failed_count: 0,
        regression_detected: false,
    };

    let md = render_trend_dashboard_md(&doc);

    assert!(md.contains("# Trend Dashboard"));
    assert!(md.contains("latest_overall_ok: true"));
    assert!(md.contains("2026-04-22T00:00:00Z :: overall_ok=false failed_count=3 completion_pct=76.9"));
    assert!(md.contains("2026-04-23T00:00:00Z :: overall_ok=true failed_count=0 completion_pct=100.0"));
}

#[test]
fn render_perf_report_md_exposes_threshold_and_waiver_contracts() {
    let doc = PerfEngineeringReportDoc {
        generated_utc: "2026-04-23T00:00:00Z".to_string(),
        strict: true,
        overall_ok: false,
        gate_completion_pct: 91.2,
        normalized_gate_score: 95.0,
        failed_checks: 1,
        release_regression_detected: true,
        linux_abi_score: 88.0,
        perf_engineering_score: 93.0,
        threshold_min_perf_score: 90.0,
        threshold_min_normalized_gate_score: 94.0,
        threshold_max_failed_checks: 1,
        waiver_allow_regression: false,
        waiver_allow_below_min_score: false,
        threshold_source: "config/perf_thresholds.json".to_string(),
        waiver_source: "config/perf_waivers.json".to_string(),
    };

    let md = render_perf_report_md(&doc);

    assert!(md.contains("# Performance Engineering Report"));
    assert!(md.contains("perf_engineering_score: 93.0"));
    assert!(md.contains("threshold_min_perf_score: 90.0"));
    assert!(md.contains("threshold_max_failed_checks: 1"));
    assert!(md.contains("waiver_allow_regression: false"));
    assert!(md.contains("waiver_source: config/perf_waivers.json"));
}
