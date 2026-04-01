use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

use crate::cli::AbSlotAction;
use crate::utils::paths;
use crate::utils::report;

/// Entry point for `cargo run -p xtask -- ab-slot <action>`.
///
/// Replaces: ab_boot_slots.py, ab_nightly_slot_flip.py, ab_boot_recovery_gate.py
pub fn execute(action: &AbSlotAction) -> Result<()> {
    match action {
        AbSlotAction::Init => init(),
        AbSlotAction::Stage { slot } => stage(slot),
        AbSlotAction::NightlyFlip => nightly_flip(),
        AbSlotAction::RecoveryGate => recovery_gate(),
    }
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone)]
struct SlotState {
    active_slot: String,
    last_known_good_slot: String,
    previous_slot: Option<String>,
    pending_slot: Option<String>,
    status: String,
    policy: SlotPolicy,
    slots: std::collections::HashMap<String, SlotMeta>,
    history: Vec<HistoryEntry>,
}

#[derive(Serialize, Deserialize, Clone)]
struct SlotPolicy {
    max_consecutive_failures: u32,
}

#[derive(Serialize, Deserialize, Clone)]
struct SlotMeta {
    generation: u32,
    version: Option<String>,
    updated_at_utc: Option<String>,
    artifacts: std::collections::HashMap<String, String>,
    boot_failures: u32,
    boot_successes: u32,
}

#[derive(Serialize, Deserialize, Clone)]
struct HistoryEntry {
    ts_utc: String,
    event: String,
    details: serde_json::Value,
}

fn default_state() -> SlotState {
    let mut slots = std::collections::HashMap::new();
    for name in &["A", "B"] {
        slots.insert(name.to_string(), SlotMeta {
            generation: 0,
            version: None,
            updated_at_utc: None,
            artifacts: std::collections::HashMap::new(),
            boot_failures: 0,
            boot_successes: 0,
        });
    }

    SlotState {
        active_slot: "A".to_string(),
        last_known_good_slot: "A".to_string(),
        previous_slot: None,
        pending_slot: None,
        status: "healthy".to_string(),
        policy: SlotPolicy { max_consecutive_failures: 3 },
        slots,
        history: Vec::new(),
    }
}

fn state_path() -> std::path::PathBuf {
    paths::resolve("artifacts/boot_ab/state.json")
}

fn load_state() -> Result<SlotState> {
    let p = state_path();
    if !p.exists() {
        return Ok(default_state());
    }
    let text = fs::read_to_string(&p)?;
    Ok(serde_json::from_str(&text)?)
}

fn save_state(state: &SlotState) -> Result<()> {
    let p = state_path();
    paths::ensure_dir(p.parent().unwrap())?;
    let json = serde_json::to_string_pretty(state)?;
    fs::write(&p, json)?;
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String> {
    let data = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    Ok(format!("{:x}", hasher.finalize()))
}

fn append_history(state: &mut SlotState, event: &str, details: serde_json::Value) {
    state.history.push(HistoryEntry {
        ts_utc: report::utc_now_iso(),
        event: event.to_string(),
        details,
    });
    if state.history.len() > 200 {
        let drain_count = state.history.len() - 200;
        state.history.drain(..drain_count);
    }
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

fn init() -> Result<()> {
    println!("[ab-slot::init] Initializing A/B boot slot metadata");
    let state = default_state();
    save_state(&state)?;
    println!("[ab-slot::init] State written: {}", state_path().display());
    Ok(())
}

fn stage(slot: &str) -> Result<()> {
    println!("[ab-slot::stage] Staging artifacts to slot {}", slot);

    let mut state = load_state()?;
    let ab_root = paths::resolve("artifacts/boot_ab");
    let slot_boot = ab_root.join("slots").join(slot).join("boot");
    paths::ensure_dir(&slot_boot)?;

    // Copy current build artifacts to slot
    let kernel_src = paths::resolve("artifacts/boot_image/stage/boot/hypercore.elf");
    let initramfs_src = paths::resolve("artifacts/boot_image/stage/boot/initramfs.cpio.gz");
    let limine_src = paths::resolve("artifacts/boot_image/stage/boot/limine.conf");

    if !kernel_src.exists() { bail!("Kernel not found: {}. Run `cargo run -p xtask -- build full` first", kernel_src.display()); }
    if !initramfs_src.exists() { bail!("Initramfs not found: {}", initramfs_src.display()); }

    fs::copy(&kernel_src, slot_boot.join("hypercore.elf"))?;
    if initramfs_src.exists() { fs::copy(&initramfs_src, slot_boot.join("initramfs.cpio.gz"))?; }
    if limine_src.exists() { fs::copy(&limine_src, slot_boot.join("limine.conf"))?; }

    // Update state
    let (generation, _version) = {
        let meta = state.slots.entry(slot.to_string()).or_insert_with(|| SlotMeta {
            generation: 0, version: None, updated_at_utc: None,
            artifacts: std::collections::HashMap::new(), boot_failures: 0, boot_successes: 0,
        });
        meta.generation += 1;
        meta.updated_at_utc = Some(report::utc_now_iso());
        meta.artifacts.insert("kernel_sha256".into(), sha256_file(&slot_boot.join("hypercore.elf"))?);
        (meta.generation, meta.version.clone())
    };

    append_history(&mut state, "stage", serde_json::json!({
        "slot": slot, "generation": generation,
    }));
    save_state(&state)?;

    println!("[ab-slot::stage] Slot {} staged (generation {})", slot, generation);
    Ok(())
}

fn nightly_flip() -> Result<()> {
    println!("[ab-slot::nightly-flip] Running nightly A/B slot flip");

    let mut state = load_state()?;
    let current = state.active_slot.clone();
    let next = if current == "A" { "B" } else { "A" };

    state.previous_slot = Some(current.clone());
    state.active_slot = next.to_string();
    state.pending_slot = Some(next.to_string());
    state.status = "pending_validation".to_string();

    append_history(&mut state, "nightly_flip", serde_json::json!({
        "from": current, "to": next,
    }));
    save_state(&state)?;

    println!("[ab-slot::nightly-flip] Flipped: {} -> {}", current, next);
    Ok(())
}

fn recovery_gate() -> Result<()> {
    println!("[ab-slot::recovery] Running boot recovery gate");

    let root = paths::repo_root();
    let soak_path = root.join("artifacts/qemu_soak/summary.json");
    let out_dir = paths::resolve("reports/ab_boot_recovery_gate");
    paths::ensure_dir(&out_dir)?;

    if !soak_path.exists() {
        let summary = serde_json::json!({ "ok": false, "reason": "soak summary not found" });
        report::write_json_report(&out_dir.join("summary.json"), &summary)?;
        println!("[ab-slot::recovery] FAIL (no soak summary)");
        return Ok(());
    }

    let soak: serde_json::Value = serde_json::from_str(&fs::read_to_string(&soak_path)?)?;
    let rounds = soak.get("rounds").and_then(|v| v.as_array());

    let (successful, chaos) = match rounds {
        Some(r) => {
            let s = r.iter().filter(|v| {
                v.get("expected_success").and_then(|v| v.as_bool()).unwrap_or(false)
                && v.get("ok").and_then(|v| v.as_bool()).unwrap_or(false)
            }).count();
            let c = r.iter().filter(|v| {
                !v.get("expected_success").and_then(|v| v.as_bool()).unwrap_or(true)
            }).count();
            (s, c)
        }
        None => (0, 0),
    };

    let ok = successful >= 3;
    let summary = serde_json::json!({
        "ok": ok,
        "successful_rounds": successful,
        "chaos_rounds": chaos,
    });
    report::write_json_report(&out_dir.join("summary.json"), &summary)?;

    let md = format!(
        "# A/B Boot Recovery Gate\n\n- ok: `{}`\n- successful_rounds: `{}`\n- chaos_rounds: `{}`\n",
        ok, successful, chaos
    );
    fs::write(out_dir.join("summary.md"), md)?;

    println!("[ab-slot::recovery] {}", if ok { "PASS" } else { "FAIL" });
    Ok(())
}
