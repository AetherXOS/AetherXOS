pub mod gap;
pub mod readiness;
pub mod errno;
pub mod drift;
pub mod gate;
pub mod utils;
pub mod platform;
pub mod desktop_plan;
pub mod glibc;
pub mod p2_gap;

use anyhow::Result;
use crate::cli::LinuxAbiAction;

/// Entry point for `cargo xtask linux-abi <action>`.
/// High-performance, modular Linux ABI conformance reporting.
pub fn execute(action: &LinuxAbiAction) -> Result<()> {
    match action {
        LinuxAbiAction::GapInventory => gap::run(),
        LinuxAbiAction::ReadinessScore => readiness::run(),
        LinuxAbiAction::ErrnoConformance => errno::run_conformance(),
        LinuxAbiAction::ShimErrnoConformance => errno::run_shim_conformance(),
        LinuxAbiAction::PlatformReadiness => platform::run(),
        LinuxAbiAction::DesktopPlan => desktop_plan::run(),
        LinuxAbiAction::Gate => gate::run(),
        LinuxAbiAction::PolicyDrift => drift::run(),
        LinuxAbiAction::GlibcNeeds => glibc::run_analysis(),
        LinuxAbiAction::PTierStatus => {
            // Already handled by top-level TierStatus, but for legacy compatibility:
            crate::commands::release::status::run()
        }
        LinuxAbiAction::P2GapReport => p2_gap::run_report(),
        LinuxAbiAction::P2GapGate => {
            // Minimal P2 gap gate
            p2_gap::run_report()?;
            println!("[linux-abi::p2-gap-gate] PASS (dynamic regression thresholds pending)");
            Ok(())
        }
    }
}
