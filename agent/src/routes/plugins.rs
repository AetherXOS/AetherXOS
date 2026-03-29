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
    let candidates = ["scripts/plugins", "../scripts/plugins", "../../scripts/plugins"];
    for dir in &candidates {
        if let Ok(entries) = std::fs::read_dir(dir) {
            return entries
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    if name.ends_with(".ps1") || name.ends_with(".py") {
                        let runtime = if name.ends_with(".ps1") { "powershell" } else { "python" };
                        let plugin_name = name.rsplit_once('.').map(|(base, _)| base.to_string()).unwrap_or_else(|| name.clone());
                        Some(json!({
                            "name": plugin_name,
                            "file_name": name,
                            "path": e.path().display().to_string(),
                            "status": "discovered",
                            "runtime": runtime,
                            "capabilities": infer_plugin_capabilities(&plugin_name),
                        }))
                    } else {
                        None
                    }
                })
                .collect();
        }
    }
    vec![]
}

fn infer_plugin_capabilities(plugin_name: &str) -> Vec<&'static str> {
    let lower = plugin_name.to_lowercase();
    let mut capabilities = vec!["run"];
    if lower.contains("secureboot") {
        capabilities.push("secureboot");
    }
    if lower.contains("report") || lower.contains("diagnostic") {
        capabilities.push("report");
    }
    if lower.contains("gate") {
        capabilities.push("gate");
    }
    if lower.contains("sign") {
        capabilities.push("sign");
    }
    capabilities
}
