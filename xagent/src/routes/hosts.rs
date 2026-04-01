use rocket::serde::json::{json, Json, Value};
use rocket::State;
use serde::Deserialize;
use crate::auth::{RequireAdmin, RequireViewer};
use crate::models::Host;
use crate::resp::{err, err_detail, ok};
use crate::state::AppState;

// ── GET /hosts ────────────────────────────────────────────────────────────────

#[rocket::get("/hosts")]
pub fn list_hosts(state: &State<AppState>, _role: RequireViewer) -> Value {
    let inner = state.read();
    let hosts: Vec<Value> = inner.hosts.iter().map(host_to_json).collect();
    ok("hosts", json!({ "hosts": hosts }))
}

// ── POST /hosts/register ──────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct HostPayload {
    pub id: String,
    pub name: Option<String>,
    pub url: String,
    pub enabled: Option<bool>,
    pub role_hint: Option<String>,
    pub token: Option<String>,
    pub capabilities: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeartbeatPayload {
    pub id: String,
    pub reachable: Option<bool>,
    pub capabilities: Option<Vec<String>>,
}

#[rocket::post("/hosts/register", data = "<body>")]
pub fn hosts_register(state: &State<AppState>, _role: RequireAdmin, body: Json<HostPayload>) -> Value {
    if body.id.is_empty() || body.url.is_empty() {
        return err("invalid_payload", "id and url are required.", "invalid_payload");
    }
    if body.id == "local" {
        return err("reserved_id", "'local' is a reserved host id.", "conflict");
    }

    let host = Host {
        id: body.id.clone(),
        name: body.name.clone().unwrap_or_else(|| body.id.clone()),
        url: body.url.clone(),
        enabled: body.enabled.unwrap_or(true),
        role_hint: body.role_hint.clone().unwrap_or_else(|| "operator".into()),
        token: body.token.clone().unwrap_or_default(),
        last_seen_utc: None,
        reachable: None,
        capabilities: body.capabilities.clone().unwrap_or_default(),
    };

    let mut inner = state.write();
    // Prevent duplicate
    if inner.hosts.iter().any(|h| h.id == host.id) {
        return err_detail("already_exists", "Host ID already registered.", "conflict", json!({ "id": host.id }));
    }
    inner.hosts.push(host.clone());
    ok("host_registered", json!({ "host": host_to_json(&host) }))
}

// ── POST /hosts/update ────────────────────────────────────────────────────────

#[rocket::post("/hosts/update", data = "<body>")]
pub fn hosts_update(state: &State<AppState>, _role: RequireAdmin, body: Json<HostPayload>) -> Value {
    if body.id.is_empty() {
        return err("invalid_payload", "id is required.", "invalid_payload");
    }
    if body.id == "local" {
        return err("reserved_id", "'local' host cannot be modified.", "conflict");
    }

    let mut inner = state.write();
    match inner.hosts.iter_mut().find(|h| h.id == body.id) {
        None => err_detail("not_found", "Host not found.", "not_found", json!({ "id": body.id })),
        Some(h) => {
            if !body.url.is_empty() { h.url = body.url.clone(); }
            if let Some(n) = &body.name { h.name = n.clone(); }
            if let Some(e) = body.enabled { h.enabled = e; }
            if let Some(r) = &body.role_hint { h.role_hint = r.clone(); }
            if let Some(t) = &body.token { h.token = t.clone(); }
            if let Some(caps) = &body.capabilities { h.capabilities = caps.clone(); }
            ok("host_updated", json!({ "host": host_to_json(h) }))
        }
    }
}

// ── POST /hosts/remove ────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RemovePayload { pub id: String }

#[rocket::post("/hosts/remove", data = "<body>")]
pub fn hosts_remove(state: &State<AppState>, _role: RequireAdmin, body: Json<RemovePayload>) -> Value {
    if body.id == "local" {
        return err("reserved_id", "'local' host cannot be removed.", "conflict");
    }
    let mut inner = state.write();
    let before = inner.hosts.len();
    inner.hosts.retain(|h| h.id != body.id);
    if inner.hosts.len() < before {
        ok("host_removed", json!({ "id": body.id }))
    } else {
        err_detail("not_found", "Host not found.", "not_found", json!({ "id": body.id }))
    }
}

// ── GET /status/hosts ─────────────────────────────────────────────────────────

#[rocket::get("/status/hosts")]
pub fn status_hosts(state: &State<AppState>, _role: RequireViewer) -> Value {
    let inner = state.read();
    let hosts: Vec<Value> = inner.hosts.iter().map(|h| json!({
        "id": h.id,
        "name": h.name,
        "url": h.url,
        "enabled": h.enabled,
        "role_hint": h.role_hint,
        "reachable": h.reachable,
        "last_seen_utc": h.last_seen_utc.map(|d| d.to_rfc3339()),
        "capabilities": h.capabilities,
    })).collect();
    ok("status_hosts", json!({ "hosts": hosts }))
}

#[rocket::post("/hosts/heartbeat", data = "<body>")]
pub fn hosts_heartbeat(state: &State<AppState>, _role: RequireAdmin, body: Json<HeartbeatPayload>) -> Value {
    let mut inner = state.write();
    match inner.hosts.iter_mut().find(|host| host.id == body.id) {
        Some(host) => {
            host.last_seen_utc = Some(chrono::Utc::now());
            if let Some(reachable) = body.reachable {
                host.reachable = Some(reachable);
            }
            if let Some(capabilities) = &body.capabilities {
                host.capabilities = capabilities.clone();
            }
            ok("host_heartbeat", json!({ "host": host_to_json(host) }))
        }
        None => err_detail("not_found", "Host not found.", "not_found", json!({ "id": body.id })),
    }
}

fn host_to_json(h: &Host) -> Value {
    json!({
        "id": h.id,
        "name": h.name,
        "url": h.url,
        "enabled": h.enabled,
        "role_hint": h.role_hint,
        "token_set": !h.token.is_empty(),
        "last_seen_utc": h.last_seen_utc.map(|d| d.to_rfc3339()),
        "reachable": h.reachable,
        "capabilities": h.capabilities,
    })
}
