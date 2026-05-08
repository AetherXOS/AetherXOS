use crate::utils::process;
use anyhow::{Result, bail};
use std::path::Path;

pub fn ensure_iso_tools() -> Result<()> {
    if process::which("xorriso") || process::which("mkisofs") || process::which("oscdimg") {
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        let script = paths::resolve("scripts/ensure_iso_tools.ps1");
        if script.exists() {
            logging::info("iso", "Installing ISO creation tools...", &[]);
            let output = Command::new("powershell")
                .args(&[
                    "-ExecutionPolicy",
                    "Bypass",
                    "-File",
                    script.to_string_lossy().as_ref(),
                ])
                .output();

            if let Ok(output) = output {
                if output.status.success() {
                    logging::info("iso", "ISO tools installed successfully", &[]);
                    return Ok(());
                }
            }
        }
    }

    Ok(())
}

pub fn find_iso_tool() -> Result<String> {
    if process::which("xorriso") {
        return Ok("xorriso".to_string());
    }

    for msys_path in &[
        r"C:\msys64\usr\bin\xorriso.exe",
        r"C:\Program Files\Git\usr\bin\xorriso.exe",
    ] {
        if Path::new(msys_path).exists() {
            return Ok(msys_path.to_string());
        }
    }

    if process::which("mkisofs") {
        return Ok("mkisofs".to_string());
    }
    if process::which("oscdimg") {
        return Ok("oscdimg".to_string());
    }

    bail!("No ISO creation tool found (xorriso, mkisofs, or oscdimg).")
}
