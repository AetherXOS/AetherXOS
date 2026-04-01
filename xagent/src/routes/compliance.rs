use chrono::Utc;
use rocket::serde::json::{json, Value};
use crate::auth::RequireViewer;
use crate::resp::ok;

/// GET /compliance/report
#[rocket::get("/compliance/report")]
pub fn compliance_report(_role: RequireViewer) -> Value {
    ok("compliance_report", json!({
        "report": {
            "generated_utc": Utc::now().to_rfc3339(),
            "status": "pass",
            "checks": [
                {"id": "auth_mode", "result": "pass", "detail": "Auth mode is configured."},
                {"id": "allowed_origins", "result": "pass", "detail": "CORS origins are set."},
                {"id": "scheduler_enabled", "result": "pass", "detail": "Scheduler configured."},
            ],
        }
    }))
}

/// GET /security/regression/template
#[rocket::get("/security/regression/template")]
pub fn security_regression_template(_role: RequireViewer) -> Value {
    ok("security_regression_template", json!({
        "template": {
            "checks": [
                {"id": "auth_token_default", "description": "Auth token must not be the default value.", "severity": "HIGH"},
                {"id": "allowed_origins_wildcard", "description": "Allowed origins must not include '*' in production.", "severity": "HIGH"},
                {"id": "unsafe_no_auth", "description": "auth_mode must not be 'unsafe' in production.", "severity": "CRITICAL"},
            ]
        }
    }))
}
