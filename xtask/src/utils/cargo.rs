use crate::constants::{cargo as cargo_consts, tools};
use crate::utils::{context, logging};
use anyhow::{Context, Result, bail};
use std::process::Command;

/// Run `cargo` with the given arguments and bail on failure.
pub fn cargo(args: &[&str]) -> Result<()> {
    logging::exec("cargo", &format!("cargo {}", args.join(" ")));
    let status = Command::new(tools::CARGO)
        .args(args)
        .status()
        .context("Failed to invoke cargo")?;
    if !status.success() {
        bail!(
            "cargo {} failed (exit code: {})",
            args.join(" "),
            status.code().unwrap_or(-1)
        );
    }
    Ok(())
}

/// Run `cargo` with the given arguments in a specific directory.
pub fn cargo_in_dir(args: &[&str], cwd: &std::path::Path) -> Result<()> {
    logging::exec("cargo", &format!("(cd {}) cargo {}", cwd.display(), args.join(" ")));
    let status = Command::new(tools::CARGO)
        .args(args)
        .current_dir(cwd)
        .status()
        .context("Failed to invoke cargo")?;
    if !status.success() {
        bail!(
            "cargo {} failed in {} (exit code: {})",
            args.join(" "),
            cwd.display(),
            status.code().unwrap_or(-1)
        );
    }
    Ok(())
}

/// Run `cargo check` for a specific feature combination.
pub fn cargo_check_features(
    label: &str,
    features: &str,
    host_target: Option<&str>,
    release: bool,
) -> Result<()> {
    logging::info("cargo", &format!("checking variant: {}", label), &[]);
    let mut args = vec![cargo_consts::CMD_CHECK, "--lib"];
    if !features.is_empty() {
        args.push(cargo_consts::ARG_FEATURES);
        args.push(features);
    }
    if let Some(target) = host_target {
        args.push(cargo_consts::ARG_TARGET);
        args.push(target);
    }
    if release {
        args.push(cargo_consts::ARG_RELEASE);
    }
    cargo(&args)
}

/// Detect the host target triple from `rustc -vV`.
pub fn detect_host_triple() -> Result<String> {
    match context::host_target() {
        Ok(host) => Ok(host.to_string()),
        Err(_) => context::detect_host_triple_from_rustc(),
    }
}
