//! Library boundary config validation — mode enum, libnet pump budget.

use crate::build_cfg::config_types::LibraryConfig;

const VALID_BOUNDARY_MODES: &[&str] = &["Strict", "Balanced", "Compat"];
const VALID_FAST_PATH_STRATEGIES: &[&str] = &["Adaptive", "Aggressive", "Conservative"];

pub fn validate(c: &LibraryConfig) -> Vec<String> {
    let mut e = Vec::new();

    if !VALID_BOUNDARY_MODES.contains(&c.boundary_mode.as_str()) {
        e.push(format!(
            "library.boundary_mode '{}' invalid, expected one of {:?}",
            c.boundary_mode, VALID_BOUNDARY_MODES
        ));
    }
    if !VALID_FAST_PATH_STRATEGIES.contains(&c.libnet_fast_path_default_strategy.as_str()) {
        e.push(format!(
            "library.libnet_fast_path_default_strategy '{}' invalid, expected one of {:?}",
            c.libnet_fast_path_default_strategy, VALID_FAST_PATH_STRATEGIES
        ));
    }
    if c.libnet_fast_path_pump_budget == 0 || c.libnet_fast_path_pump_budget > 4096 {
        e.push(format!(
            "library.libnet_fast_path_pump_budget {} out of range [1, 4096]",
            c.libnet_fast_path_pump_budget
        ));
    }
    if c.max_services == 0 || c.max_services > 65536 {
        e.push(format!(
            "library.max_services {} out of range [1, 65536]",
            c.max_services
        ));
    }

    e
}
