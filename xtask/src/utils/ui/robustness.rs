use crate::utils::logging;
use anyhow::Result;

pub fn check_prerequisites() -> Result<()> {
    logging::step("AUDIT", "Checking system prerequisites...");

    // Check for rustc
    let rustc = std::process::Command::new("rustc")
        .arg("--version")
        .output();
    if rustc.is_err() {
        logging::error("AUDIT", "rustc not found in PATH", &[]);
        return Err(anyhow::anyhow!("rustc missing"));
    }

    // Check for qemu (optional but recommended)
    let qemu = std::process::Command::new("qemu-system-x86_64")
        .arg("--version")
        .output();
    if qemu.is_err() {
        logging::warn(
            "AUDIT",
            "qemu-system-x86_64 not found; emulation will fail",
            &[],
        );
    }

    logging::success("AUDIT", "System audit completed", &[]);
    Ok(())
}

pub fn wrap_execution<F, T>(module: &str, operation: &str, f: F) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    crate::utils::ui::logging::aop_wrap_result(module, operation, f)
}
