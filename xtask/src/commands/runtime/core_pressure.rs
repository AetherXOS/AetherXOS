use anyhow::{Result, bail};
use serde::Serialize;


// ---------------------------------------------------------------------------
// Core Pressure Snapshot decoder
// Replaces: scripts/core_pressure_report.py
// ---------------------------------------------------------------------------

const CORE_PRESSURE_WORDS: usize = 18;

const CORE_CLASS: &[(u64, &str)] = &[
    (0, "Nominal"), (1, "Elevated"), (2, "High"), (3, "Critical"),
];

fn class_label(raw: u64) -> &'static str {
    CORE_CLASS.iter().find(|(v, _)| *v == raw).map(|(_, l)| *l).unwrap_or("Unknown")
}

#[derive(Serialize)]
struct CorePressureSnapshot {
    schema_version: u64,
    online_cpus: u64,
    runqueue_total: u64,
    runqueue_max: u64,
    runqueue_avg_milli: u64,
    rt_starvation_alert: bool,
    rt_forced_reschedules: u64,
    watchdog_stall_detections: u64,
    net_queue_limit: u64,
    net_rx_depth: u64,
    net_tx_depth: u64,
    net_saturation_percent: u64,
    lb_imbalance_p50: u64,
    lb_imbalance_p90: u64,
    lb_imbalance_p99: u64,
    lb_prefer_local_forced_moves: u64,
    core_pressure_class_raw: u64,
    scheduler_pressure_class_raw: u64,
    core_pressure_class: String,
    scheduler_pressure_class: String,
}

#[derive(Serialize)]
struct LotteryReplay {
    seq: u64,
    task_id: u64,
    winner_ticket: u64,
    total_tickets: u64,
    rng_state: String,
}

/// Decode and report core pressure snapshot from raw syscall words.
///
/// Usage: cargo xtask core-pressure --words "2,8,10,4,1250,0,12,0,1024,40,20,3,2,5,8,0,1,1"
pub fn execute(words_str: &str, lottery_words_str: &Option<String>, format: &str, out: &Option<String>) -> Result<()> {
    println!("[core-pressure] Decoding core pressure snapshot");

    let words: Vec<u64> = words_str.split(',')
        .map(|s| {
            let s = s.trim();
            if s.starts_with("0x") || s.starts_with("0X") {
                u64::from_str_radix(&s[2..], 16).unwrap_or(0)
            } else {
                s.parse::<u64>().unwrap_or(0)
            }
        })
        .collect();

    if words.len() < CORE_PRESSURE_WORDS {
        bail!("GET_CORE_PRESSURE_SNAPSHOT requires at least {} words, got {}", CORE_PRESSURE_WORDS, words.len());
    }

    let snapshot = CorePressureSnapshot {
        schema_version: words[0],
        online_cpus: words[1],
        runqueue_total: words[2],
        runqueue_max: words[3],
        runqueue_avg_milli: words[4],
        rt_starvation_alert: words[5] != 0,
        rt_forced_reschedules: words[6],
        watchdog_stall_detections: words[7],
        net_queue_limit: words[8],
        net_rx_depth: words[9],
        net_tx_depth: words[10],
        net_saturation_percent: words[11],
        lb_imbalance_p50: words[12],
        lb_imbalance_p90: words[13],
        lb_imbalance_p99: words[14],
        lb_prefer_local_forced_moves: words[15],
        core_pressure_class_raw: words[16],
        scheduler_pressure_class_raw: words[17],
        core_pressure_class: class_label(words[16]).to_string(),
        scheduler_pressure_class: class_label(words[17]).to_string(),
    };

    let replay = lottery_words_str.as_ref().map(|s| {
        let w: Vec<u64> = s.split(',').map(|t| {
            let t = t.trim();
            if t.starts_with("0x") || t.starts_with("0X") {
                u64::from_str_radix(&t[2..], 16).unwrap_or(0)
            } else {
                t.parse::<u64>().unwrap_or(0)
            }
        }).collect();
        LotteryReplay {
            seq: *w.first().unwrap_or(&0),
            task_id: *w.get(1).unwrap_or(&0),
            winner_ticket: *w.get(2).unwrap_or(&0),
            total_tickets: *w.get(3).unwrap_or(&0),
            rng_state: format!("0x{:x}", w.get(4).unwrap_or(&0)),
        }
    });

    let rendered = if format == "json" {
        let payload = serde_json::json!({
            "core_pressure_snapshot": snapshot,
            "lottery_replay_latest": replay,
        });
        serde_json::to_string_pretty(&payload)?
    } else {
        let mut md = String::from("# Core Pressure Snapshot Report\n\n## Core Pressure\n\n");
        md.push_str(&format!("- schema_version: `{}`\n", snapshot.schema_version));
        md.push_str(&format!("- online_cpus: `{}`\n", snapshot.online_cpus));
        md.push_str(&format!("- runqueue_total: `{}`\n", snapshot.runqueue_total));
        md.push_str(&format!("- runqueue_max: `{}`\n", snapshot.runqueue_max));
        md.push_str(&format!("- core_pressure_class: `{}`\n", snapshot.core_pressure_class));
        md.push_str(&format!("- scheduler_pressure_class: `{}`\n", snapshot.scheduler_pressure_class));
        if let Some(ref r) = replay {
            md.push_str("\n## Lottery Replay\n\n");
            md.push_str(&format!("- seq: `{}`\n", r.seq));
            md.push_str(&format!("- task_id: `{}`\n", r.task_id));
            md.push_str(&format!("- rng_state: `{}`\n", r.rng_state));
        }
        md
    };

    if let Some(out_path) = out {
        let p = crate::utils::paths::resolve(out_path);
        crate::utils::paths::ensure_dir(p.parent().unwrap())?;
        std::fs::write(&p, &rendered)?;
        println!("[core-pressure] Report written: {}", p.display());
    } else {
        println!("{}", rendered);
    }

    Ok(())
}
