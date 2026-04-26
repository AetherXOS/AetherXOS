pub mod driver;
pub mod host;
pub mod kernel_refactor_audit;
pub mod linux_app_compat;
pub mod posix;
pub mod tier;

use crate::cli::TestAction;
use crate::constants;
use crate::constants::npm;
use crate::utils::logging;
use anyhow::{Result, bail};

/// Entry point for `cargo run -p xtask -- test <action>`.
pub fn execute(action: &TestAction) -> Result<()> {
    match action {
        TestAction::QualityGate => quality_gate(),
        TestAction::Host { release } => host::validate_feature_matrix(*release),
        TestAction::AgentContract => agent_contract(),
        TestAction::All { ci } => tier::run_all(*ci),
        TestAction::Tier { tier, ci } => tier::run(tier, *ci),
        TestAction::PosixConformance => posix::run_gate(),
        TestAction::DriverSmoke => driver::run_smoke(),
        TestAction::LinuxAppCompat {
            desktop_smoke,
            quick,
            qemu,
            strict,
            ci,
            require_busybox,
            require_glibc,
            require_wayland,
            require_x11,
            require_fs_stack,
            require_package_stack,
            require_desktop_app_stack,
        } => linux_app_compat::run(linux_app_compat::LinuxAppCompatOptions {
            desktop_smoke: *desktop_smoke,
            quick: *quick,
            qemu: *qemu,
            strict: *strict,
            ci: *ci,
            require_busybox: *require_busybox,
            require_glibc: *require_glibc,
            require_wayland: *require_wayland,
            require_x11: *require_x11,
            require_fs_stack: *require_fs_stack,
            require_package_stack: *require_package_stack,
            require_desktop_app_stack: *require_desktop_app_stack,
        }),
        TestAction::KernelRefactorAudit {
            max_lines,
            magic_repeat_threshold,
        } => kernel_refactor_audit::run(*max_lines, *magic_repeat_threshold),
    }
}

/// Run the full tooling quality gate.
/// Replaces: scripts/full-check.ps1
fn quality_gate() -> Result<()> {
    logging::info(
        "test::quality-gate",
        "Running native quality gate pipeline",
        &[],
    );
    host::validate_feature_matrix(false)?;
    driver::run_smoke()?;
    posix::run_gate()?;
    crate::commands::validation::linux_abi::execute(&crate::cli::LinuxAbiAction::Gate)?;
    crate::commands::infra::build::execute(&crate::cli::BuildAction::Full {
        arch: constants::defaults::build::ARCH,
        bootloader: crate::cli::Bootloader::Limine,
        format: crate::cli::ImageFormat::Iso,
        release: false,
    })?;
    crate::commands::ops::qemu::smoke_test()?;
    logging::success("test::quality-gate", "Pipeline passed successfully", &[]);
    Ok(())
}

/// Run dashboard agent contract verification.
/// Replaces: scripts/agent-contract.ps1
fn agent_contract() -> Result<()> {
    logging::info(
        "test::agent-contract",
        "Running native dashboard workflow contract suite",
        &[],
    );
    let dashboard_dir = constants::paths::dashboard_dir();

    // Build and workflow tests act as the agent contract baseline.
    crate::utils::process::run_checked_in_dir(
        crate::utils::process::npm_bin(),
        &[npm::ARG_RUN, npm::SCRIPT_BUILD],
        &dashboard_dir,
    )?;
    let workflow = crate::utils::process::run_status_in_dir(
        crate::utils::process::npm_bin(),
        &[npm::ARG_RUN, npm::SCRIPT_WORKFLOW_TEST],
        &dashboard_dir,
    )?;
    if !workflow.success() {
        bail!("dashboard workflow contract test failed");
    }
    logging::success(
        "test::agent-contract",
        "Workflow contract passed successfully",
        &[],
    );
    Ok(())
}
