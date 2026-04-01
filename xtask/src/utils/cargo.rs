use anyhow::{Context, Result, bail};
use std::process::Command;
use crate::utils::logging;

/// Run `cargo` with the given arguments and bail on failure.
pub fn cargo(args: &[&str]) -> Result<()> {
    logging::exec("cargo", &format!("cargo {}", args.join(" ")));
    let status = Command::new("cargo")
        .args(args)
        .status()
        .context("Failed to invoke cargo")?;
    if !status.success() {
        bail!("cargo {} failed (exit code: {})", args.join(" "), status.code().unwrap_or(-1));
    }
    Ok(())
}

/// Run `cargo check` for a specific feature combination.
#[allow(dead_code)]
pub fn cargo_check_features(label: &str, features: &str, host_target: Option<&str>, release: bool) -> Result<()> {
    logging::info("cargo", &format!("checking variant: {}", label), &[]);
    let mut args = vec!["check", "--lib"];
    if !features.is_empty() {
        args.push("--features");
        args.push(features);
    }
    if let Some(target) = host_target {
        args.push("--target");
        args.push(target);
    }
    if release {
        args.push("--release");
    }
    cargo(&args)
}

/// Detect the host target triple from `rustc -vV`.
#[allow(dead_code)]
pub fn detect_host_triple() -> Result<String> {
    let output = Command::new("rustc")
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
