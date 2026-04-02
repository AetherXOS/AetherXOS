use crate::auth::{RequireAdmin, RequireViewer};
use crate::models::{PolicyTrace, RolePolicy, default_role_policies};
use crate::resp::{err, ok};
use crate::state::AppState;
use chrono::Utc;
use rocket::State;
use rocket::serde::json::{Json, Value, json};
use serde::Deserialize;

// ── GET /policy ───────────────────────────────────────────────────────────────

#[rocket::get("/policy")]
pub fn get_policy(state: &State<AppState>, _role: RequireViewer) -> Value {
    let inner = state.read();
    ok("policy", json!({ "policy": policy_snapshot(&inner) }))
}

// ── GET /policy/template ──────────────────────────────────────────────────────

#[rocket::get("/policy/template")]
pub fn policy_template(_role: RequireViewer) -> Value {
    let defaults = default_role_policies();
    let rows: Vec<Value> = ["viewer", "operator", "admin"]
        .iter()
        .map(|r| {
            let p = &defaults[*r];
            json!({
                "role": r,
                "max_risk": p.max_risk,
                "denied_actions": p.denied_actions,
                "denied_categories": p.denied_categories,
            })
        })
        .collect();
    ok("policy_template", json!({ "roles": rows }))
}

// ── POST /policy/apply ────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct PolicyApplyPayload {
    pub roles: Value,
}

#[rocket::post("/policy/apply", data = "<body>")]
pub fn policy_apply(
    state: &State<AppState>,
    _role: RequireAdmin,
    body: Json<PolicyApplyPayload>,
) -> Value {
    let parsed = parse_roles_input(&body.roles);
    if parsed.is_empty() {
        return err(
            "invalid_payload",
            "roles must be an object or array of role policies.",
            "invalid_payload",
        );
    }

    {
        let mut inner = state.write();
        for (role, rp) in &parsed {
            inner.role_policies.insert(role.clone(), rp.clone());
        }
        inner.push_policy_trace(PolicyTrace {
            ts_utc: Utc::now(),
            source: "api".into(),
            role: "admin".into(),
            action: "policy_apply".into(),
            category: "policy".into(),
            risk: "INFO".into(),
            allowed: true,
            reason: "policy_updated".into(),
        });
    }

    let inner = state.read();
    ok(
        "policy_applied",
        json!({ "policy": policy_snapshot(&inner) }),
    )
}

// ── POST /policy/validate ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct PolicyValidatePayload {
    pub roles: Value,
}

#[rocket::post("/policy/validate", data = "<body>")]
pub fn policy_validate(_role: RequireViewer, body: Json<PolicyValidatePayload>) -> Value {
    let parsed = parse_roles_input(&body.roles);
    let valid = !parsed.is_empty();
    ok(
        "policy_validated",
        json!({ "valid": valid, "role_count": parsed.len() }),
    )
}

// ── POST /policy/reset ────────────────────────────────────────────────────────

#[rocket::post("/policy/reset")]
pub fn policy_reset(state: &State<AppState>, _role: RequireAdmin) -> Value {
    let mut inner = state.write();
    inner.role_policies = default_role_policies();
    inner.push_policy_trace(PolicyTrace {
        ts_utc: Utc::now(),
        source: "api".into(),
        role: "admin".into(),
        action: "policy_reset".into(),
        category: "policy".into(),
        risk: "INFO".into(),
        allowed: true,
        reason: "policy_reset_to_defaults".into(),
    });
    ok("policy_reset", json!({ "policy": policy_snapshot(&inner) }))
}

// ── POST /policy/simulate ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SimulatePayload {
    pub role: String,
    pub action: String,
}

#[rocket::post("/policy/simulate", data = "<body>")]
pub fn policy_simulate(
    state: &State<AppState>,
    _role: RequireViewer,
    body: Json<SimulatePayload>,
) -> Value {
    let inner = state.read();
    let result = inner.check_policy(&body.role, &body.action);
    let (allowed, reason) = match result {
        Ok(_) => (true, "allowed"),
        Err(ref r) => (false, r.as_str()),
    };
    ok(
        "policy_simulated",
        json!({
            "role": body.role,
            "action": body.action,
            "allowed": allowed,
            "reason": reason,
        }),
    )
}

// ── GET /policy/traces ────────────────────────────────────────────────────────

#[rocket::get("/policy/traces")]
pub fn policy_traces(state: &State<AppState>, _role: RequireViewer) -> Value {
    let inner = state.read();
    let traces: Vec<Value> = inner
        .policy_traces
        .iter()
        .rev()
        .take(100)
        .map(|t| {
            json!({
                "ts_utc": t.ts_utc.to_rfc3339(),
                "source": t.source,
                "role": t.role,
                "action": t.action,
                "category": t.category,
                "risk": t.risk,
                "allowed": t.allowed,
                "reason": t.reason,
            })
        })
        .collect();
    ok(
        "policy_traces",
        json!({ "traces": traces, "total": inner.policy_traces.len() }),
    )
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn policy_snapshot(inner: &crate::state::Inner) -> Value {
    let rows: Vec<Value> = ["viewer", "operator", "admin"]
        .iter()
        .map(|r| {
            let p = inner
                .role_policies
                .get(*r)
                .cloned()
                .unwrap_or_else(|| RolePolicy::default_for(r));
            json!({
                "role": r,
                "max_risk": p.max_risk,
                "denied_actions": p.denied_actions,
                "denied_categories": p.denied_categories,
            })
        })
        .collect();
    json!({ "roles": rows })
}

fn parse_roles_input(value: &Value) -> std::collections::HashMap<String, RolePolicy> {
    let mut out = std::collections::HashMap::new();
    let valid_roles = ["viewer", "operator", "admin"];

    if let Some(arr) = value.as_array() {
        for item in arr {
            let role = item["role"].as_str().unwrap_or("").to_string();
            if !valid_roles.contains(&role.as_str()) {
                continue;
            }
            let rp = parse_role_policy(item, &role);
            out.insert(role, rp);
        }
    } else if let Some(obj) = value.as_object() {
        for role in &valid_roles {
            if let Some(rp_val) = obj.get(*role) {
                let rp = parse_role_policy(rp_val, role);
                out.insert(role.to_string(), rp);
            }
        }
    }
    out
}

fn parse_role_policy(v: &Value, role: &str) -> RolePolicy {
    let def = RolePolicy::default_for(role);
    RolePolicy {
        max_risk: v["max_risk"].as_str().unwrap_or(&def.max_risk).to_string(),
        denied_actions: v["denied_actions"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|s| s.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or(def.denied_actions),
        denied_categories: v["denied_categories"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|s| s.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or(def.denied_categories),
    }
}
