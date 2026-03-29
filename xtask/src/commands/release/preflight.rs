use anyhow::Result;

use crate::cli::ReleaseAction;
use crate::utils::{cargo, process};
use crate::commands::infra;
use crate::commands::ops;
use crate::commands::validation;

/// Entry point for `cargo xtask release <action>`.
///
/// Replaces: release_preflight.ps1, release_candidate_gate.ps1,
///           p0_readiness_gate.ps1, p0_release_acceptance.ps1,
///           p1_nightly.ps1, p1_release_acceptance.ps1, p0_p1_nightly.ps1
pub fn execute(action: &ReleaseAction) -> Result<()> {
    match action {
        ReleaseAction::Preflight { skip_host_tests, skip_boot_artifacts } => {
            preflight(*skip_host_tests, *skip_boot_artifacts)
        }
        ReleaseAction::CandidateGate => {
            candidate_gate()
        }
        ReleaseAction::P0Gate => {
            p0_gate()
        }
        ReleaseAction::P0Acceptance => {
            p0_acceptance()
        }
        ReleaseAction::P1Nightly => {
            p1_nightly()
        }
        ReleaseAction::P1Acceptance => {
            p1_acceptance()
        }
        ReleaseAction::P0P1Nightly => {
            p0_p1_nightly()
        }
    }
}

/// Full release preflight validation. Replaces scripts/release_preflight.ps1
fn preflight(skip_host_tests: bool, skip_boot_artifacts: bool) -> Result<()> {
    println!("[release::preflight] Starting release preflight validation (native)");

    // Step 1: Toolchain info
    println!("[release::preflight] Step 1: Toolchain information");
    process::run_checked("rustc", &["-vV"])?;
    process::run_checked("cargo", &["-V"])?;

    // Step 2: Clean check
    println!("[release::preflight] Step 2: Clean check (all targets)");
    cargo::cargo(&["check", "--all-targets"])?;

    // Step 3: Release build
    println!("[release::preflight] Step 3: Release build profile check");
    cargo::cargo(&["build", "--release"])?;

    // Step 4: Boot artifacts
    if !skip_boot_artifacts {
        println!("[release::preflight] Step 4: Full boot artifact build validation");
        infra::execute_build(&crate::cli::BuildAction::Full)?;
    } else {
        println!("[release::preflight] Step 4: Skipped (--skip-boot-artifacts)");
    }

    // Step 5: Host tests
    if !skip_host_tests {
        println!("[release::preflight] Step 5: Host tests feature matrix");
        validation::execute_test(&crate::cli::TestAction::Host { release: false })?;
    } else {
        println!("[release::preflight] Step 5: Skipped (--skip-host-tests)");
    }

    // Step 6: Syscall coverage gates
    println!("[release::preflight] Step 6: Linux syscall coverage gate (default)");
    validation::execute_syscall_coverage(
        false, 
        "md", 
        &Some("reports/syscall_coverage.md".to_string())
    )?;

    println!("[release::preflight] Step 7: linux_compat profile compile + syscall gate");
    cargo::cargo(&["check", "--features", "linux_compat,posix_deep_tests"])?;
    validation::execute_syscall_coverage(
        true, 
        "md", 
        &Some("reports/syscall_coverage_linux_compat.md".to_string())
    )?;

    // Step 8: POSIX deep tests compile gate
    println!("[release::preflight] Step 8: POSIX deep tests compile gate");
    validation::execute_test(&crate::cli::TestAction::PosixConformance)?;

    println!("[release::preflight] PASS - Release preflight completed successfully.");
    Ok(())
}

fn p0_gate() -> Result<()> {
    println!("[release::p0-gate] Running native P0 readiness gate");
    preflight(false, false)?;
    validation::execute_linux_abi(&crate::cli::LinuxAbiAction::ErrnoConformance)?;
    validation::execute_linux_abi(&crate::cli::LinuxAbiAction::ShimErrnoConformance)?;
    validation::execute_linux_abi(&crate::cli::LinuxAbiAction::GapInventory)?;
    validation::execute_syscall_coverage(
        true,
        "md",
        &Some("reports/syscall_coverage/summary.md".to_string()),
    )?;
    validation::execute_linux_abi(&crate::cli::LinuxAbiAction::ReadinessScore)?;
    println!("[release::p0-gate] PASS");
    Ok(())
}

fn p0_acceptance() -> Result<()> {
    println!("[release::p0-acceptance] Running native P0 release acceptance");
    p0_gate()?;
    ops::execute_soak(false)?;
    println!("[release::p0-acceptance] PASS");
    Ok(())
}

fn p1_nightly() -> Result<()> {
    println!("[release::p1-nightly] Running native P1 nightly pipeline");
    p0_gate()?;
    ops::execute_soak(false)?;
    crate::commands::release::status::run()?;
    println!("[release::p1-nightly] PASS");
    Ok(())
}

fn p1_acceptance() -> Result<()> {
    println!("[release::p1-acceptance] Running native P1 release acceptance");
    p1_nightly()?;
    println!("[release::p1-acceptance] PASS");
    Ok(())
}

fn p0_p1_nightly() -> Result<()> {
    println!("[release::p0-p1-nightly] Running native combined P0+P1 nightly pipeline");
    p0_acceptance()?;
    p1_nightly()?;
    validation::execute_linux_abi(&crate::cli::LinuxAbiAction::P2GapReport)?;
    validation::execute_linux_abi(&crate::cli::LinuxAbiAction::P2GapGate)?;
    crate::commands::ops::execute_archive(&None)?;
    crate::commands::release::status::run()?;
    println!("[release::p0-p1-nightly] PASS");
    Ok(())
}

fn candidate_gate() -> Result<()> {
    println!("[release::candidate-gate] Running native release candidate gate");
    p0_p1_nightly()?;
    println!("[release::candidate-gate] PASS");
    Ok(())
}
