use anyhow::{Context, Result};
use serde::Serialize;
use std::path::Path;

/// Write a JSON report to disk, creating parent directories as needed.
#[allow(dead_code)]
pub fn write_json_report<T: Serialize>(path: &Path, data: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create report directory: {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(data)
        .context("Failed to serialize report to JSON")?;
    std::fs::write(path, json)
        .with_context(|| format!("Failed to write report: {}", path.display()))?;
    println!("[report] Written: {}", path.display());
    Ok(())
}

/// Returns the current UTC timestamp as an ISO-8601 string.
#[allow(dead_code)]
pub fn utc_now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}
