use rocket::serde::json::{json, Json, Value};
use rocket::response::stream::{Event, EventStream};
use rocket::State;
use serde::Deserialize;
use crate::auth::{OptionalIdempotencyKey, RequireOperator, RequireViewer};
use crate::models::JobStatus;
use crate::resp::{err, err_detail, ok};
use crate::state::AppState;
use crate::actions::{dispatch_queue, enqueue_job, enqueue_job_idempotent, retry_job};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DispatchPayload {
    pub host_id: Option<String>,
    pub action: String,
    pub priority: Option<String>,
    pub confirmation_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FanoutPayload {
    pub action: String,
    pub priority: Option<String>,
    pub hosts: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DispatchCancelPayload {
    pub id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DispatchRetryPayload {
    pub id: String,
}

// ── POST /dispatch/run_async ──────────────────────────────────────────────────

/// POST /dispatch/run_async
/// Dispatches an action to a specific host (or local if host_id == "local" or omitted)
#[rocket::post("/dispatch/run_async", data = "<body>")]
pub fn dispatch_run_async(
    state: &State<AppState>,
    role: RequireOperator,
    idem: OptionalIdempotencyKey,
    body: Json<DispatchPayload>,
) -> Value {
    let host_id = body.host_id.clone().unwrap_or_else(|| "local".into());
    let action_id = body.action.trim().to_string();
    if action_id.is_empty() {
        return err_detail("invalid_action", "action must be non-empty.", "invalid_payload", json!({ "action": body.action }));
    }
    let priority = body.priority.clone().unwrap_or_else(|| "normal".into());

    if !["high", "normal", "low"].contains(&priority.as_str()) {
        return err_detail(
            "invalid_priority",
            "priority must be high|normal|low.",
            "invalid_payload",
            json!({ "priority": priority }),
        );
    }

    let is_high_risk = {
        let inner = state.read();
        if !inner.hosts.iter().any(|h| h.id == host_id) {
            return err_detail("host_not_found", "Host not registered.", "not_found", json!({ "host_id": host_id }));
        }
        let action = match inner.action_by_id(&action_id) {
            Some(a) => a,
            None => return err_detail("unknown_action", "Action not found.", "not_found", json!({ "action": action_id })),
        };
        if let Err(reason) = inner.check_policy(&role.0, &action_id) {
            return err_detail("policy_denied", "Policy denies this action.", "forbidden", json!({ "reason": reason }));
        }
        if action.risk == "HIGH" {
            if let Some(cid) = &body.confirmation_id {
                if let Some(conf) = inner.confirmations.get(cid) {
                    if conf.action != action_id {
                        return err("confirmation_mismatch", "Confirmation is for a different action.", "conflict");
                    }
                    if chrono::Utc::now() > conf.expires_utc {
                        return err("confirmation_expired", "Confirmation token has expired.", "gone");
                    }
                } else {
                    return err("confirmation_not_found", "Confirmation ID not found.", "not_found");
                }
            } else {
                return err_detail(
                    "confirmation_required",
                    "This HIGH-risk action requires a confirmation_id. Use POST /confirm/request first.",
                    "precondition_failed",
                    json!({ "action": action_id }),
                );
            }
        }
        action.risk == "HIGH"
    };

    if is_high_risk {
        if let Some(cid) = &body.confirmation_id {
            let mut inner = state.write();
            inner.confirmations.remove(cid);
        }
    }

    let source = format!("dispatch:host:{}", host_id);
    match enqueue_job_idempotent(state, "dispatch/run_async", idem.0.as_deref(), &action_id, &priority, &source) {
        Some((job_id, replayed)) => {
            dispatch_queue(state);
            ok("dispatched", json!({ "id": job_id, "host_id": host_id, "action": action_id, "priority": priority, "replayed": replayed, "idempotency_key": idem.0 }))
        }
        None => err("queue_full", "Job queue is full.", "too_many_requests"),
    }
}

// ── POST /dispatch/fanout ─────────────────────────────────────────────────────

/// POST /dispatch/fanout — enqueues the action once per requested host
#[rocket::post("/dispatch/fanout", data = "<body>")]
pub fn dispatch_fanout(
    state: &State<AppState>,
    role: RequireOperator,
    idem: OptionalIdempotencyKey,
    body: Json<FanoutPayload>,
) -> Value {
    let action_id = body.action.trim().to_string();
    if action_id.is_empty() {
        return err_detail("invalid_action", "action must be non-empty.", "invalid_payload", json!({ "action": body.action }));
    }
    let priority = body.priority.clone().unwrap_or_else(|| "normal".into());

    if !["high", "normal", "low"].contains(&priority.as_str()) {
        return err_detail("invalid_priority", "priority must be high|normal|low.", "invalid_payload", json!({ "priority": priority }));
    }

    let host_ids: Vec<String> = {
        let inner = state.read();
        if inner.action_by_id(&action_id).is_none() {
            return err_detail("unknown_action", "Action not found.", "not_found", json!({ "action": action_id }));
        }
        if let Err(reason) = inner.check_policy(&role.0, &action_id) {
            return err_detail("policy_denied", "Policy denies this action.", "forbidden", json!({ "reason": reason }));
        }
        match &body.hosts {
            Some(ids) => ids.iter().filter(|id| inner.hosts.iter().any(|h| &h.id == *id)).cloned().collect(),
            None => inner.hosts.iter().filter(|h| h.enabled).map(|h| h.id.clone()).collect(),
        }
    };

    if let Some(existing_key) = idem.0.as_deref() {
        let inner = state.read();
        if let Some(record) = inner.idempotency.get(existing_key) {
            if record.route == "dispatch/fanout" && record.action == action_id {
                return ok("fanout_dispatched", json!({
                    "dispatched": record.job_ids.iter().map(|job_id| json!({ "job_id": job_id })).collect::<Vec<_>>(),
                    "count": record.job_ids.len(),
                    "replayed": true,
                    "idempotency_key": idem.0,
                }));
            }
        }
    }

    let mut dispatched: Vec<Value> = vec![];
    let mut job_ids: Vec<String> = vec![];
    for host_id in &host_ids {
        let source = format!("fanout:host:{}", host_id);
        if let Some(job_id) = enqueue_job(state, &action_id, &priority, &source) {
            dispatched.push(json!({ "host_id": host_id, "job_id": job_id }));
            job_ids.push(dispatched.last().and_then(|v| v.get("job_id")).and_then(|v| v.as_str()).unwrap_or_default().to_string());
        }
    }
    if let Some(existing_key) = idem.0.as_deref() {
        let normalized = existing_key.trim();
        if !normalized.is_empty() {
            let mut inner = state.write();
            inner.idempotency.insert(
                normalized.to_string(),
                crate::models::IdempotencyRecord {
                    key: normalized.to_string(),
                    created_utc: chrono::Utc::now(),
                    route: "dispatch/fanout".into(),
                    action: action_id.clone(),
                    job_ids,
                },
            );
        }
    }
    dispatch_queue(state);

    ok("fanout_dispatched", json!({ "dispatched": dispatched, "count": dispatched.len(), "replayed": false, "idempotency_key": idem.0 }))
}

// ── GET /dispatch/jobs ────────────────────────────────────────────────────────

#[rocket::get("/dispatch/jobs")]
pub fn dispatch_jobs(state: &State<AppState>, _role: RequireViewer) -> Value {
    let inner = state.read();
    let jobs: Vec<Value> = inner
        .jobs
        .values()
        .filter(|j| j.source.starts_with("dispatch:") || j.source.starts_with("fanout:"))
        .map(|j| json!({
            "id": j.id,
            "action": j.action,
            "status": j.status.as_str(),
            "source": j.source,
            "queued_utc": j.queued_utc.to_rfc3339(),
            "finished_utc": j.finished_utc.map(|d| d.to_rfc3339()),
            "exit_code": j.exit_code,
        }))
        .collect();
    ok("dispatch_jobs", json!({ "jobs": jobs }))
}

// ── GET /dispatch/job?id=<id> ─────────────────────────────────────────────────

#[rocket::get("/dispatch/job?<id>")]
pub fn dispatch_job(state: &State<AppState>, id: String, _role: RequireViewer) -> Value {
    let inner = state.read();
    match inner.jobs.get(&id) {
        Some(j) => ok("dispatch_job", json!({
            "job": {
                "id": j.id,
                "action": j.action,
                "status": j.status.as_str(),
                "source": j.source,
                "queued_utc": j.queued_utc.to_rfc3339(),
                "started_utc": j.started_utc.map(|d| d.to_rfc3339()),
                "finished_utc": j.finished_utc.map(|d| d.to_rfc3339()),
                "exit_code": j.exit_code,
                "output": j.output,
                "error": j.error,
            }
        })),
        None => err("not_found", "Job not found.", "not_found"),
    }
}

// ── POST /dispatch/job/cancel ────────────────────────────────────────────────

#[rocket::post("/dispatch/job/cancel", data = "<body>")]
pub fn dispatch_job_cancel(
    state: &State<AppState>,
    _role: RequireOperator,
    body: Json<DispatchCancelPayload>,
) -> Value {
    let id = &body.id;
    let mut inner = state.write();
    match inner.jobs.get_mut(id) {
        Some(j) if j.status == JobStatus::Queued => {
            j.status = JobStatus::Cancelled;
            inner.queue.retain(|jid| jid != id);
            ok("dispatch_job_cancelled", json!({ "id": id }))
        }
        Some(j) if j.status == JobStatus::Running => {
            err("running", "Cannot cancel a running job.", "conflict")
        }
        Some(_) => err("already_done", "Job is already terminal.", "conflict"),
        None => err("not_found", "Job not found.", "not_found"),
    }
}

// ── GET /dispatch/job/stream?id=<id> ─────────────────────────────────────────

#[rocket::get("/dispatch/job/stream?<id>")]
pub fn dispatch_job_stream(state: &State<AppState>, id: String, _role: RequireViewer) -> Value {
    let inner = state.read();
    match inner.jobs.get(&id) {
        Some(j) => ok("dispatch_job_stream", json!({
            "id": id,
            "status": j.status.as_str(),
            "lines": j.output,
        })),
        None => err("not_found", "Job not found.", "not_found"),
    }
}

#[rocket::get("/dispatch/job/events?<id>&<follow>&<heartbeat_ms>")]
pub fn dispatch_job_events(
    state: &State<AppState>,
    id: String,
    follow: Option<bool>,
    heartbeat_ms: Option<u64>,
    _role: RequireViewer,
) -> EventStream![] {
    let state = state.inner().clone();
    let follow = follow.unwrap_or(false);
    let heartbeat_ms = heartbeat_ms.unwrap_or(500).clamp(100, 5_000);

    EventStream! {
        let mut sent_lines = 0usize;
        let mut sent_snapshot = false;
        loop {
            let snapshot = {
                let inner = state.read();
                inner.jobs.get(&id).cloned()
            };

            match snapshot {
                Some(job) => {
                    if !sent_snapshot {
                        yield Event::json(&json!({
                            "type": "snapshot",
                            "id": job.id,
                            "action": job.action,
                            "status": job.status.as_str(),
                            "queued_utc": job.queued_utc.to_rfc3339(),
                            "started_utc": job.started_utc.map(|d| d.to_rfc3339()),
                            "finished_utc": job.finished_utc.map(|d| d.to_rfc3339()),
                            "exit_code": job.exit_code,
                            "line_count": job.output.len(),
                        }));
                        sent_snapshot = true;
                    }

                    while sent_lines < job.output.len() {
                        let line = job.output[sent_lines].clone();
                        sent_lines += 1;
                        yield Event::json(&json!({
                            "type": "line",
                            "index": sent_lines - 1,
                            "line": line,
                        }));
                    }

                    if matches!(job.status, JobStatus::Done | JobStatus::Failed | JobStatus::Cancelled)
                    {
                        yield Event::json(&json!({
                            "type": "complete",
                            "id": job.id,
                            "status": job.status.as_str(),
                            "finished_utc": job.finished_utc.map(|d| d.to_rfc3339()),
                            "exit_code": job.exit_code,
                            "error": job.error,
                        }));
                        break;
                    }
                }
                None => {
                    yield Event::json(&json!({
                        "type": "error",
                        "code": "not_found",
                        "message": "Job not found."
                    }));
                    break;
                }
            }

            if !follow && sent_snapshot {
                break;
            }

            yield Event::comment("heartbeat");
            tokio::time::sleep(std::time::Duration::from_millis(heartbeat_ms)).await;
        }
    }
}

#[rocket::post("/dispatch/job/retry", data = "<body>")]
pub fn dispatch_job_retry(
    state: &State<AppState>,
    _role: RequireOperator,
    body: Json<DispatchRetryPayload>,
) -> Value {
    match retry_job(state, &body.id, "api:dispatch-job-retry") {
        Ok(job_id) => {
            dispatch_queue(state);
            ok("dispatch_job_retried", json!({ "original_id": body.id, "id": job_id }))
        }
        Err("not_found") => err("not_found", "Job not found.", "not_found"),
        Err(_) => err("retry_failed", "Dispatch job could not be retried.", "conflict"),
    }
}
