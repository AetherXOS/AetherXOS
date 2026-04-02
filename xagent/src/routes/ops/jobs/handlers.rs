use rocket::State;
use rocket::serde::json::{Json, Value, json};
use serde::Deserialize;

use crate::actions::{dispatch_queue, retry_job};
use crate::auth::{RequireOperator, RequireViewer};
use crate::models::JobStatus;
use crate::resp::{err, ok};
use crate::state::AppState;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CancelPayload {
    pub id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetryPayload {
    pub id: String,
}

#[rocket::get("/job?<id>")]
pub fn get_job(state: &State<AppState>, id: String, _role: RequireViewer) -> Value {
    let inner = state.read();
    match inner.jobs.get(&id) {
        Some(j) => ok("job", json!({ "job": j.detail_value() })),
        None => err("not_found", "Job not found.", "not_found"),
    }
}

#[rocket::post("/job/cancel", data = "<body>")]
pub fn cancel_job(
    state: &State<AppState>,
    _role: RequireOperator,
    body: Json<CancelPayload>,
) -> Value {
    let id = &body.id;
    let mut inner = state.write();
    if let Some(job) = inner.jobs.get_mut(id) {
        match job.status {
            JobStatus::Queued => {
                job.status = JobStatus::Cancelled;
                inner.queue.retain(|jid| jid != id);
                return ok("job_cancelled", json!({ "id": id }));
            }
            JobStatus::Running => {
                return err("running", "Cannot cancel a running job.", "conflict");
            }
            _ => {
                return err(
                    "already_done",
                    "Job is already in a terminal state.",
                    "conflict",
                );
            }
        }
    }
    err("not_found", "Job not found.", "not_found")
}

#[rocket::post("/job/retry", data = "<body>")]
pub fn retry_existing_job(
    state: &State<AppState>,
    _role: RequireOperator,
    body: Json<RetryPayload>,
) -> Value {
    match retry_job(state, &body.id, "api:job-retry") {
        Ok(job_id) => {
            dispatch_queue(state);
            ok(
                "job_retried",
                json!({ "original_id": body.id, "id": job_id }),
            )
        }
        Err("not_found") => err("not_found", "Job not found.", "not_found"),
        Err(_) => err("retry_failed", "Job could not be retried.", "conflict"),
    }
}

#[rocket::delete("/jobs/prune?<hours>")]
pub fn prune_jobs(state: &State<AppState>, _role: RequireOperator, hours: Option<i64>) -> Value {
    let threshold = chrono::Utc::now() - chrono::Duration::hours(hours.unwrap_or(24).max(1));
    let mut inner = state.write();
    let before = inner.jobs.len();
    let removable: Vec<String> = inner
        .jobs
        .iter()
        .filter(|(_, job)| {
            matches!(
                job.status,
                JobStatus::Done | JobStatus::Failed | JobStatus::Cancelled
            ) && job.finished_utc.map(|ts| ts < threshold).unwrap_or(false)
        })
        .map(|(id, _)| id.clone())
        .collect();
    for id in &removable {
        inner.jobs.remove(id);
        inner.queue.retain(|queued_id| queued_id != id);
    }
    ok(
        "jobs_pruned",
        json!({
            "removed": removable.len(),
            "before": before,
            "after": inner.jobs.len(),
            "hours": hours.unwrap_or(24).max(1),
        }),
    )
}
