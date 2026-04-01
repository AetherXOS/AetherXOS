use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;
use crate::utils::logging;

/// Run an arbitrary command, only checking for success.
pub fn run_checked(program: &str, args: &[&str]) -> Result<()> {
    logging::exec("exec", &format!("{} {}", program, args.join(" ")));
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("Failed to execute: {} {}", program, args.join(" ")))?;
    if !status.success() {
        bail!(
            "{} failed (exit code: {})",
            program,
            status.code().unwrap_or(-1)
        );
    }
    Ok(())
}

#[allow(dead_code)]
pub fn run_checked_owned(program: &str, args: &[String]) -> Result<()> {
    let rendered = args.join(" ");
    logging::exec("exec", &format!("{} {}", program, rendered));
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("Failed to execute: {} {}", program, rendered))?;
    if !status.success() {
        bail!(
            "{} failed (exit code: {})",
            program,
            status.code().unwrap_or(-1)
        );
    }
    Ok(())
}

/// Run a command with explicit environment variables.
#[allow(dead_code)]
pub fn run_checked_with_env(program: &str, args: &[&str], envs: &[(&str, &str)]) -> Result<()> {
    logging::exec("exec", &format!("{} {}", program, args.join(" ")));
    let status = Command::new(program)
        .args(args)
        .envs(envs.iter().copied())
        .status()
        .with_context(|| format!("Failed to execute: {} {}", program, args.join(" ")))?;
    if !status.success() {
        bail!(
            "{} failed (exit code: {})",
            program,
            status.code().unwrap_or(-1)
        );
    }
    Ok(())
}

#[allow(dead_code)]
pub fn run_checked_with_env_owned(
    program: &str,
    args: &[String],
    envs: &[(&str, &str)],
) -> Result<()> {
    let rendered = args.join(" ");
    logging::exec("exec", &format!("{} {}", program, rendered));
    let status = Command::new(program)
        .args(args)
        .envs(envs.iter().copied())
        .status()
        .with_context(|| format!("Failed to execute: {} {}", program, rendered))?;
    if !status.success() {
        bail!(
            "{} failed (exit code: {})",
            program,
            status.code().unwrap_or(-1)
        );
    }
    Ok(())
}

/// Run an arbitrary command in a specific working directory.
#[allow(dead_code)]
pub fn run_checked_in_dir(program: &str, args: &[&str], cwd: &Path) -> Result<()> {
    logging::exec("exec", &format!("(cd {}) {} {}", cwd.display(), program, args.join(" ")));
    let status = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .status()
        .with_context(|| {
            format!(
                "Failed to execute in {}: {} {}",
                cwd.display(),
                program,
                args.join(" ")
            )
        })?;
    if !status.success() {
        bail!(
            "{} failed (exit code: {})",
            program,
            status.code().unwrap_or(-1)
        );
    }
    Ok(())
}

/// Run a command in a specific directory and return raw exit status.
#[allow(dead_code)]
pub fn run_status_in_dir(
    program: &str,
    args: &[&str],
    cwd: &Path,
) -> Result<std::process::ExitStatus> {
    logging::exec("exec", &format!("(cd {}) {} {}", cwd.display(), program, args.join(" ")));
    Command::new(program)
        .args(args)
        .current_dir(cwd)
        .status()
        .with_context(|| {
            format!(
                "Failed to execute in {}: {} {}",
                cwd.display(),
                program,
                args.join(" ")
            )
        })
}

/// Check if a binary is available on the system PATH.
pub fn which(binary: &str) -> bool {
    // On Windows, use `where`; on unix, use `which`.
    let probe = if cfg!(windows) { "where" } else { "which" };
    Command::new(probe)
        .arg(binary)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
