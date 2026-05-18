use crate::utils::fs::paths::LAYOUT;
use crate::utils::logging;
use anyhow::Result;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::sync::Mutex;
use std::time::Duration;

/// Central registry for build metrics and performance trends.
static REGISTRY: Lazy<Mutex<TelemetryRegistry>> =
    Lazy::new(|| Mutex::new(TelemetryRegistry::default()));

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
        logging::error(
            "TELEMETRY",
            &format!("Task '{}' failed: {}", name, error),
            &[],
        );
        // In a real system, we'd send this to a database or SSE stream
    }

    /// Retrieve all recorded timings for the current session.
    pub fn session_snapshot() -> HashMap<String, Duration> {
        REGISTRY
            .lock()
            .map(|reg| reg.timings.clone())
            .unwrap_or_default()
    }

    fn save_trend(name: &str, duration: Duration) -> Result<()> {
        let path = LAYOUT
            .root
            .join(".xtask")
            .join("metrics")
            .join(format!("{}.trend", name));

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut f = fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)?;

        writeln!(f, "{}", duration.as_secs_f32())?;
        Ok(())
    }
}
