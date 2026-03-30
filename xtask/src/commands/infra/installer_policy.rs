use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::fs;

const POLICY_CATALOG_PATH: &str = "artifacts/tooling/installer/policies.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallerPolicy {
    pub retry_max_attempts: u32,
    pub retry_backoff_seconds: u32,
    pub checksum_required: bool,
    pub metadata_signature_required: bool,
    pub metadata_signature_mode: String,
    pub apt_trusted_keyring_paths: Vec<String>,
    pub pacman_keyring_dir: String,
    pub artifact_ledger_path: String,
    pub transaction_log_path: String,
    pub transaction_state_path: String,
    pub event_log_path: String,
    pub resume_marker_path: String,
    pub rollback_marker_path: String,
    pub install_timeout_seconds: u32,
    pub smoke_timeout_seconds: u32,
    pub postinstall_hooks: Vec<String>,
    pub package_pins: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallerPolicyCatalog {
    pub defaults: InstallerPolicy,
    pub profile_overrides: std::collections::BTreeMap<String, InstallerPolicy>,
}

pub fn resolve_policy(profile: &str) -> Result<InstallerPolicy> {
    let catalog = load_policy_catalog()?;
    let key = profile.trim().to_ascii_lowercase();
    if let Some(policy) = catalog.profile_overrides.get(&key) {
        return Ok(policy.clone());
    }
    Ok(catalog.defaults)
}

fn load_policy_catalog() -> Result<InstallerPolicyCatalog> {
    let path = crate::utils::paths::resolve(POLICY_CATALOG_PATH);
    let raw = fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {}", path.display(), e))?;
    let catalog: InstallerPolicyCatalog = serde_json::from_str(&raw)
        .map_err(|e| anyhow::anyhow!("invalid JSON at {}: {}", path.display(), e))?;
    validate_policy(&catalog.defaults)?;
    for (profile, policy) in &catalog.profile_overrides {
        if profile.trim().is_empty() {
            bail!("installer policy has empty profile_overrides key")
        }
        validate_policy(policy)?;
    }
    Ok(catalog)
}

fn validate_policy(policy: &InstallerPolicy) -> Result<()> {
    if policy.retry_max_attempts == 0 {
        bail!("retry_max_attempts must be > 0")
    }
    if policy.install_timeout_seconds == 0 {
        bail!("install_timeout_seconds must be > 0")
    }
    if policy.smoke_timeout_seconds == 0 {
        bail!("smoke_timeout_seconds must be > 0")
    }
    if policy.metadata_signature_mode != "presence"
        && policy.metadata_signature_mode != "gpg-strict"
    {
        bail!(
            "metadata_signature_mode must be one of: presence, gpg-strict (got '{}')",
            policy.metadata_signature_mode
        )
    }
    for path in &policy.apt_trusted_keyring_paths {
        if path.trim().is_empty() {
            bail!("apt_trusted_keyring_paths must not contain empty entries")
        }
    }
    if policy.pacman_keyring_dir.trim().is_empty() {
        bail!("pacman_keyring_dir must not be empty")
    }
    if policy.artifact_ledger_path.trim().is_empty() {
        bail!("artifact_ledger_path must not be empty")
    }
    if policy.transaction_log_path.trim().is_empty() {
        bail!("transaction_log_path must not be empty")
    }
    if policy.transaction_state_path.trim().is_empty() {
        bail!("transaction_state_path must not be empty")
    }
    if policy.event_log_path.trim().is_empty() {
        bail!("event_log_path must not be empty")
    }
    if policy.resume_marker_path.trim().is_empty() {
        bail!("resume_marker_path must not be empty")
    }
    if policy.rollback_marker_path.trim().is_empty() {
        bail!("rollback_marker_path must not be empty")
    }
    Ok(())
}
