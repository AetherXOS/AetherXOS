use anyhow::Result;

use crate::cli::ReleaseAction;
use crate::commands::infra;
use crate::commands::ops;
use crate::commands::validation;
use crate::constants::{arch, cargo as cargo_consts, tools};
use crate::utils::logging;
use crate::utils::{cargo, process};

pub fn execute(action: &ReleaseAction) -> Result<()> {
    match action {
        ReleaseAction::Preflight {
            skip_host_tests,
            skip_boot_artifacts,
        } => preflight(*skip_host_tests, *skip_boot_artifacts),
        ReleaseAction::CandidateGate => candidate_gate(),
        ReleaseAction::P0Gate => p0_gate(),
        ReleaseAction::P0Acceptance => p0_acceptance(),
        ReleaseAction::P1Nightly => p1_nightly(),
        ReleaseAction::P1Acceptance => p1_acceptance(),
        ReleaseAction::P0P1Nightly => p0_p1_nightly(),
    }
}

fn preflight(skip_host_tests: bool, skip_boot_artifacts: bool) -> Result<()> {
    logging::info("release::preflight", "Starting release preflight validation", &[]);

    logging::info("release::preflight", "Step 1: Toolchain information", &[]);
    process::run_checked(tools::RUSTC, &["-vV"])?;
    process::run_checked(tools::CARGO, &["-V"])?;

    logging::info("release::preflight", "Step 2: Clean check (all targets)", &[]);
    cargo::cargo(&[cargo_consts::CMD_CHECK, "--all-targets"])?;

    logging::info("release::preflight", "Step 3: Release build profile check", &[]);
    cargo::cargo(&[cargo_consts::CMD_BUILD, cargo_consts::ARG_RELEASE])?;

    if !skip_boot_artifacts {
        logging::info("release::preflight", "Step 4: Full boot artifact build validation", &[]);
        infra::build::execute(&crate::cli::BuildAction::Full {
            arch: arch::X86_64.to_string(),
            bootloader: crate::cli::Bootloader::Limine,
            format: crate::cli::ImageFormat::Iso,
            release: false,
        })?;
    } else {
        logging::info("release::preflight", "Step 4: Skipped", &[("reason", "--skip-boot-artifacts")]);
    }

    if !skip_host_tests {
        logging::info("release::preflight", "Step 5: Host tests feature matrix", &[]);
        validation::test::execute(&crate::cli::TestAction::Host { release: false })?;
    } else {
        logging::info("release::preflight", "Step 5: Skipped", &[("reason", "--skip-host-tests")]);
    }

    logging::info("release::preflight", "Step 6: Linux syscall coverage gate", &[]);
    validation::syscall_coverage::execute(
        false,
        "md",
        &Some("reports/syscall_coverage.md".to_string()),
    )?;

    logging::info("release::preflight", "Step 7: linux_compat profile compile and syscall gate", &[]);
    cargo::cargo(&[
        cargo_consts::CMD_CHECK,
        cargo_consts::ARG_FEATURES,
        "linux_compat,posix_deep_tests",
    ])?;
    validation::syscall_coverage::execute(
        true,
        "md",
        &Some("reports/syscall_coverage_linux_compat.md".to_string()),
    )?;

    logging::info("release::preflight", "Step 8: POSIX deep tests compile gate", &[]);
    validation::test::execute(&crate::cli::TestAction::PosixConformance)?;

    logging::info("release::preflight", "Release preflight completed successfully", &[]);
    Ok(())
}

fn p0_gate() -> Result<()> {
    logging::info("release::p0_gate", "Running native P0 readiness gate", &[]);
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
    logging::info("release::p0_gate", "P0 readiness gate passed", &[]);
    Ok(())
}

fn p0_acceptance() -> Result<()> {
    logging::info("release::p0_acceptance", "Running native P0 release acceptance", &[]);
    p0_gate()?;
    ops::soak::execute(false)?;
    logging::info("release::p0_acceptance", "P0 release acceptance passed", &[]);
    Ok(())
}

fn p1_nightly() -> Result<()> {
    logging::info("release::p1_nightly", "Running native P1 nightly pipeline", &[]);
    p0_gate()?;
    ops::soak::execute(false)?;
    crate::commands::release::status::run()?;
    logging::info("release::p1_nightly", "P1 nightly pipeline passed", &[]);
    Ok(())
}

fn p1_acceptance() -> Result<()> {
    logging::info("release::p1_acceptance", "Running native P1 release acceptance", &[]);
    p1_nightly()?;
    logging::info("release::p1_acceptance", "P1 release acceptance passed", &[]);
    Ok(())
}

fn p0_p1_nightly() -> Result<()> {
    logging::info("release::p0_p1_nightly", "Running native combined P0+P1 nightly pipeline", &[]);
    p0_acceptance()?;
    p1_nightly()?;
    validation::linux_abi::execute(&crate::cli::LinuxAbiAction::P2GapReport)?;
    validation::linux_abi::execute(&crate::cli::LinuxAbiAction::P2GapGate)?;
    crate::commands::ops::archive::execute(&None)?;
    crate::commands::release::status::run()?;
    logging::info("release::p0_p1_nightly", "P0+P1 nightly pipeline passed", &[]);
    Ok(())
}

fn candidate_gate() -> Result<()> {
    logging::info("release::candidate_gate", "Running native release candidate gate", &[]);
    p0_p1_nightly()?;
    logging::info("release::candidate_gate", "Release candidate gate passed", &[]);
    Ok(())
}
