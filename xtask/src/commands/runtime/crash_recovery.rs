use anyhow::Result;
use serde::Serialize;
use std::fs;

use crate::constants;
use crate::utils::paths;
use crate::utils::report;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct CrashReport {
    ok: bool,
    logs_processed: usize,
    panic_counts: Vec<usize>,
    latest_seqs: Vec<usize>,
    total_events: Vec<usize>,
    checks: CrashChecks,
}

#[derive(Serialize)]
struct CrashChecks {
    panic_count_monotonic: bool,
    latest_seq_monotonic: bool,
}

const PANIC_MARKERS: &[&str] = &[
    "PANIC report:",
    "[KERNEL DUMP] panic_count=",
    "kernel panic",
];

/// Run crash-dump recovery pipeline over captured kernel logs.
///
/// Replaces: scripts/crash_recovery_pipeline.py + scripts/crash_artifacts_report.py
pub fn execute() -> Result<()> {
    println!("[crash-recovery] Running crash artifact pipeline");

    let logs_dir = constants::paths::crash_logs_dir();
    let out_dir = constants::paths::crash_reports_dir();
    paths::ensure_dir(&out_dir)?;

    if !logs_dir.exists() {
        let summary = CrashReport {
            ok: false, logs_processed: 0, panic_counts: vec![], latest_seqs: vec![],
            total_events: vec![], checks: CrashChecks { panic_count_monotonic: true, latest_seq_monotonic: true },
        };
        report::write_json_report(&out_dir.join("summary.json"), &summary)?;
        println!("[crash-recovery] FAIL (no logs directory)");
        return Ok(());
    }

    let mut logs: Vec<_> = fs::read_dir(&logs_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "log").unwrap_or(false))
        .map(|e| e.path())
        .collect();
    logs.sort();

    if logs.is_empty() {
        let summary = CrashReport {
            ok: false, logs_processed: 0, panic_counts: vec![], latest_seqs: vec![],
            total_events: vec![], checks: CrashChecks { panic_count_monotonic: true, latest_seq_monotonic: true },
        };
        report::write_json_report(&out_dir.join("summary.json"), &summary)?;
        println!("[crash-recovery] FAIL (no .log files)");
        return Ok(());
    }

    let mut panic_counts = Vec::new();
    let mut latest_seqs = Vec::new();
    let mut total_events = Vec::new();
    let seq_re = regex::Regex::new(r"\[SEQ=(\d+)\]").expect("valid sequence regex");

    for log_path in &logs {
        let text = fs::read_to_string(log_path).unwrap_or_default();
        let lines: Vec<&str> = text.lines().collect();

        // Count panic markers
        let panic_count = lines.iter()
            .filter(|line| PANIC_MARKERS.iter().any(|m| line.contains(m)))
            .count();
        panic_counts.push(panic_count);

        // Extract latest seq from "[SEQ=N]" patterns
        let max_seq = lines.iter()
            .filter_map(|line| seq_re.captures(line))
            .filter_map(|cap| cap[1].parse::<usize>().ok())
            .max()
            .unwrap_or(0);
        latest_seqs.push(max_seq);

        total_events.push(lines.len());

        // Write per-log report
        let per_log = serde_json::json!({
            "file": log_path.to_string_lossy(),
            "panic_count": panic_count,
            "latest_seq": max_seq,
            "event_count": lines.len(),
        });
        let stem = log_path.file_stem().unwrap().to_string_lossy();
        report::write_json_report(&out_dir.join(format!("{}.json", stem)), &per_log)?;
    }

    // Check monotonicity
    let panic_monotonic = panic_counts.windows(2).all(|w| w[0] <= w[1]);
    let seq_monotonic = latest_seqs.windows(2).all(|w| w[0] <= w[1]);

    let summary = CrashReport {
        ok: panic_monotonic && seq_monotonic,
        logs_processed: logs.len(),
        panic_counts,
        latest_seqs,
        total_events,
        checks: CrashChecks {
            panic_count_monotonic: panic_monotonic,
            latest_seq_monotonic: seq_monotonic,
        },
    };

    report::write_json_report(&out_dir.join("summary.json"), &summary)?;

    // Write markdown
    let mut md = String::from("# Crash Recovery Pipeline\n\n");
    md.push_str(&format!("- ok: `{}`\n", summary.ok));
    md.push_str(&format!("- logs_processed: `{}`\n", summary.logs_processed));
    md.push_str(&format!("- panic_count_monotonic: `{}`\n", panic_monotonic));
    md.push_str(&format!("- latest_seq_monotonic: `{}`\n\n", seq_monotonic));
    md.push_str("## Logs\n\n");
    for p in &logs {
        md.push_str(&format!("- `{}`\n", p.display()));
    }
    fs::write(out_dir.join("summary.md"), md)?;

    println!("[crash-recovery] {} ({} logs processed)", if summary.ok { "PASS" } else { "FAIL" }, summary.logs_processed);
    Ok(())
}
