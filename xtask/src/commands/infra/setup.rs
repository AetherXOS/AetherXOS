use crate::cli::SetupAction;
use anyhow::{Context, Result};

#[path = "setup/audit.rs"]
mod audit;
#[path = "setup/download.rs"]
mod download;
#[path = "setup/platform.rs"]
mod platform;
#[path = "setup/provision.rs"]
mod provision;

/// Entry layer for generic system orchestrations, toolchain validations, and automated resource provisionings.
pub fn execute(action: &SetupAction) -> Result<()> {
    match action {
        SetupAction::Audit => {
            audit::audit_host_environment().context("Host machine evaluation capability failed")?;
        }
        SetupAction::Repair | SetupAction::Bootstrap => {
            println!(
                "[setup::bootstrap] Initiating zero-dependency automated remediation sequence."
            );
            provision::provision_host_environment().context("Strict host provisioning failed")?;
            download::fetch_limine_binaries().context("Bootloader synchronization failed")?;
        }
        SetupAction::FetchBootloader => {
            download::fetch_limine_binaries()
                .context("Bootloader binary synchronization workflow collapsed")?;
        }
        SetupAction::Toolchain => {
            provision::provision_cross_compiler()
                .context("Cross-compiler synchronization failed")?;
        }
    }

    Ok(())
}
