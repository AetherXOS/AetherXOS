use chrono::Utc;
use rocket::serde::json::{json, Value};
use rocket::State;
use crate::auth::RequireViewer;
use crate::resp::ok;
use crate::state::AppState;

// ── GET /api/launcher/agent-status ───────────────────────────────────────────

#[rocket::get("/api/launcher/agent-status")]
pub fn launcher_agent_status(state: &State<AppState>, _role: RequireViewer) -> Value {
    let inner = state.read();
    ok("launcher_agent_status", json!({
        "running": true,
        "started_utc": inner.started_utc.to_rfc3339(),
        "pid": std::process::id(),
        "port": 7401,
    }))
}

// ── GET /api/launcher/audit ───────────────────────────────────────────────────

#[rocket::get("/api/launcher/audit")]
pub fn launcher_audit(state: &State<AppState>, _role: RequireViewer) -> Value {
    let inner = state.read();
    let recent: Vec<Value> = inner.recent.iter().rev().take(50).map(|r| json!({
        "id": r.id,
        "action": r.action,
        "status": r.status,
        "started_utc": r.started_utc.map(|d| d.to_rfc3339()),
        "finished_utc": r.finished_utc.map(|d| d.to_rfc3339()),
        "exit_code": r.exit_code,
        "source": r.source,
    })).collect();
    ok("launcher_audit", json!({
        "entries": recent,
        "total": inner.recent.len(),
        "as_of_utc": Utc::now().to_rfc3339(),
    }))
}

// ── POST /api/launcher/start-agent ───────────────────────────────────────────

#[rocket::post("/api/launcher/start-agent")]
pub fn launcher_start_agent(_role: RequireViewer) -> Value {
    // Already running — return current status
    ok("launcher_start_agent", json!({
        "status": "already_running",
        "note": "Agent is this process; it is already up.",
        "pid": std::process::id(),
    }))
}

// ── POST /api/launcher/stop-agent ────────────────────────────────────────────

#[rocket::post("/api/launcher/stop-agent")]
pub fn launcher_stop_agent(_state: &State<AppState>, _role: crate::auth::RequireAdmin) -> Value {
    tracing::warn!("Stop-agent requested via launcher API. Initiating graceful shutdown.");
    // Signal Tokio runtime to shut down
    tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        std::process::exit(0);
    });
    ok("launcher_stop_agent", json!({
        "status": "stopping",
        "note": "Agent will stop shortly.",
    }))
}

// ── POST /api/launcher/restart-agent ─────────────────────────────────────────

#[rocket::post("/api/launcher/restart-agent")]
pub fn launcher_restart_agent(_role: crate::auth::RequireAdmin) -> Value {
    ok("launcher_restart_agent", json!({
        "status": "unsupported",
        "note": "Restart must be performed by the process supervisor.",
    }))
}
