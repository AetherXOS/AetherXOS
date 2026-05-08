use crate::utils::sys::process;
use crate::utils::ui::logging;
use anyhow::{Context, Result, anyhow};
use std::path::Path;

/// Converts a Windows path (e.g. C:\foo) to a WSL path (e.g. /mnt/c/foo or /mnt/host/c/foo).
pub fn to_wsl_path(p: &Path) -> Result<String> {
    let s = p.to_string_lossy().replace('\\', "/");

    let output = std::process::Command::new("wsl")
        .args(["wslpath", "-a", "-u"])
        .arg(&s)
        .output()
        .context("Failed to run 'wsl wslpath'")?;

    if !output.status.success() {
        if let Some(pos) = s.find(':') {
            if pos == 1 {
                let drive = s.chars().next().unwrap().to_ascii_lowercase();
                let rest = &s[2..];
                return Ok(format!("/mnt/{}/{}", drive, rest.trim_start_matches('/')));
            }
        }
        return Ok(s);
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Runs a command inside WSL with a set of best-effort tool checks.
pub fn run_in_wsl(script: &str, tools: &[&str]) -> Result<()> {
    if !process::which("wsl") {
        return Err(anyhow!("WSL is not available on this host"));
    }

    let mut tool_check = String::new();
    for tool in tools {
        tool_check.push_str(&format!(
            "if ! command -v \"{}\" &>/dev/null; then echo \"ERROR: {} not found in WSL\"; exit 1; fi\n",
            tool, tool
        ));
    }

    let full_script = format!("set -e\n{}\n{}", tool_check, script);

    logging::info("wsl", "Executing script in WSL environment", &[]);
    process::run_checked("wsl", ["-e", "sh", "-c", &full_script])
}

/// Checks if a file exists within the WSL environment.
pub fn wsl_file_exists(path: &str) -> bool {
    process::run_best_effort("wsl", ["-e", "test", "-f", path])
}
