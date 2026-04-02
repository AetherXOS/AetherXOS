use std::process::Stdio;
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;
use crate::models::{AgentEvent, IdempotencyRecord, Job, JobStatus, RecentEntry};
use crate::state::AppState;

const PRIORITY_ORDER: &[&str] = &["high", "normal", "low"];

fn priority_rank(p: &str) -> usize {
    PRIORITY_ORDER.iter().position(|v| *v == p).unwrap_or(2)
}

/// Create a job and add it to the queue. Returns the job ID, or None if the queue is full.
pub fn enqueue_job(state: &AppState, action: &str, priority: &str, source: &str) -> Option<String> {
    let mut inner = state.write();

    // Verify action exists
    if inner.action_by_id(action).is_none() {
        return None;
    }

    if inner.queue.len() as u32 >= inner.max_queue {
        return None;
    }

    let job = Job::new(action, priority, source);
    let id = job.id.clone();
    inner.jobs.insert(id.clone(), job);

    // Insert into queue sorted by priority
    let rank = priority_rank(priority);
    let insert_pos = inner
        .queue
        .iter()
        .position(|jid| {
            inner
                .jobs
                .get(jid)
                .map(|j| priority_rank(&j.priority) > rank)
                .unwrap_or(false)
        })
        .unwrap_or(inner.queue.len());
    inner.queue.insert(insert_pos, id.clone());

    inner.push_event(AgentEvent {
        id: Uuid::new_v4().to_string(),
        kind: "job_queued".into(),
        ts_utc: Utc::now(),
        related_id: Some(id.clone()),
        action: Some(action.to_string()),
        status: Some(JobStatus::Queued.as_str().to_string()),
        source: Some(source.to_string()),
        detail: json!({ "priority": priority }),
    });

    Some(id)
}

pub fn enqueue_job_idempotent(
    state: &AppState,
    route: &str,
    key: Option<&str>,
    action: &str,
    priority: &str,
    source: &str,
) -> Option<(String, bool)> {
    if let Some(raw_key) = key {
        let normalized = raw_key.trim();
        if !normalized.is_empty() {
            let inner = state.read();
            if let Some(record) = inner.idempotency.get(normalized) {
                if record.route == route && record.action == action {
                    if let Some(job_id) = record.job_ids.first() {
                        return Some((job_id.clone(), true));
                    }
                }
            }
        }
    }

    let job_id = enqueue_job(state, action, priority, source)?;
    if let Some(raw_key) = key {
        let normalized = raw_key.trim();
        if !normalized.is_empty() {
            let mut inner = state.write();
            inner.idempotency.insert(
                normalized.to_string(),
                IdempotencyRecord {
                    key: normalized.to_string(),
                    created_utc: Utc::now(),
                    route: route.to_string(),
                    action: action.to_string(),
                    job_ids: vec![job_id.clone()],
                },
            );
        }
    }
    Some((job_id, false))
}

pub fn retry_job(state: &AppState, original_job_id: &str, source: &str) -> Result<String, &'static str> {
    let (action, priority, retry_count) = {
        let inner = state.read();
        let job = inner.jobs.get(original_job_id).ok_or("not_found")?;
        let retry_count = inner
            .jobs
            .values()
            .filter(|candidate| candidate.source.contains(original_job_id) && candidate.action == job.action)
            .count();
        (job.action.clone(), job.priority.clone(), retry_count)
    };

    let source = format!("{}:retry-of:{}:attempt:{}", source, original_job_id, retry_count + 1);
    enqueue_job(state, &action, &priority, &source).ok_or("enqueue_failed")
}

/// Dispatch queued jobs up to concurrency limit. Each dispatched job is
/// spawned as a Tokio task that runs the external `aethercore.ps1` command.
pub fn dispatch_queue(state: &AppState) {
    loop {
        let (job_id, action_cmd, action_args) = {
            let inner = state.read();
            if inner.queue.is_empty() || inner.is_busy() {
                return;
            }
            let job_id = inner.queue[0].clone();
            let job = match inner.jobs.get(&job_id) {
                Some(j) => j,
                None => return,
            };
            let action = match inner.action_by_id(&job.action) {
                Some(a) => a,
                None => return,
            };
            (job_id, action.cmd.clone(), action.args.clone())
        };

        // Transition job to Running
        {
            let mut inner = state.write();
            inner.queue.retain(|id| id != &job_id);
            if let Some(job) = inner.jobs.get_mut(&job_id) {
                job.status = JobStatus::Running;
                job.started_utc = Some(Utc::now());
                let action = job.action.clone();
                let source = job.source.clone();
                inner.push_event(AgentEvent {
                    id: Uuid::new_v4().to_string(),
                    kind: "job_started".into(),
                    ts_utc: Utc::now(),
                    related_id: Some(job_id.clone()),
                    action: Some(action),
                    status: Some(JobStatus::Running.as_str().to_string()),
                    source: Some(source),
                    detail: json!({}),
                });
            }
        }

        // Spawn background task
        let state_clone = state.clone();
        let jid = job_id.clone();
        tokio::spawn(async move {
            run_job(state_clone, jid, action_cmd, action_args).await;
        });
    }
}

async fn run_job(state: AppState, job_id: String, cmd: String, args: Vec<String>) {
    // Determine the script path relative to the workspace root.
    // Works whether the binary is run from agent/ or from OS/.
    let script_root = locate_scripts_root();
    let aethercore_ps1 = {
        let candidate = format!("{}/aethercore.ps1", script_root);
        if std::path::Path::new(&candidate).exists() {
            candidate
        } else {
            format!("{}/hypercore.ps1", script_root)
        }
    };

    let mut pwsh_args: Vec<String> = vec![
        "-NoProfile".into(),
        "-NonInteractive".into(),
        "-ExecutionPolicy".into(),
        "Bypass".into(),
        "-File".into(),
        aethercore_ps1,
        cmd.clone(),
    ];
    pwsh_args.extend_from_slice(&args);

    let (exit_code, output, err) = execute_pwsh(&pwsh_args).await;

    let finished = Utc::now();
    let status = if exit_code == Some(0) { JobStatus::Done } else { JobStatus::Failed };

    let recent = {
        let mut inner = state.write();
        let mut event_payload = None;
        if let Some(job) = inner.jobs.get_mut(&job_id) {
            job.status = status.clone();
            job.finished_utc = Some(finished);
            job.exit_code = exit_code;
            job.output = output.clone();
            job.error = err.clone();
            event_payload = Some((job.action.clone(), job.source.clone(), job.status.as_str().to_string(), job.error.clone()));
        }
        if let Some((action, source, final_status, error)) = event_payload {
            inner.push_event(AgentEvent {
                id: Uuid::new_v4().to_string(),
                kind: "job_finished".into(),
                ts_utc: finished,
                related_id: Some(job_id.clone()),
                action: Some(action),
                status: Some(final_status),
                source: Some(source),
                detail: json!({ "exit_code": exit_code, "error": error }),
            });
        }
        let job = inner.jobs.get(&job_id).cloned();
        job.map(|j| RecentEntry {
            id: j.id.clone(),
            action: j.action.clone(),
            status: j.status.as_str().to_string(),
            started_utc: j.started_utc,
            finished_utc: j.finished_utc,
            exit_code: j.exit_code,
            source: j.source.clone(),
        })
    };

    if let Some(entry) = recent {
        let mut inner = state.write();
        inner.push_recent(entry);
    }

    tracing::info!(
        job_id = %job_id,
        cmd = %cmd,
        exit_code = ?exit_code,
        "job finished"
    );
}

async fn execute_pwsh(args: &[String]) -> (Option<i32>, Vec<String>, Option<String>) {
    let pwsh_bin = if cfg!(windows) { "pwsh" } else { "pwsh" };

    let child_result = tokio::process::Command::new(pwsh_bin)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    let child = match child_result {
        Ok(c) => c,
        Err(e) => {
            return (Some(-1), vec![], Some(format!("spawn failed: {e}")));
        }
    };

    let output = match child.wait_with_output().await {
        Ok(o) => o,
        Err(e) => {
            return (Some(-1), vec![], Some(format!("wait failed: {e}")));
        }
    };

    let exit_code = output.status.code();
    let stdout_lines: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|l| l.to_string())
        .collect();
    let stderr = if output.stderr.is_empty() {
        None
    } else {
        Some(String::from_utf8_lossy(&output.stderr).to_string())
    };

    (exit_code, stdout_lines, stderr)
}

fn locate_scripts_root() -> String {
    // Try common locations: cwd/scripts, cwd/../scripts
    let candidates = [
        "scripts",
        "../scripts",
        "../../scripts",
    ];
    for c in candidates {
        if std::path::Path::new(c).join("aethercore.ps1").exists() || std::path::Path::new(c).join("hypercore.ps1").exists() {
            return c.to_string();
        }
    }
    "scripts".to_string()
}
