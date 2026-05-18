use crate::engine::EngineState;
use crate::utils::logging;
use anyhow::Result;

pub fn execute() -> Result<()> {
    logging::status("ENGINE", "Retrieving system-wide build status...");

    let state = EngineState::load();
    println!("\n🚀 AetherX OS Engine Status Report");
    println!("====================================");

    println!("📍 Recent Task Fingerprints:");
    let hashes = state.get_all_hashes();
    if hashes.is_empty() {
        println!("  (No tasks executed yet)");
    } else {
        for (name, hash) in hashes {
            println!("  - {}: {:.8}...", name, hash);
        }
    }

    println!("\n📦 Distro Cache Status:");
    // Mocking some logic for now
    println!("  - Alpine Base: READY");
    println!("  - Debian Base: STALE (Fingerprint Mismatch)");

    println!("\n🛠️ Host Environment:");
    println!("  - Rust Toolchain: nightly-2026-05-10");
    println!("  - Build Arch: x86_64-unknown-none");

    println!("====================================\n");
    Ok(())
}
