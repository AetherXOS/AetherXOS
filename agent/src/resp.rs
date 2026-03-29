use chrono::Utc;
use rocket::serde::json::{json, Value};

/// Helper — successful JSON response.
pub fn ok(message: impl Into<String>, data: Value) -> Value {
    let mut v = json!({
        "ok": true,
        "code": "ok",
        "message": message.into(),
        "ts_utc": Utc::now().to_rfc3339(),
    });
    if let (Some(obj), Some(data_obj)) = (v.as_object_mut(), data.as_object()) {
        for (k, val) in data_obj {
            obj.insert(k.clone(), val.clone());
        }
    }
    v
}

/// Helper — error JSON response.
pub fn err(
    code: impl Into<String>,
    message: impl Into<String>,
    error: impl Into<String>,
) -> Value {
    json!({
        "ok": false,
        "code": code.into(),
        "message": message.into(),
        "error": error.into(),
        "ts_utc": Utc::now().to_rfc3339(),
    })
}

pub fn err_detail(
    code: impl Into<String>,
    message: impl Into<String>,
    error: impl Into<String>,
    details: Value,
) -> Value {
    json!({
        "ok": false,
        "code": code.into(),
        "message": message.into(),
        "error": error.into(),
        "details": details,
        "ts_utc": Utc::now().to_rfc3339(),
    })
}
