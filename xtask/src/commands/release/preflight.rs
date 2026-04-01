use anyhow::Result;

use crate::cli::ReleaseAction;
use crate::utils::{cargo, process};
use crate::commands::infra;
use crate::commands::ops;
use crate::commands::validation;

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

fn preflight(skip_host_tests: bool, skip_boot_artifacts: bool) -> Result<()> {
    println!("[release::preflight] Starting release preflight validation (native)");

    println!("[release::preflight] Step 1: Toolchain information");
    process::run_checked("rustc", &["-vV"])?;
    process::run_checked("cargo", &["-V"])?;

    println!("[release::preflight] Step 2: Clean check (all targets)");
    cargo::cargo(&["check", "--all-targets"])?;

    println!("[release::preflight] Step 3: Release build profile check");
    cargo::cargo(&["build", "--release"])?;

    if !skip_boot_artifacts {
        println!("[release::preflight] Step 4: Full boot artifact build validation");
        infra::build::execute(&crate::cli::BuildAction::Full {
            arch: "x86_64".to_string(),
            bootloader: crate::cli::Bootloader::Limine,
            format: crate::cli::ImageFormat::Iso,
            release: false,
        })?;
    } else {
        println!("[release::preflight] Step 4: Skipped (--skip-boot-artifacts)");
    }

    if !skip_host_tests {
        println!("[release::preflight] Step 5: Host tests feature matrix");
        validation::test::execute(&crate::cli::TestAction::Host { release: false })?;
    } else {
        println!("[release::preflight] Step 5: Skipped (--skip-host-tests)");
    }

    println!("[release::preflight] Step 6: Linux syscall coverage gate (default)");
    validation::syscall_coverage::execute(
        false, 
        "md", 
        &Some("reports/syscall_coverage.md".to_string())
    )?;

    println!("[release::preflight] Step 7: linux_compat profile compile + syscall gate");
    cargo::cargo(&["check", "--features", "linux_compat,posix_deep_tests"])?;
    validation::syscall_coverage::execute(
        true, 
        "md", 
        &Some("reports/syscall_coverage_linux_compat.md".to_string())
    )?;

    println!("[release::preflight] Step 8: POSIX deep tests compile gate");
    validation::test::execute(&crate::cli::TestAction::PosixConformance)?;

    println!("[release::preflight] PASS - Release preflight completed successfully.");
    Ok(())
}

fn p0_gate() -> Result<()> {
    println!("[release::p0-gate] Running native P0 readiness gate");
    preflight(false, false)?;
    validation::linux_abi::execute(&crate::cli::LinuxAbiAction::ErrnoConformance)?;
    validation::linux_abi::execute(&crate::cli::LinuxAbiAction::ShimErrnoConformance)?;
    validation::linux_abi::execute(&crate::cli::LinuxAbiAction::GapInventory)?;
    validation::syscall_coverage::execute(
        true,
        "md",
        &Some("reports/syscall_coverage/summary.md".to_string()),
    )?;
    validation::linux_abi::execute(&crate::cli::LinuxAbiAction::ReadinessScore)?;
    println!("[release::p0-gate] PASS");
    Ok(())
}

fn p0_acceptance() -> Result<()> {
    println!("[release::p0-acceptance] Running native P0 release acceptance");
    p0_gate()?;
    ops::soak::execute(false)?;
    println!("[release::p0-acceptance] PASS");
    Ok(())
}

fn p1_nightly() -> Result<()> {
    println!("[release::p1-nightly] Running native P1 nightly pipeline");
    p0_gate()?;
    ops::soak::execute(false)?;
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
    validation::linux_abi::execute(&crate::cli::LinuxAbiAction::P2GapReport)?;
    validation::linux_abi::execute(&crate::cli::LinuxAbiAction::P2GapGate)?;
    crate::commands::ops::archive::execute(&None)?;
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
