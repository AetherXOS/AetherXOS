use anyhow::Result;
use crate::utils::logging;

pub fn check_prerequisites() -> Result<()> {
    logging::step("AUDIT", "Checking system prerequisites...");
    
    // Check for rustc
    let rustc = std::process::Command::new("rustc").arg("--version").output();
    if rustc.is_err() {
        logging::error("AUDIT", "rustc not found in PATH", &[]);
        return Err(anyhow::anyhow!("rustc missing"));
    }
    
    // Check for qemu (optional but recommended)
    let qemu = std::process::Command::new("qemu-system-x86_64").arg("--version").output();
    if qemu.is_err() {
        logging::warn("AUDIT", "qemu-system-x86_64 not found; emulation will fail", &[]);
    }
    
    logging::success("AUDIT", "System audit completed", &[]);
    Ok(())
}

pub fn wrap_execution<F, T>(module: &str, operation: &str, f: F) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    logging::status(module, &format!("Starting {}...", operation));
    match f() {
        Ok(val) => {
            logging::success(module, &format!("{} completed successfully", operation), &[]);
            Ok(val)
        }
        Err(e) => {
            logging::error(module, &format!("{} failed", operation), &[("error", &format!("{:#}", e))]);
            Err(e)
        }
    }
}
