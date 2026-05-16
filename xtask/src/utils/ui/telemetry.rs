use anyhow::Result;
use std::collections::HashMap;
use std::sync::Mutex;
use crate::utils::logging;
use std::time::Duration;
use once_cell::sync::Lazy;

/// Central registry for build metrics and performance trends.
static REGISTRY: Lazy<Mutex<TelemetryRegistry>> = Lazy::new(|| Mutex::new(TelemetryRegistry::default()));

#[derive(Default)]
struct TelemetryRegistry {
    timings: HashMap<String, Duration>,
}

pub struct Telemetry;

impl Telemetry {
    /// Record a duration for a specific build step.
    pub fn record(name: impl Into<String>, duration: Duration) {
        let name = name.into();
        if let Ok(mut reg) = REGISTRY.lock() {
            reg.timings.insert(name.clone(), duration);
        }
        let _ = Self::save_trend(&name, duration);
    }

    /// Record a task failure for telemetry.
    pub fn record_failure(name: &str, error: &str) {
        logging::error("TELEMETRY", &format!("Task '{}' failed: {}", name, error), &[]);
        // In a real system, we'd send this to a database or SSE stream
    }

    /// Retrieve all recorded timings for the current session.
    pub fn session_snapshot() -> HashMap<String, Duration> {
        REGISTRY.lock().map(|reg| reg.timings.clone()).unwrap_or_default()
    }

    fn save_trend(name: &str, duration: Duration) -> Result<()> {
        let path = crate::utils::paths::repo_root()
            .join(".xtask")
            .join("metrics")
            .join(format!("{}.trend", name));
            
        crate::utils::paths::ensure_dir(&path.parent().unwrap())?;
        
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)?;
            
        writeln!(f, "{}", duration.as_secs_f32())?;
        Ok(())
    }
}
