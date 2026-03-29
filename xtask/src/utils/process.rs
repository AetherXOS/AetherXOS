use anyhow::{Context, Result, bail};
use std::process::Command;
use std::path::Path;

/// Run an arbitrary command, only checking for success.
pub fn run_checked(program: &str, args: &[&str]) -> Result<()> {
    println!("[exec] {} {}", program, args.join(" "));
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("Failed to execute: {} {}", program, args.join(" ")))?;
    if !status.success() {
        bail!("{} failed (exit code: {})", program, status.code().unwrap_or(-1));
    }
    Ok(())
}

/// Run an arbitrary command in a specific working directory.
pub fn run_checked_in_dir(program: &str, args: &[&str], cwd: &Path) -> Result<()> {
    println!("[exec] (cd {}) {} {}", cwd.display(), program, args.join(" "));
    let status = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .status()
        .with_context(|| format!("Failed to execute in {}: {} {}", cwd.display(), program, args.join(" ")))?;
    if !status.success() {
        bail!("{} failed (exit code: {})", program, status.code().unwrap_or(-1));
    }
    Ok(())
}

/// Run a command in a specific directory and return raw exit status.
pub fn run_status_in_dir(program: &str, args: &[&str], cwd: &Path) -> Result<std::process::ExitStatus> {
    println!("[exec] (cd {}) {} {}", cwd.display(), program, args.join(" "));
    Command::new(program)
        .args(args)
        .current_dir(cwd)
        .status()
        .with_context(|| format!("Failed to execute in {}: {} {}", cwd.display(), program, args.join(" ")))
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
