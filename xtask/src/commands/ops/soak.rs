use anyhow::Result;
use serde::Serialize;

use crate::utils::{paths, report};

/// Run QEMU soak/stress testing matrix.
///
/// Replaces: scripts/qemu_soak_matrix.py, scripts/soak_stress_chaos.py
pub fn execute(dry_run: bool) -> Result<()> {
    println!("[soak] Running native QEMU soak matrix (dry_run={})", dry_run);

    let out_dir = paths::resolve("artifacts/qemu_soak");
    let summary_path = out_dir.join("summary.json");
    paths::ensure_dir(&out_dir)?;

    if dry_run {
        let summary = SoakSummary {
            generated_utc: report::utc_now_iso(),
            ok: true,
            dry_run,
            total_rounds: 0,
            passed_rounds: 0,
            failed_rounds: 0,
            rounds: Vec::new(),
        };
        report::write_json_report(&summary_path, &summary)?;
        println!("[soak] DRY-RUN summary={}", summary_path.display());
        return Ok(());
    }

    // Keep the first native version deterministic and lightweight.
    let mut rounds = Vec::new();
    for round in 1..=6 {
        let start = std::time::Instant::now();
        let result = crate::commands::ops::qemu::smoke_test();
        let duration_sec = start.elapsed().as_secs_f64();
        match result {
            Ok(()) => rounds.push(SoakRound {
                round,
                ok: true,
                duration_sec,
                error: None,
            }),
            Err(err) => rounds.push(SoakRound {
                round,
                ok: false,
                duration_sec,
                error: Some(err.to_string()),
            }),
        }
    }

    let passed_rounds = rounds.iter().filter(|r| r.ok).count();
    let failed_rounds = rounds.len().saturating_sub(passed_rounds);
    let summary = SoakSummary {
        generated_utc: report::utc_now_iso(),
        ok: failed_rounds == 0,
        dry_run,
        total_rounds: rounds.len(),
        passed_rounds,
        failed_rounds,
        rounds,
    };

    report::write_json_report(&summary_path, &summary)?;
    println!(
        "[soak] rounds={} pass={} fail={} summary={}",
        summary.total_rounds,
        summary.passed_rounds,
        summary.failed_rounds,
        summary_path.display()
    );

    if summary.ok {
        Ok(())
    } else {
        anyhow::bail!("native soak matrix failed")
    }
}

#[derive(Serialize)]
struct SoakSummary {
    generated_utc: String,
    ok: bool,
    dry_run: bool,
    total_rounds: usize,
    passed_rounds: usize,
    failed_rounds: usize,
    rounds: Vec<SoakRound>,
}

#[derive(Serialize)]
struct SoakRound {
    round: usize,
    ok: bool,
    duration_sec: f64,
    error: Option<String>,
}
