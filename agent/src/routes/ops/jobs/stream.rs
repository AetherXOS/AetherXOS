use rocket::response::stream::{Event, EventStream};
use rocket::serde::json::{json, Value};
use rocket::State;

use crate::auth::RequireViewer;
use crate::models::JobStatus;
use crate::resp::{err, ok};
use crate::state::AppState;

#[rocket::get("/job/stream?<id>")]
pub fn job_stream(state: &State<AppState>, id: String, _role: RequireViewer) -> Value {
    let inner = state.read();
    match inner.jobs.get(&id) {
        Some(j) => ok(
            "job_stream",
            json!({
                "id": id,
                "status": j.status.as_str(),
                "lines": j.output,
            }),
        ),
        None => err("not_found", "Job not found.", "not_found"),
    }
}

#[rocket::get("/job/events?<id>&<follow>&<heartbeat_ms>")]
pub fn job_events(
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
