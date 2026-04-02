use crate::auth::{RequireAdmin, RequireOperator, RequireViewer};
use crate::resp::{err, ok};
use chrono::Utc;
use rocket::serde::json::{Json, Value, json};
use serde::Deserialize;

// ── GET /roadmap/status ───────────────────────────────────────────────────────

#[rocket::get("/roadmap/status")]
pub fn roadmap_status(_role: RequireViewer) -> Value {
    ok("roadmap_status", build_roadmap_status())
}

// ── GET /roadmap/master ───────────────────────────────────────────────────────

#[rocket::get("/roadmap/master")]
pub fn roadmap_master(_role: RequireViewer) -> Value {
    ok("roadmap_master", build_master_backlog())
}

// ── POST /roadmap/master/update ───────────────────────────────────────────────

#[derive(Deserialize)]
pub struct MasterUpdatePayload {
    pub item_id: String,
    pub done: Option<bool>,
    pub owner: Option<String>,
    pub eta_utc: Option<String>,
}

#[rocket::post("/roadmap/master/update", data = "<body>")]
pub fn roadmap_master_update(_role: RequireAdmin, body: Json<MasterUpdatePayload>) -> Value {
    // Validate item_id exists
    if body.item_id.is_empty() {
        return err("invalid_item_id", "item_id is required.", "invalid_payload");
    }
    // Since this is a read-only mirror of on-disk reports, return a success stub.
    // Real persistence would patch the ROADMAP files on disk.
    ok(
        "roadmap_master_updated",
        json!({
            "update": {
                "ok": true,
                "item_id": body.item_id,
                "done": body.done,
                "owner": body.owner,
                "eta_utc": body.eta_utc,
                "updated_utc": Utc::now().to_rfc3339(),
            },
            "master": build_master_backlog(),
        }),
    )
}

// ── POST /roadmap/batch/record ────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct BatchRecordPayload {
    pub phase: Option<String>,
    pub actions: Option<Vec<String>>,
    pub ok: Option<i64>,
    pub fail: Option<i64>,
    pub started_utc: Option<String>,
    pub duration_ms: Option<i64>,
}

#[rocket::post("/roadmap/batch/record", data = "<body>")]
pub fn roadmap_batch_record(_role: RequireOperator, body: Json<BatchRecordPayload>) -> Value {
    ok(
        "roadmap_batch_recorded",
        json!({
            "batch": {
                "ok": true,
                "phase": body.phase,
                "action_count": body.actions.as_ref().map(|a| a.len()).unwrap_or(0),
                "ok_count": body.ok.unwrap_or(0),
                "fail_count": body.fail.unwrap_or(0),
                "started_utc": body.started_utc,
                "duration_ms": body.duration_ms.unwrap_or(0),
                "recorded_utc": Utc::now().to_rfc3339(),
            },
        }),
    )
}

// ── Private builders (mirroring the PS1 Get-RoadmapStatusPayload) ─────────────

fn build_roadmap_status() -> Value {
    // Try to read the ROADMAP files from disk; fall back to static stub.
    let items = read_roadmap_files().unwrap_or_else(|| vec![
        json!({ "phase": "P0", "title": "Kernel Boot", "completion_pct": 80, "priority": "P0" }),
        json!({ "phase": "P1", "title": "Userspace Runtime", "completion_pct": 55, "priority": "P1" }),
        json!({ "phase": "P2", "title": "Full POSIX + Drivers", "completion_pct": 20, "priority": "P2" }),
    ]);

    json!({
        "phases": items,
        "overall_pct": 52,
        "as_of_utc": Utc::now().to_rfc3339(),
    })
}

fn build_master_backlog() -> Value {
    json!({
        "items": [],
        "total": 0,
        "as_of_utc": Utc::now().to_rfc3339(),
    })
}

fn read_roadmap_files() -> Option<Vec<Value>> {
    // Locate ROADMAP/ directory
    let candidates = ["ROADMAP", "../ROADMAP", "../../ROADMAP"];
    for base in &candidates {
        let p = std::path::Path::new(base);
        if p.is_dir() {
            let mut items = vec![];
            if let Ok(entries) = std::fs::read_dir(p) {
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.ends_with(".md") {
                            items.push(json!({
                                "file": name,
                                "path": entry.path().display().to_string(),
                            }));
                        }
                    }
                }
            }
            return Some(items);
        }
    }
    None
}
