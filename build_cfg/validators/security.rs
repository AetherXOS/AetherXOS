//! Security config validation — ring level, monitors, labels.

use crate::build_cfg::config_types::SecurityConfig;

const VALID_RING_LEVELS: &[&str] = &["Ring0", "Ring3"];
const VALID_MONITORS: &[&str] = &[
    "NullMonitor",
    "AccessControlList",
    "ObjectCapability",
    "SeL4_Style",
];

pub fn validate(c: &SecurityConfig) -> Vec<String> {
    let mut e = Vec::new();

    if !VALID_RING_LEVELS.contains(&c.ring_level.as_str()) {
        e.push(format!(
            "security.ring_level '{}' invalid, expected one of {:?}",
            c.ring_level, VALID_RING_LEVELS
        ));
    }
    if !VALID_MONITORS.contains(&c.monitor.as_str()) {
        e.push(format!(
            "security.monitor '{}' invalid, expected one of {:?}",
            c.monitor, VALID_MONITORS
        ));
    }
    if c.max_security_labels == 0 || c.max_security_labels > 65536 {
        e.push(format!(
            "security.max_security_labels {} out of range [1, 65536]",
            c.max_security_labels
        ));
    }
    if c.max_capability_tokens == 0 || c.max_capability_tokens > 1048576 {
        e.push(format!(
            "security.max_capability_tokens {} out of range [1, 1048576]",
            c.max_capability_tokens
        ));
    }
    if c.zero_trust_mode && c.monitor == "NullMonitor" {
        e.push("security.zero_trust_mode=true is incompatible with NullMonitor".to_string());
    }

    e
}
