use chrono::Utc;
use rocket::serde::json::{json, Json, Value};
use rocket::State;
use serde::Deserialize;
use crate::auth::{RequireAdmin, RequireOperator, RequireViewer};
use crate::models::ScheduledTask;
use crate::resp::{err, err_detail, ok};
use crate::state::AppState;
use crate::actions::{dispatch_queue, enqueue_job};

// ── GET /scheduler ────────────────────────────────────────────────────────────

#[rocket::get("/scheduler")]
pub fn get_scheduler(state: &State<AppState>, _role: RequireViewer) -> Value {
    let inner = state.read();
    let tasks: Vec<Value> = inner.schedules.values().map(task_to_json).collect();
    ok("scheduler", json!({
        "enabled": inner.scheduler_enabled,
        "tasks": tasks,
    }))
}

// ── GET /scheduler/templates ──────────────────────────────────────────────────

#[rocket::get("/scheduler/templates")]
pub fn scheduler_templates(_role: RequireViewer) -> Value {
    ok("scheduler_templates", json!({
        "templates": get_templates(),
    }))
}

// ── POST /scheduler/apply_template ───────────────────────────────────────────

#[derive(Deserialize)]
pub struct ApplyTemplatePayload {
    pub template_id: String,
}

#[rocket::post("/scheduler/apply_template", data = "<body>")]
pub fn scheduler_apply_template(
    state: &State<AppState>,
    _role: RequireAdmin,
    body: Json<ApplyTemplatePayload>,
) -> Value {
    let templates = get_templates();
    let tpl = templates.iter().find(|t| t["id"].as_str() == Some(&body.template_id));

    match tpl {
        None => err_detail("not_found", "Template not found.", "not_found", json!({ "template_id": body.template_id })),
        Some(tpl) => {
            let mut inner = state.write();
            inner.schedules.clear();
            inner.scheduler_enabled = tpl["scheduler_enabled"].as_bool().unwrap_or(true);

            let now = Utc::now();
            if let Some(tasks) = tpl["tasks"].as_array() {
                for t in tasks {
                    let id = t["id"].as_str().unwrap_or("").to_string();
                    let action = t["action"].as_str().unwrap_or("").to_string();
                    if id.is_empty() || action.is_empty() { continue; }
                    if inner.action_by_id(&action).is_none() { continue; }
                    let interval = t["interval_sec"].as_u64().unwrap_or(3600).max(60);
                    let priority = t["priority"].as_str().unwrap_or("low").to_string();
                    let enabled = t["enabled"].as_bool().unwrap_or(true);
                    inner.schedules.insert(id.clone(), ScheduledTask {
                        id: id.clone(),
                        action,
                        interval_sec: interval,
                        priority,
                        enabled,
                        last_run_utc: None,
                        next_run_utc: now + chrono::Duration::seconds(interval as i64),
                        source: format!("scheduler:{}", id),
                    });
                }
            }

            let tasks: Vec<Value> = inner.schedules.values().map(task_to_json).collect();
            ok("template_applied", json!({
                "template_id": body.template_id,
                "enabled": inner.scheduler_enabled,
                "tasks": tasks,
            }))
        }
    }
}

// ── POST /scheduler/run_now ───────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RunNowPayload {
    pub id: String,
}

#[rocket::post("/scheduler/run_now", data = "<body>")]
pub fn scheduler_run_now(
    state: &State<AppState>,
    _role: RequireOperator,
    body: Json<RunNowPayload>,
) -> Value {
    let (action, priority, source) = {
        let inner = state.read();
        match inner.schedules.get(&body.id) {
            None => {
                return err_detail("not_found", "Scheduled task not found.", "not_found", json!({ "id": body.id }))
            }
            Some(s) => (s.action.clone(), s.priority.clone(), s.source.clone()),
        }
    };

    match enqueue_job(state, &action, &priority, &source) {
        Some(job_id) => {
            dispatch_queue(state);
            ok("scheduler_run_now", json!({ "id": body.id, "job_id": job_id }))
        }
        None => err("queue_full", "Job queue is full.", "too_many_requests"),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn task_to_json(t: &ScheduledTask) -> Value {
    json!({
        "id": t.id,
        "action": t.action,
        "interval_sec": t.interval_sec,
        "priority": t.priority,
        "enabled": t.enabled,
        "last_run_utc": t.last_run_utc.map(|d| d.to_rfc3339()),
        "next_run_utc": t.next_run_utc.to_rfc3339(),
        "source": t.source,
    })
}

fn get_templates() -> Vec<Value> {
    vec![
        json!({
            "id": "balanced_default",
            "title": "Balanced Default",
            "description": "Daily smoke, weekly quality gate, and periodic dashboard refresh.",
            "scheduler_enabled": true,
            "tasks": [
                {"id": "nightly_smoke", "action": "qemu_smoke", "interval_sec": 86400, "priority": "low", "enabled": true},
                {"id": "weekly_quality_gate", "action": "quality_gate", "interval_sec": 604800, "priority": "low", "enabled": true},
                {"id": "dashboard_refresh", "action": "dashboard_build", "interval_sec": 21600, "priority": "low", "enabled": true},
            ]
        }),
        json!({
            "id": "release_hardening",
            "title": "Release Hardening",
            "description": "More frequent smoke and gate cadence for RC periods.",
            "scheduler_enabled": true,
            "tasks": [
                {"id": "release_smoke_6h", "action": "qemu_smoke", "interval_sec": 21600, "priority": "normal", "enabled": true},
                {"id": "release_gate_daily", "action": "quality_gate", "interval_sec": 86400, "priority": "normal", "enabled": true},
                {"id": "release_dashboard_2h", "action": "dashboard_build", "interval_sec": 7200, "priority": "low", "enabled": true},
            ]
        }),
        json!({
            "id": "diagnostics_focus",
            "title": "Diagnostics Focus",
            "description": "Frequent diagnostics for unstable periods.",
            "scheduler_enabled": true,
            "tasks": [
                {"id": "diag_crash_bundle_4h", "action": "crash_diagnostics", "interval_sec": 14400, "priority": "normal", "enabled": true},
                {"id": "diag_triage_4h", "action": "crash_triage", "interval_sec": 14400, "priority": "normal", "enabled": true},
                {"id": "diag_smoke_12h", "action": "qemu_smoke", "interval_sec": 43200, "priority": "low", "enabled": true},
            ]
        }),
    ]
}
