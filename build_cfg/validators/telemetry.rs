//! Telemetry config validation — log level enum, history bounds.

use crate::build_cfg::config_types::TelemetryConfig;

const VALID_LOG_LEVELS: &[&str] = &["Error", "Warn", "Info", "Debug", "Trace"];

pub fn validate(c: &TelemetryConfig) -> Vec<String> {
    let mut e = Vec::new();

    if !VALID_LOG_LEVELS.contains(&c.log_level.as_str()) {
        e.push(format!(
            "telemetry.log_level '{}' invalid, expected one of {:?}",
            c.log_level, VALID_LOG_LEVELS
        ));
    }
    if c.history_len == 0 || c.history_len > 100_000 {
        e.push(format!(
            "telemetry.history_len {} out of range [1, 100000]",
            c.history_len
        ));
    }

    e
}
