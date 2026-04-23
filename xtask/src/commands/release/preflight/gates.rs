use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::commands::infra;
use crate::commands::ops;
use crate::commands::validation;
use crate::config;
use crate::constants;
use crate::utils::{cargo, paths, process};

use super::abi::abi_drift_report;
use super::ci::reproducible_evidence;
use super::diagnostics::{critical_policy_guard, release_diagnostics};
use super::evidence_bundle;
use super::host_tools::host_tool_verify;

pub fn preflight(
    skip_host_tests: bool,
    skip_boot_artifacts: bool,
    strict_production_gate: bool,
) -> Result<()> {
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
            arch: constants::defaults::build::ARCH,
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
        constants::defaults::glibc::FORMAT_MD,
        &Some("reports/syscall_coverage.md".to_string()),
    )?;

    println!("[release::preflight] Step 7: linux_compat profile compile + syscall gate");
    cargo::cargo(&["check", "--features", "linux_compat,posix_deep_tests"])?;
    validation::syscall_coverage::execute(
        true,
        constants::defaults::glibc::FORMAT_MD,
        &Some("reports/syscall_coverage_linux_compat.md".to_string()),
    )?;

    println!("[release::preflight] Step 8: POSIX deep tests compile gate");
    validation::test::execute(&crate::cli::TestAction::PosixConformance)?;

    println!("[release::preflight] Step 9: Generate production acceptance scorecard");
    crate::commands::release::status::run()?;

    if strict_production_gate {
        println!("[release::preflight] Step 10: Strict production acceptance gate");
        enforce_production_acceptance_gate()?;
    } else {
        println!(
            "[release::preflight] Step 10: Skipped strict production gate (--strict-production-gate to enable)"
        );
    }

    println!("[release::preflight] PASS - Release preflight completed successfully.");
    Ok(())
}

pub fn enforce_production_acceptance_gate() -> Result<()> {
    let root = paths::repo_root();
    let scorecard_path = root.join(config::repo_paths::PRODUCTION_ACCEPTANCE_SCORECARD_JSON);

    let scorecard_text = std::fs::read_to_string(&scorecard_path).with_context(|| {
        format!(
            "strict production gate requires scorecard: {}",
            scorecard_path.display()
        )
    })?;
    let scorecard: Value = serde_json::from_str(&scorecard_text).with_context(|| {
        format!(
            "failed parsing scorecard JSON: {}",
            scorecard_path.display()
        )
    })?;

    let overall_ok = scorecard
        .get("overall_ok")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !overall_ok {
        let completion = scorecard
            .get("completion_pct")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        bail!(
            "strict production gate failed: production acceptance scorecard overall_ok=false (completion_pct={completion:.1}). See {}",
            scorecard_path.display()
        );
    }

    enforce_p_tier_trend_no_regression()?;
    enforce_abi_perf_strict_gate()?;

    println!("[release::preflight] strict production acceptance gate PASS");
    Ok(())
}

pub fn enforce_abi_perf_strict_gate() -> Result<()> {
    abi_perf_gate(true)?;

    println!("[release::preflight] ABI + performance strict gate PASS");
    Ok(())
}

pub fn abi_perf_gate(strict: bool) -> Result<()> {
    crate::commands::release::reporting::abi_perf::abi_perf_gate(strict)
}

pub fn enforce_p_tier_trend_no_regression() -> Result<()> {
    let root = paths::repo_root();
    let p_tier_path = root.join(config::repo_paths::P_TIER_STATUS_JSON);

    let p_tier_text = std::fs::read_to_string(&p_tier_path).with_context(|| {
        format!(
            "strict trend gate requires p-tier status report: {}",
            p_tier_path.display()
        )
    })?;
    let p_tier: Value = serde_json::from_str(&p_tier_text).with_context(|| {
        format!(
            "failed parsing p-tier status JSON: {}",
            p_tier_path.display()
        )
    })?;

    let tier_regression = p_tier
        .get("trend")
        .and_then(|v| v.get("overall_regression"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    if tier_regression {
        bail!(
            "strict trend gate failed: p-tier trend regression detected. See {}",
            p_tier_path.display()
        );
    }

    println!("[release::preflight] p-tier trend regression gate PASS");
    Ok(())
}

pub fn p0_gate() -> Result<()> {
    println!("[release::p0-gate] Running native P0 readiness gate");
    preflight(false, false, false)?;
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

pub fn p0_acceptance() -> Result<()> {
    println!("[release::p0-acceptance] Running native P0 release acceptance");
    p0_gate()?;
    ops::soak::execute(false)?;
    println!("[release::p0-acceptance] PASS");
    Ok(())
}

pub fn p1_nightly() -> Result<()> {
    println!("[release::p1-nightly] Running native P1 nightly pipeline");
    p0_gate()?;
    ops::soak::execute(false)?;
    crate::commands::release::status::run()?;
    enforce_p_tier_trend_no_regression()?;
    println!("[release::p1-nightly] PASS");
    Ok(())
}

pub fn p1_acceptance() -> Result<()> {
    println!("[release::p1-acceptance] Running native P1 release acceptance");
    p1_nightly()?;
    println!("[release::p1-acceptance] PASS");
    Ok(())
}

pub fn p0_p1_nightly() -> Result<()> {
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

pub fn candidate_gate() -> Result<()> {
    println!("[release::candidate-gate] Running native release candidate gate");
    p0_p1_nightly()?;
    release_diagnostics(false)?;
    host_tool_verify(true)?;
    critical_policy_guard(true)?;
    reproducible_evidence()?;
    abi_drift_report(None, true)?;
    evidence_bundle::run(true)?;
    enforce_production_acceptance_gate()?;
    release_diagnostics(true)?;
    println!("[release::candidate-gate] PASS");
    Ok(())
}
