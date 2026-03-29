//! Modular build configuration system for HyperCore OS.
//!
//! Each subsystem owns its own validation, range constants, and feature rules.
//! build.rs delegates to this module instead of being a monolithic god-file.

mod cfg_emitter;
mod config_loader;
mod config_types;
mod emitter;
mod feature_graph;
mod runtime_codegen;
mod validators;

use std::fs;
use std::path::Path;

pub use config_loader::load_config_from_manifest;

/// Main entry point called from build.rs.
pub fn run() {
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=src/config.rs");
    println!("cargo:rerun-if-changed=src/config");

    let config = load_config_from_manifest();

    // Phase 1: Validate all subsystem configs (each subsystem validates itself)
    validators::validate_all(&config);

    // Phase 2: Generate runtime key autogen
    runtime_codegen::generate_runtime_key_autogen();

    // Phase 3: Emit generated_consts.rs
    let dest_path = Path::new("src/generated_consts.rs");
    let content = emitter::emit_all_consts(&config);
    fs::write(dest_path, content).expect("Failed to write generated_consts.rs");

    // Phase 4: Emit rustc-cfg flags
    cfg_emitter::emit_check_cfgs();
    cfg_emitter::emit_compile_cfgs(&config);
}
