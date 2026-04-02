//! Loads config from Cargo.toml [package.metadata.aethercore.config].

use super::config_types::*;
use std::fs;
use std::path::Path;

pub fn load_config_from_manifest() -> Config {
    let manifest_path = Path::new("Cargo.toml");
    let manifest_content = fs::read_to_string(manifest_path)
        .unwrap_or_else(|e| panic!("Failed to read '{}': {}", manifest_path.display(), e));
    let manifest: CargoManifest = toml::from_str(&manifest_content)
        .unwrap_or_else(|e| panic!("Failed to parse '{}': {}", manifest_path.display(), e));

    manifest
        .package
        .metadata
        .aethercore
        .config
        .unwrap_or_else(|| {
            panic!(
                "Missing [package.metadata.aethercore.config] in Cargo.toml. \
                 Move kernel config there to continue."
            )
        })
}
