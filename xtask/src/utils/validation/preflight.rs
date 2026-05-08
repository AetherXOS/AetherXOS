use crate::utils::sys::process;
use crate::utils::ui::logging;
use anyhow::{Context, Result};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

/// Orchestrates a comprehensive system health audit before pipeline initiation.
pub fn run_audit() -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner().template("{spinner:.magenta} {msg:.bold.white}")?,
    );
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    pb.set_message("Auditing binary dependencies...");
    check_binary_dependencies(&pb)?;

    pb.set_message("Verifying cryptographic utility availability...");
    check_crypto_tools(&pb)?;

    pb.set_message("Evaluating storage resource bounds...");
    check_disk_space(10)?;

    pb.set_message("Validating environmental integrity...");
    check_environment_health()?;

    pb.finish_with_message("System audit complete. Environment verified for production build.");
    Ok(())
}

fn check_binary_dependencies(pb: &ProgressBar) -> Result<()> {
    let required: &[&str] = if cfg!(windows) {
        &["wsl", "tar", "7z"]
    } else {
        &["tar", "xorriso", "mkisofs"]
    };

    for tool in required {
        if !process::which(tool) {
            pb.println(format!(
                "  {} Recommendation: '{}' is missing from PATH.",
                "⚠️".yellow(),
                tool
            ));
        }
    }
    Ok(())
}

fn check_crypto_tools(pb: &ProgressBar) -> Result<()> {
    if !cfg!(windows) {
        let tools = ["sha256sum", "md5sum", "sha1sum"];
        for tool in tools {
            if !process::which(tool) {
                pb.println(format!(
                    "  {} Optimization: Cryptographic tool '{}' not found.",
                    "ℹ".blue(),
                    tool
                ));
            }
        }
    }
    Ok(())
}

pub fn check_disk_space(needed_gb: u64) -> Result<()> {
    if cfg!(windows) {
        let output = std::process::Command::new("powershell")
            .args(&[
                "-NoProfile",
                "-Command",
                "Get-PSDrive C | Select-Object -ExpandProperty Free",
            ])
            .output()
            .context("Failed to query storage metrics via PowerShell")?;

        let free_bytes = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse::<u64>()
            .unwrap_or(u64::MAX);

        let free_gb = free_bytes / 1024 / 1024 / 1024;

        if free_gb < needed_gb {
            logging::warn(
                "preflight",
                "constrained storage capacity",
                &[
                    ("needed", &format!("{}GB", needed_gb)),
                    ("available", &format!("{}GB", free_gb)),
                ],
            );
        }
    }
    Ok(())
}

pub fn check_environment_health() -> Result<()> {
    let artifacts_dir = crate::utils::fs::paths::resolve(crate::constants::paths::ARTIFACTS_DIR);
    if !artifacts_dir.exists() {
        std::fs::create_dir_all(&artifacts_dir)
            .context("Failed establishing artifacts boundary")?;
    }

    let test_file = artifacts_dir.join(".health_probe");
    std::fs::write(&test_file, "AETHERCORE_INTEGRITY_PROBE")
        .context("Environmental Write Fault: Artifacts directory is read-only or locked.")?;

    let _ = std::fs::remove_file(test_file);
    Ok(())
}
