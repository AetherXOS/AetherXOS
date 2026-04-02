use crate::auth::{RequireAdmin, RequireViewer};
use crate::resp::{err, ok};
use chrono::Utc;
use rocket::serde::json::{Json, Value, json};
use serde::Deserialize;

// ── GET /config ────────────────────────────────────────────────────────────────

#[rocket::get("/config")]
pub fn get_config(_role: RequireViewer) -> Value {
    let raw = load_config_file();
    ok("config", json!({ "config": raw }))
}

// ── GET /config/compose ────────────────────────────────────────────────────────

#[rocket::get("/config/compose")]
pub fn config_compose(_role: RequireViewer) -> Value {
    // Returns a composed view of active settings
    let raw = load_config_file();
    ok(
        "config_compose",
        json!({
            "composed": raw,
            "as_of_utc": Utc::now().to_rfc3339(),
        }),
    )
}

// ── GET /config/drift ─────────────────────────────────────────────────────────

#[rocket::get("/config/drift")]
pub fn config_drift(_role: RequireViewer) -> Value {
    ok(
        "config_drift",
        json!({
            "drifted_keys": [],
            "as_of_utc": Utc::now().to_rfc3339(),
        }),
    )
}

// ── POST /config/drift/apply ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct DriftApplyPayload {
    pub keys: Option<Vec<String>>,
}

#[rocket::post("/config/drift/apply", data = "<body>")]
pub fn config_drift_apply(_role: RequireAdmin, body: Json<DriftApplyPayload>) -> Value {
    ok(
        "config_drift_applied",
        json!({
            "applied_keys": body.keys.as_deref().unwrap_or(&[]),
            "applied_utc": Utc::now().to_rfc3339(),
        }),
    )
}

// ── GET /config/export ────────────────────────────────────────────────────────

#[rocket::get("/config/export")]
pub fn config_export(_role: RequireAdmin) -> Value {
    let raw = load_config_file();
    ok("config_export", json!({ "export": raw }))
}

// ── GET /config/overrides/template ───────────────────────────────────────────

#[rocket::get("/config/overrides/template")]
pub fn config_overrides_template(_role: RequireViewer) -> Value {
    ok(
        "config_overrides_template",
        json!({
            "template": {
                "agent": {
                    "port": 7401,
                    "auth_mode": "strict",
                    "auth_token": "change-me",
                    "allowed_origins": ["http://localhost:5173"],
                    "max_concurrency": 1,
                    "max_queue": 100,
                }
            }
        }),
    )
}

// ── POST /config/update ────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ConfigUpdatePayload {
    pub key: String,
    pub value: Value,
}

#[rocket::post("/config/update", data = "<body>")]
pub fn config_update(_role: RequireAdmin, body: Json<ConfigUpdatePayload>) -> Value {
    if body.key.is_empty() {
        return err("invalid_key", "key is required.", "invalid_payload");
    }
    // In a full implementation this would write back to disk.
    ok(
        "config_updated",
        json!({
            "key": body.key,
            "value": body.value,
            "updated_utc": Utc::now().to_rfc3339(),
        }),
    )
}

// ── POST /config/auto ─────────────────────────────────────────────────────────

#[rocket::post("/config/auto")]
pub fn config_auto(_role: RequireAdmin) -> Value {
    ok(
        "config_auto_applied",
        json!({
            "applied_utc": Utc::now().to_rfc3339(),
            "changes": [],
        }),
    )
}

// ── POST /config/import ───────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ConfigImportPayload {
    pub config: Value,
}

#[rocket::post("/config/import", data = "<body>")]
pub fn config_import(_role: RequireAdmin, body: Json<ConfigImportPayload>) -> Value {
    ok(
        "config_imported",
        json!({
            "imported_utc": Utc::now().to_rfc3339(),
            "keys_imported": body.config.as_object().map(|o| o.len()).unwrap_or(0),
        }),
    )
}

// ── POST /config/compose/apply ────────────────────────────────────────────────

#[rocket::post("/config/compose/apply")]
pub fn config_compose_apply(_role: RequireAdmin) -> Value {
    ok(
        "config_compose_applied",
        json!({
            "applied_utc": Utc::now().to_rfc3339(),
        }),
    )
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn load_config_file() -> Value {
    let candidates = [
        "config/aethercore.defaults.cjson",
        "../config/aethercore.defaults.cjson",
        "../../config/aethercore.defaults.cjson",
    ];
    for p in &candidates {
        if let Ok(raw) = std::fs::read_to_string(p) {
            if let Ok(v) = serde_json::from_str::<Value>(&raw) {
                return v;
            }
        }
    }
    json!({})
}
