use crate::auth::{RequireOperator, RequireViewer};
use crate::models::Confirmation;
use crate::resp::{err_detail, ok};
use crate::state::AppState;
use chrono::Utc;
use rocket::State;
use rocket::serde::json::{Json, Value, json};
use serde::Deserialize;
use uuid::Uuid;

const CONFIRM_TTL_SEC: i64 = 120;

// ── GET /confirm/list ─────────────────────────────────────────────────────────

#[rocket::get("/confirm/list")]
pub fn confirm_list(state: &State<AppState>, _role: RequireViewer) -> Value {
    let now = Utc::now();
    let inner = state.read();
    let confs: Vec<Value> = inner
        .confirmations
        .values()
        .filter(|c| c.expires_utc > now)
        .map(|c| {
            json!({
                "id": c.id,
                "action": c.action,
                "priority": c.priority,
                "issued_utc": c.issued_utc.to_rfc3339(),
                "expires_utc": c.expires_utc.to_rfc3339(),
                "role": c.role,
            })
        })
        .collect();
    ok("confirm_list", json!({ "confirmations": confs }))
}

// ── POST /confirm/request ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ConfirmRequestPayload {
    pub action: String,
    pub priority: Option<String>,
}

#[rocket::post("/confirm/request", data = "<body>")]
pub fn confirm_request(
    state: &State<AppState>,
    role: RequireOperator,
    body: Json<ConfirmRequestPayload>,
) -> Value {
    let action_id = body.action.trim().to_string();
    let priority = body.priority.clone().unwrap_or_else(|| "normal".into());

    {
        let inner = state.read();
        if inner.action_by_id(&action_id).is_none() {
            return err_detail(
                "unknown_action",
                "Action not found.",
                "not_found",
                json!({ "action": action_id }),
            );
        }
        // Only HIGH risk actions actually need confirmation; others get a stub
        let action = inner.action_by_id(&action_id).unwrap();
        if action.risk != "HIGH" {
            return ok(
                "confirm_not_required",
                json!({
                    "confirmation_id": null,
                    "action": action_id,
                    "note": "This action does not require a confirmation token.",
                }),
            );
        }
    }

    let now = Utc::now();
    let conf = Confirmation {
        id: Uuid::new_v4().to_string(),
        action: action_id.clone(),
        priority: priority.clone(),
        issued_utc: now,
        expires_utc: now + chrono::Duration::seconds(CONFIRM_TTL_SEC),
        role: role.0.clone(),
    };

    let cid = conf.id.clone();
    let expires = conf.expires_utc;
    {
        let mut inner = state.write();
        // Prune expired
        let expired: Vec<String> = inner
            .confirmations
            .values()
            .filter(|c| c.expires_utc <= now)
            .map(|c| c.id.clone())
            .collect();
        for id in expired {
            inner.confirmations.remove(&id);
        }
        inner.confirmations.insert(cid.clone(), conf);
    }

    ok(
        "confirm_issued",
        json!({
            "confirmation_id": cid,
            "action": action_id,
            "priority": priority,
            "expires_utc": expires.to_rfc3339(),
            "ttl_sec": CONFIRM_TTL_SEC,
        }),
    )
}

// ── POST /confirm/revoke ──────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ConfirmRevokePayload {
    pub id: String,
}

#[rocket::post("/confirm/revoke", data = "<body>")]
pub fn confirm_revoke(
    state: &State<AppState>,
    _role: RequireOperator,
    body: Json<ConfirmRevokePayload>,
) -> Value {
    let mut inner = state.write();
    if inner.confirmations.remove(&body.id).is_some() {
        ok("confirm_revoked", json!({ "id": body.id }))
    } else {
        err_detail(
            "not_found",
            "Confirmation not found or already expired.",
            "not_found",
            json!({ "id": body.id }),
        )
    }
}
