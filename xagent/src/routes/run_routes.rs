use crate::actions::{dispatch_queue, enqueue_job_idempotent};
use crate::auth::{OptionalIdempotencyKey, RequireOperator};
use crate::resp::{err, err_detail, ok};
use crate::state::AppState;
use chrono::Utc;
use rocket::State;
use rocket::serde::json::{Json, Value, json};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunPayload {
    pub action: String,
    pub priority: Option<String>,
    pub confirmation_id: Option<String>,
}

/// POST /run  — synchronous: queues + dispatches, returns job id
#[rocket::post("/run", data = "<body>")]
pub fn run(
    state: &State<AppState>,
    role: RequireOperator,
    idem: OptionalIdempotencyKey,
    body: Json<RunPayload>,
) -> Value {
    run_inner(state, &role.0, idem.0.as_deref(), &body, "api:run")
}

/// POST /run_async — alias for /run
#[rocket::post("/run_async", data = "<body>")]
pub fn run_async(
    state: &State<AppState>,
    role: RequireOperator,
    idem: OptionalIdempotencyKey,
    body: Json<RunPayload>,
) -> Value {
    run_inner(state, &role.0, idem.0.as_deref(), &body, "api:run_async")
}

fn run_inner(
    state: &State<AppState>,
    role: &str,
    idem_key: Option<&str>,
    body: &RunPayload,
    source: &str,
) -> Value {
    let action_id = body.action.trim().to_string();
    if action_id.is_empty() {
        return err_detail(
            "invalid_action",
            "action must be a non-empty catalog id.",
            "invalid_payload",
            json!({ "action": body.action }),
        );
    }
    let priority = body.priority.clone().unwrap_or_else(|| "normal".into());

    if !["high", "normal", "low"].contains(&priority.as_str()) {
        return err_detail(
            "invalid_priority",
            "priority must be one of high|normal|low.",
            "invalid_payload",
            json!({ "priority": priority }),
        );
    }

    // Check action exists
    {
        let inner = state.read();
        if inner.action_by_id(&action_id).is_none() {
            return err_detail(
                "unknown_action",
                "Action not found in catalog.",
                "not_found",
                json!({ "action": action_id }),
            );
        }
        // Policy check
        if let Err(reason) = inner.check_policy(role, &action_id) {
            return err_detail(
                "policy_denied",
                "Policy does not permit this action for your role.",
                "forbidden",
                json!({ "reason": reason, "role": role, "action": action_id }),
            );
        }
        // Confirmation check for HIGH risk
        let action = inner.action_by_id(&action_id).unwrap();
        if action.risk == "HIGH" {
            if let Some(cid) = &body.confirmation_id {
                if let Some(conf) = inner.confirmations.get(cid) {
                    if conf.action != action_id {
                        return err(
                            "confirmation_mismatch",
                            "Confirmation is for a different action.",
                            "conflict",
                        );
                    }
                    if Utc::now() > conf.expires_utc {
                        return err(
                            "confirmation_expired",
                            "Confirmation token has expired.",
                            "gone",
                        );
                    }
                } else {
                    return err(
                        "confirmation_not_found",
                        "Confirmation ID not found.",
                        "not_found",
                    );
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
    }

    // Consume confirmation if present
    if let Some(cid) = &body.confirmation_id {
        let mut inner = state.write();
        inner.confirmations.remove(cid);
    }

    match enqueue_job_idempotent(state, source, idem_key, &action_id, &priority, source) {
        Some((job_id, replayed)) => {
            dispatch_queue(state);
            ok(
                "queued",
                json!({ "id": job_id, "action": action_id, "priority": priority, "replayed": replayed, "idempotency_key": idem_key }),
            )
        }
        None => err("queue_full", "Job queue is full.", "too_many_requests"),
    }
}
