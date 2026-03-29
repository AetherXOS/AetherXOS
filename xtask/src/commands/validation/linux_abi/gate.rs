use anyhow::Result;
use std::fs;

use crate::utils::{paths, report};
use crate::config;
use crate::commands::validation::linux_abi::{gap, readiness, errno};

pub fn run() -> Result<()> {
    println!("[linux-abi::gate] Running Linux ABI quality gate");

    // All steps must pass to continue
    println!("[linux-abi::gate] Step 1/5: Full gap inventory");
    gap::run()?;

    println!("[linux-abi::gate] Step 2/5: Static errno conformance");
    errno::run_conformance()?;

    println!("[linux-abi::gate] Step 3/5: Shim errno conformance");
    errno::run_shim_conformance()?;

    println!("[linux-abi::gate] Step 4/5: Readiness score calculation");
    readiness::run()?;

    println!("[linux-abi::gate] Step 5/5: Evaluating gate thresholds");

    let root = paths::repo_root();
    let out_dir = paths::resolve("reports/linux_abi_gate");
    paths::ensure_dir(&out_dir)?;

    let gap_data: serde_json::Value = {
        let p = root.join(config::repo_paths::ABI_GAP_SUMMARY);
        if p.exists() {
            serde_json::from_str(&fs::read_to_string(&p)?)?
        } else {
            serde_json::json!({})
        }
    };

    let total_gaps = gap_data.pointer("/summary/total_gaps")
        .and_then(|v| v.as_u64()).unwrap_or(0);

    let readiness_data: serde_json::Value = {
        let p = root.join(config::repo_paths::ABI_READINESS_SUMMARY);
        if p.exists() {
            serde_json::from_str(&fs::read_to_string(&p)?)?
        } else {
            serde_json::json!({})
        }
    };

    let score = readiness_data.pointer("/summary/score")
        .and_then(|v| v.as_f64()).unwrap_or(0.0);

    // Dynamic thresholds could be added here
    let ok = score >= 5.0; // Minimal threshold

    let summary = serde_json::json!({
        "summary": {
            "ok": ok,
            "metrics": { "total_gaps": total_gaps, "readiness_score": score },
        }
    });

    report::write_json_report(&out_dir.join("summary.json"), &summary)?;
    println!("[linux-abi::gate] {} (gaps={}, readiness={:.1})", if ok { "PASS" } else { "FAIL" }, total_gaps, score);
    
    Ok(())
}
