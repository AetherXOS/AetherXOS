//! Orchestrates all subsystem validators.
//! Each subsystem owns its own ranges, rules, and feature coherence checks.

mod aarch64;
mod core_config;
mod drivers;
mod features;
mod governor;
mod ipc;
mod kernel;
mod library;
mod memory;
mod network;
mod scheduler;
mod security;
mod telemetry;
mod vfs;

use super::config_types::Config;

/// Run all subsystem validations. Panics with collected errors on failure.
pub fn validate_all(config: &Config) {
    let mut errors: Vec<String> = Vec::new();

    errors.extend(kernel::validate(&config.kernel));
    errors.extend(core_config::validate(&config.core));
    errors.extend(memory::validate(&config.memory));
    errors.extend(scheduler::validate(&config.scheduler));
    errors.extend(ipc::validate(&config.ipc));
    errors.extend(security::validate(&config.security));
    errors.extend(telemetry::validate(&config.telemetry));
    errors.extend(network::validate(&config.network));
    errors.extend(drivers::validate(&config.drivers));
    errors.extend(vfs::validate(&config.vfs));
    errors.extend(governor::validate(&config.governor));
    errors.extend(aarch64::validate(&config.aarch64));
    errors.extend(library::validate(&config.library));
    errors.extend(features::validate_feature_coherence(config));

    if !errors.is_empty() {
        for e in &errors {
            println!("cargo:warning=CONFIG ERROR: {}", e);
        }
        panic!(
            "\n\n========================================\n\
             Configuration validation failed with {} error(s).\n\
             Fix the issues above in Cargo.toml [package.metadata.aethercore.config.*]\n\
             ========================================\n",
            errors.len()
        );
    }
}
