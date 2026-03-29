use anyhow::Result;
use std::fs;

use crate::utils::{paths, report};
use crate::config;

pub fn run() -> Result<()> {
    println!("[linux-abi::readiness] Computing Linux ABI readiness score");

    let root = paths::repo_root();
    let out_dir = paths::resolve("reports/abi_readiness");
    paths::ensure_dir(&out_dir)?;

    let load_json = |rel_path: &str| -> Result<serde_json::Value> {
        let p = root.join(rel_path);
        if !p.exists() {
            return Ok(serde_json::json!({}));
        }
        let text = fs::read_to_string(&p)?;
        Ok(serde_json::from_str(&text)?)
    };

    let errno = load_json(config::repo_paths::ERRNO_CONFORMANCE_SUMMARY)?;
    let shim = load_json(config::repo_paths::SHIM_ERRNO_SUMMARY)?;
    let gap = load_json(config::repo_paths::ABI_GAP_SUMMARY)?;
    let cov = load_json(config::repo_paths::SYSCALL_COVERAGE_SUMMARY)?;

    let get_val = |v: &serde_json::Value, ptr: &str| -> f64 {
        v.pointer(ptr).and_then(|val| val.as_f64().or_else(|| val.as_u64().map(|u| u as f64))).unwrap_or(0.0)
    };

    let errno_checks = get_val(&errno, "/summary/checks");
    let errno_passed = get_val(&errno, "/summary/passed");
    let errno_pass_ratio = if errno_checks > 0.0 { (errno_passed / errno_checks).clamp(0.0, 1.0) } else { 0.0 };

    let shim_checks = get_val(&shim, "/summary/checks");
    let shim_passed = get_val(&shim, "/summary/passed");
    let shim_pass_ratio = if shim_checks > 0.0 { (shim_passed / shim_checks).clamp(0.0, 1.0) } else { 0.0 };

    let total_gaps = get_val(&gap, "/summary/total_gaps");
    let stub_count = get_val(&gap, "/summary/stub_count");
    let partial_count = get_val(&gap, "/summary/partial_or_feature_gated_count");
    
    let gap_denom = if total_gaps > 0.0 { total_gaps } else { 1.0 };
    let gap_penalty = ((stub_count + partial_count * 0.5) / gap_denom).clamp(0.0, 1.0);
    let gap_score = 1.0 - gap_penalty;

    let implemented_pct = get_val(&cov, "/implemented_pct");
    let coverage_score = (implemented_pct / 100.0).clamp(0.0, 1.0);

    // Composite Score Weights:
    // 35% static errno | 25% shim errno | 20% gap analysis | 20% implementation coverage
    let score = ((0.35 * errno_pass_ratio + 0.25 * shim_pass_ratio + 0.20 * gap_score + 0.20 * coverage_score) * 10000.0).round() / 100.0;

    let payload = serde_json::json!({
        "summary": {
            "score": score,
            "components": {
                "errno_pass_ratio": (errno_pass_ratio * 10000.0).round() / 10000.0,
                "linux_shim_pass_ratio": (shim_pass_ratio * 10000.0).round() / 10000.0,
                "abi_gap_score": (gap_score * 10000.0).round() / 10000.0,
                "syscall_coverage_score": (coverage_score * 10000.0).round() / 10000.0,
            }
        }
    });

    report::write_json_report(&out_dir.join("summary.json"), &payload)?;

    let md = format!(
        "# Linux ABI Readiness Score\n\n- score: {score}\n- errno_pass_ratio: {errno_pass_ratio:.4}\n- linux_shim_pass_ratio: {shim_pass_ratio:.4}\n- abi_gap_score: {gap_score:.4}\n- syscall_coverage_score: {coverage_score:.4}\n",
    );
    fs::write(out_dir.join("summary.md"), md)?;

    println!("[linux-abi::readiness] Score calculated from {} components: {}", 4, score);
    Ok(())
}
