use chrono::Utc;
use rocket::serde::json::{json, Json, Value};
use rocket::State;
use serde::Deserialize;
use crate::auth::RequireAdmin;
use crate::auth::RequireViewer;
use crate::resp::{err_detail, ok};
use crate::state::AppState;

/// GET /auth/status
#[rocket::get("/auth/status")]
pub fn auth_status(state: &State<AppState>, _role: RequireViewer) -> Value {
    let inner = state.read();
    let roles: Vec<Value> = ["viewer", "operator", "admin"]
        .iter()
        .map(|r| {
            let tok = inner.tokens.tokens.get(*r).cloned().unwrap_or_default();
            let updated = inner.tokens.updated.get(*r).and_then(|d| d.as_ref()).map(|d| d.to_rfc3339()).unwrap_or_default();
            json!({
                "role": r,
                "token_set": !tok.is_empty(),
                "updated_utc": updated,
            })
        })
        .collect();

    ok("auth_status", json!({
        "auth_mode": if inner.unsafe_no_auth { "unsafe" } else { "strict" },
        "roles": roles,
    }))
}

#[derive(Deserialize)]
pub struct RotatePayload {
    pub role: Option<String>,
    pub rotate_all: Option<bool>,
}

/// POST /auth/rotate
#[rocket::post("/auth/rotate", data = "<body>")]
pub fn auth_rotate(
    state: &State<AppState>,
    _role: RequireAdmin,
    body: Json<RotatePayload>,
) -> Value {
    let rotate_all = body.rotate_all.unwrap_or(false);
    let role = body.role.as_deref().unwrap_or("").to_string();

    let targets: Vec<&str> = if rotate_all {
        vec!["viewer", "operator", "admin"]
    } else {
        if !["viewer", "operator", "admin"].contains(&role.as_str()) {
            return err_detail(
                "invalid_role",
                "role must be one of viewer|operator|admin when rotate_all is false.",
                "invalid_payload",
                json!({ "role": role }),
            );
        }
        vec![Box::leak(role.into_boxed_str())]
    };

    let mut rotated: Vec<Value> = vec![];
    let mut inner = state.write();

    for &r in &targets {
        let new_token = generate_token();
        inner.tokens.tokens.insert(r.to_string(), new_token.clone());
        let now = Utc::now();
        inner.tokens.updated.insert(r.to_string(), Some(now));
        rotated.push(json!({
            "role": r,
            "token": new_token,
            "updated_utc": now.to_rfc3339(),
        }));
    }

    // Build auth status inline
    let auth_roles: Vec<Value> = ["viewer", "operator", "admin"]
        .iter()
        .map(|r| {
            let tok = inner.tokens.tokens.get(*r).cloned().unwrap_or_default();
            let updated = inner.tokens.updated.get(*r).and_then(|d| d.as_ref()).map(|d| d.to_rfc3339()).unwrap_or_default();
            json!({
                "role": r,
                "token_set": !tok.is_empty(),
                "updated_utc": updated,
            })
        })
        .collect();

    ok("auth_rotated", json!({
        "rotated": rotated,
        "auth": {
            "auth_mode": if inner.unsafe_no_auth { "unsafe" } else { "strict" },
            "roles": auth_roles,
        },
    }))
}

fn generate_token() -> String {
    // Use two v4 UUIDs (128 bits each = 256 bits of entropy total) as token
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}
