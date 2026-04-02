use rocket::State;
use rocket::form::FromForm;
use rocket::serde::json::{Value, json};

use crate::auth::RequireViewer;
use crate::models::JobStatus;
use crate::resp::ok;
use crate::state::AppState;

#[derive(Debug, Clone, FromForm)]
pub struct JobsQuery {
    pub status: Option<String>,
    pub action: Option<String>,
    pub source: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub order: Option<String>,
}

#[rocket::get("/jobs?<q..>")]
pub fn list_jobs(state: &State<AppState>, _role: RequireViewer, q: Option<JobsQuery>) -> Value {
    let query = q.unwrap_or(JobsQuery {
        status: None,
        action: None,
        source: None,
        limit: None,
        offset: None,
        order: None,
    });

    let status_filter = query.status.as_deref().map(|s| s.to_lowercase());
    let action_filter = query.action.as_deref().map(|s| s.to_lowercase());
    let source_filter = query.source.as_deref().map(|s| s.to_lowercase());

    let inner = state.read();
    let mut filtered: Vec<&crate::models::Job> = inner
        .jobs
        .values()
        .filter(|j| {
            status_filter
                .as_ref()
                .map(|s| j.status.as_str() == s)
                .unwrap_or(true)
                && action_filter
                    .as_ref()
                    .map(|a| j.action.to_lowercase().contains(a))
                    .unwrap_or(true)
                && source_filter
                    .as_ref()
                    .map(|s| j.source.to_lowercase().contains(s))
                    .unwrap_or(true)
        })
        .collect();

    let order = query.order.as_deref().unwrap_or("desc").to_lowercase();
    filtered.sort_by(|a, b| {
        if order == "asc" {
            a.queued_utc.cmp(&b.queued_utc)
        } else {
            b.queued_utc.cmp(&a.queued_utc)
        }
    });

    let total = filtered.len();
    let offset = query.offset.unwrap_or(0).min(total);
    let limit = query.limit.unwrap_or(50).clamp(1, 500);
    let end = (offset + limit).min(total);

    let jobs: Vec<Value> = filtered[offset..end]
        .iter()
        .map(|j| j.summary_value())
        .collect();
    ok(
        "jobs",
        json!({
            "jobs": jobs,
            "total": total,
            "offset": offset,
            "limit": limit,
            "returned": jobs.len(),
            "order": order,
            "filters": {
                "status": query.status,
                "action": query.action,
                "source": query.source,
            }
        }),
    )
}

#[rocket::get("/jobs/stats")]
pub fn jobs_stats(state: &State<AppState>, _role: RequireViewer) -> Value {
    let inner = state.read();
    let queued = inner
        .jobs
        .values()
        .filter(|j| j.status == JobStatus::Queued)
        .count();
    let running = inner
        .jobs
        .values()
        .filter(|j| j.status == JobStatus::Running)
        .count();
    let done = inner
        .jobs
        .values()
        .filter(|j| j.status == JobStatus::Done)
        .count();
    let failed = inner
        .jobs
        .values()
        .filter(|j| j.status == JobStatus::Failed)
        .count();
    let cancelled = inner
        .jobs
        .values()
        .filter(|j| j.status == JobStatus::Cancelled)
        .count();

    ok(
        "jobs_stats",
        json!({
            "total": inner.jobs.len(),
            "queue_depth": inner.queue_count(),
            "running_count": inner.running_count(),
            "by_status": {
                "queued": queued,
                "running": running,
                "done": done,
                "failed": failed,
                "cancelled": cancelled,
            }
        }),
    )
}
