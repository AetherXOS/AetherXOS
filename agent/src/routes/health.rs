use chrono::Utc;
use rocket::serde::json::Value;
use rocket::State;
use crate::auth::ResolvedRole;
use crate::resp::ok;
use crate::state::AppState;

/// GET /health
#[rocket::get("/health")]
pub fn health(state: &State<AppState>, role: ResolvedRole) -> Value {
    let inner = state.read();
    ok(
        "agent healthy",
        rocket::serde::json::json!({
            "busy": inner.is_busy(),
            "started_utc": inner.started_utc.to_rfc3339(),
            "now_utc": Utc::now().to_rfc3339(),
            "max_concurrency": inner.max_concurrency,
            "max_queue": inner.max_queue,
            "running_count": inner.running_count(),
            "queue_count": inner.queue_count(),
            "scheduler_enabled": inner.scheduler_enabled,
            "auth_mode": if inner.unsafe_no_auth { "unsafe" } else { "strict" },
            "unsafe_no_auth": inner.unsafe_no_auth,
            "role": role.0,
        }),
    )
}

/// GET /ready
#[rocket::get("/ready")]
pub fn ready(state: &State<AppState>, role: ResolvedRole) -> Value {
    let inner = state.read();
    let queue_available = inner.queue_count() < inner.max_queue as usize;
    let has_catalog = !inner.actions.is_empty();
    let ready = queue_available && has_catalog;
    ok(
        if ready { "ready" } else { "degraded" },
        rocket::serde::json::json!({
            "ready": ready,
            "queue_available": queue_available,
            "catalog_loaded": has_catalog,
            "role": role.0,
        }),
    )
}

/// GET /status
#[rocket::get("/status")]
pub fn status(state: &State<AppState>, role: ResolvedRole) -> Value {
    let inner = state.read();

    let action_stats: Vec<Value> = inner
        .actions
        .iter()
        .map(|a| {
            let runs = inner
                .jobs
                .values()
                .filter(|j| j.action == a.id)
                .count();
            rocket::serde::json::json!({
                "id": a.id,
                "title": a.title,
                "run_count": runs,
            })
        })
        .collect();

    ok(
        "status",
        rocket::serde::json::json!({
            "started_utc": inner.started_utc.to_rfc3339(),
            "now_utc": Utc::now().to_rfc3339(),
            "running_count": inner.running_count(),
            "queue_count": inner.queue_count(),
            "total_jobs": inner.jobs.len(),
            "scheduler_enabled": inner.scheduler_enabled,
            "log_retention_days": inner.log_retention_days,
            "audit_dir": inner.audit_dir,
            "recent_count": inner.recent.len(),
            "recent": inner.recent.iter().rev().take(20).collect::<Vec<_>>(),
            "actions": action_stats,
            "role": role.0,
        }),
    )
}

/// GET /metrics
#[rocket::get("/metrics")]
pub fn metrics(state: &State<AppState>, role: ResolvedRole) -> Value {
    use crate::models::JobStatus;
    let inner = state.read();

    let done = inner.jobs.values().filter(|j| j.status == JobStatus::Done).count();
    let failed = inner.jobs.values().filter(|j| j.status == JobStatus::Failed).count();
    let running = inner.running_count();
    let queued = inner.queue_count();
    let cancelled = inner.jobs.values().filter(|j| j.status == JobStatus::Cancelled).count();

    ok(
        "metrics",
        rocket::serde::json::json!({
            "jobs_total": inner.jobs.len(),
            "jobs_done": done,
            "jobs_failed": failed,
            "jobs_running": running,
            "jobs_queued": queued,
            "jobs_cancelled": cancelled,
            "scheduler_task_count": inner.schedules.len(),
            "host_count": inner.hosts.len(),
            "role": role.0,
        }),
    )
}

/// GET /state
#[rocket::get("/state")]
pub fn agent_state(state: &State<AppState>, role: ResolvedRole) -> Value {
    let inner = state.read();
    ok(
        "state",
        rocket::serde::json::json!({
            "started_utc": inner.started_utc.to_rfc3339(),
            "running_count": inner.running_count(),
            "queue_count": inner.queue_count(),
            "scheduler_enabled": inner.scheduler_enabled,
            "unsafe_no_auth": inner.unsafe_no_auth,
            "max_concurrency": inner.max_concurrency,
            "max_queue": inner.max_queue,
            "role": role.0,
        }),
    )
}

/// GET /queue
#[rocket::get("/queue")]
pub fn queue(state: &State<AppState>, role: ResolvedRole) -> Value {
    let inner = state.read();
    let queued: Vec<Value> = inner
        .queue
        .iter()
        .filter_map(|id| inner.jobs.get(id))
        .map(|job| rocket::serde::json::json!({
            "id": job.id,
            "action": job.action,
            "priority": job.priority,
            "queued_utc": job.queued_utc.to_rfc3339(),
            "source": job.source,
        }))
        .collect();
    let running: Vec<Value> = inner
        .jobs
        .values()
        .filter(|job| job.status == crate::models::JobStatus::Running)
        .map(|job| rocket::serde::json::json!({
            "id": job.id,
            "action": job.action,
            "priority": job.priority,
            "started_utc": job.started_utc.map(|d| d.to_rfc3339()),
            "source": job.source,
        }))
        .collect();
    ok(
        "queue",
        rocket::serde::json::json!({
            "queued": queued,
            "running": running,
            "queue_depth": inner.queue_count(),
            "running_count": inner.running_count(),
            "role": role.0,
        }),
    )
}
