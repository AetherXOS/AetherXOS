use crate::constants::{cargo as cargo_consts, tools};
use crate::utils::{context, logging};
use anyhow::{bail, Context, Result};
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
        Err(_) => {
            let output = Command::new(tools::RUSTC)
                .args(["-vV"])
                .output()
                .context("Failed to run rustc -vV")?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if let Some(triple) = line.strip_prefix("host: ") {
                    return Ok(triple.trim().to_string());
                }
            }
            bail!("Could not detect host triple from rustc output")
        }
    }
}
