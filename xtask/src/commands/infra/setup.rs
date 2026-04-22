use crate::cli::SetupAction;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use crate::commands::infra::{installer_policy, installer_profile};
use crate::utils::context;

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
        SetupAction::InstallerSelect {
            profile,
            apps,
            packages,
            include,
            exclude,
            mirror,
            out,
        } => {
            let selection = installer_profile::resolve_selection(
                profile,
                apps.as_deref(),
                packages.as_deref(),
                include.as_deref(),
                exclude.as_deref(),
                mirror.as_deref(),
            )
            .context("Failed to resolve installer selection")?;
            let policy = installer_policy::resolve_policy(&selection.profile)
                .context("Failed to resolve installer policy")?;

            let out_path = if let Some(path) = out.as_deref() {
                PathBuf::from(path)
            } else {
                context::out_dir().join("tooling/installer/selection.json")
            };

            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent).context("Failed to create installer output directory")?;
            }

            let payload = serde_json::json!({
                "selection": selection,
                "policy": policy,
            });
            fs::write(&out_path, serde_json::to_string_pretty(&payload)?)
                .context("Failed to write installer selection artifact")?;

            println!("[setup::installer] profile={}", profile);
            println!("[setup::installer] wrote {}", out_path.display());
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
