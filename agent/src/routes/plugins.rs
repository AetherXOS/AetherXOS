use chrono::Utc;
use rocket::serde::json::{json, Value};
use crate::auth::RequireViewer;
use crate::resp::{err, ok};

/// GET /crash/summary
#[rocket::get("/crash/summary")]
pub fn crash_summary(_role: RequireViewer) -> Value {
    // Read crash artifacts directory if it exists
    let crash_count = count_crash_artifacts();
    ok("crash_summary", json!({
        "crash_count": crash_count,
        "artifacts_dir": "artifacts/crash",
        "as_of_utc": Utc::now().to_rfc3339(),
    }))
}

/// GET /plugins/health
#[rocket::get("/plugins/health")]
pub fn plugins_health(_role: RequireViewer) -> Value {
    let plugins = scan_plugins();
    ok("plugins_health", json!({
        "plugins": plugins,
        "as_of_utc": Utc::now().to_rfc3339(),
    }))
}

#[rocket::get("/plugins")]
pub fn plugins_list(_role: RequireViewer) -> Value {
    let plugins = scan_plugins();
    ok("plugins", json!({ "plugins": plugins, "total": plugins.len() }))
}

#[rocket::get("/plugins/<name>")]
pub fn plugin_detail(name: String, _role: RequireViewer) -> Value {
    let plugins = scan_plugins();
    match plugins.into_iter().find(|plugin| plugin.get("name").and_then(|value| value.as_str()) == Some(name.as_str())) {
        Some(plugin) => ok("plugin", json!({ "plugin": plugin })),
        None => err("not_found", "Plugin not found.", "not_found"),
    }
}

fn count_crash_artifacts() -> usize {
    let candidates = ["artifacts/crash", "../artifacts/crash", "../../artifacts/crash"];
    for dir in &candidates {
        if let Ok(entries) = std::fs::read_dir(dir) {
            return entries.filter_map(|e| e.ok()).count();
        }
    }
    0
}

fn scan_plugins() -> Vec<Value> {
    let candidates = ["config/plugins", "../config/plugins", "../../config/plugins"];
    for dir in &candidates {
        if let Ok(entries) = std::fs::read_dir(dir) {
            let discovered = entries
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    if name.ends_with(".json") {
                        let path = e.path();
                        let content = std::fs::read_to_string(&path).ok()?;
                        let manifest = serde_json::from_str::<Value>(&content).ok()?;
                        let plugin_name = manifest
                            .get("name")
                            .and_then(|v| v.as_str())
                            .map(|v| v.to_string())
                            .or_else(|| name.rsplit_once('.').map(|(base, _)| base.to_string()))
                            .unwrap_or(name.clone());
                        let capabilities = manifest
                            .get("capabilities")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                    .collect::<Vec<String>>()
                            })
                            .filter(|arr| !arr.is_empty())
                            .unwrap_or_else(|| infer_plugin_capabilities(&plugin_name));

                        Some(json!({
                            "name": plugin_name,
                            "file_name": name,
                            "path": path.display().to_string(),
                            "status": "discovered",
                            "runtime": manifest.get("runtime").and_then(|v| v.as_str()).unwrap_or("xtask"),
                            "capabilities": capabilities,
                        }))
                    } else {
                        None
                    }
                })
                .collect::<Vec<Value>>();
            if !discovered.is_empty() {
                return discovered;
            }
        }
    }
    vec![json!({
        "name": "native-xtask",
        "file_name": "builtin",
        "path": "xtask",
        "status": "builtin",
        "runtime": "xtask",
        "capabilities": ["build", "run", "validate", "release", "dashboard"]
    })]
}

fn infer_plugin_capabilities(plugin_name: &str) -> Vec<String> {
    let lower = plugin_name.to_lowercase();
    let mut capabilities = vec!["run".to_string()];
    if lower.contains("secureboot") {
        capabilities.push("secureboot".to_string());
    }
    if lower.contains("report") || lower.contains("diagnostic") {
        capabilities.push("report".to_string());
    }
    if lower.contains("gate") {
        capabilities.push("gate".to_string());
    }
    if lower.contains("sign") {
        capabilities.push("sign".to_string());
    }
    capabilities
}
