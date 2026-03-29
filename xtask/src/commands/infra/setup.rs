use anyhow::{Result, bail};

use crate::cli::SetupAction;
use crate::utils::process;

/// Entry point for `cargo xtask setup <action>`.
pub fn execute(action: &SetupAction) -> Result<()> {
    match action {
        SetupAction::Audit => {
            println!("[setup::audit] Auditing host environment for missing dependencies");
            run_audit()
        }
        SetupAction::Repair => {
            println!("[setup::repair] Auto-repairing host environment");
            run_repair()
        }
        SetupAction::Bootstrap => {
            println!("[setup::bootstrap] Full workspace bootstrap pipeline");
            println!("[setup::bootstrap]   1. Environment audit");
            println!("[setup::bootstrap]   2. Auto-repair");
            println!("[setup::bootstrap]   3. Build + smoke test");
            println!("[setup::bootstrap]   4. Dashboard generation");
            run_audit()?;
            run_repair()?;
            crate::commands::infra::build::execute(&crate::cli::BuildAction::Full)?;
            crate::commands::ops::qemu::smoke_test()?;
            crate::commands::dashboard::execute(&crate::cli::DashboardAction::Build)
        }
    }
}

fn run_audit() -> Result<()> {
    let required = [
        "cargo",
        "rustc",
        "python",
        "npm",
    ];
    let mut missing = Vec::new();
    for bin in required {
        if process::which(bin) {
            println!("[setup::audit] OK: {}", bin);
        } else {
            println!("[setup::audit] MISSING: {}", bin);
            missing.push(bin.to_string());
        }
    }
    if process::which("qemu-system-x86_64") {
        println!("[setup::audit] OK: qemu-system-x86_64");
    } else {
        println!("[setup::audit] WARN: qemu-system-x86_64 not found (smoke tests limited)");
    }
    if missing.is_empty() {
        Ok(())
    } else {
        bail!("missing required tools: {}", missing.join(", "))
    }
}

fn run_repair() -> Result<()> {
    // Native repair is intentionally conservative: validate core toolchain and
    // install workspace dependencies where package managers are already present.
    if process::which("npm") {
        let dashboard_dir = crate::utils::paths::resolve("dashboard");
        let _ = process::run_status_in_dir("npm", &["install"], &dashboard_dir)?;
    }
    if process::which("python") {
        let _ = process::run_status_in_dir("python", &["-m", "pip", "install", "-r", "requirements.txt"], &crate::utils::paths::repo_root());
    }
    println!("[setup::repair] Completed best-effort dependency repair");
    Ok(())
}
