use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const PROFILE_CATALOG_PATH: &str = "artifacts/tooling/installer/profiles.json";
const APP_TARGET_CATALOG_PATH: &str = "artifacts/tooling/installer/app_targets.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageManager {
    Apt,
    Pacman,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallerSelection {
    pub profile: String,
    pub package_manager: PackageManager,
    pub mirror: Option<String>,
    pub selected_apps: Vec<String>,
    pub packages: Vec<String>,
    pub download_artifacts: Vec<InstallerDownloadArtifact>,
    pub smoke_commands: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallerDownloadArtifact {
    pub id: String,
    pub url: String,
    pub sha256: Option<String>,
    pub destination: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallerPreset {
    pub id: String,
    pub title: String,
    pub description: String,
    pub package_manager: PackageManager,
    pub default_packages: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallerPresetCatalog {
    pub presets: Vec<InstallerPreset>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallerAppTarget {
    pub id: String,
    pub title: String,
    pub description: String,
    pub packages_by_profile: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub download_artifacts_by_profile: BTreeMap<String, Vec<InstallerDownloadArtifact>>,
    #[serde(default)]
    pub smoke_commands_by_profile: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallerAppTargetCatalog {
    pub targets: Vec<InstallerAppTarget>,
}

pub fn resolve_selection(
    profile: &str,
    apps_csv: Option<&str>,
    packages_override_csv: Option<&str>,
    include_csv: Option<&str>,
    exclude_csv: Option<&str>,
    mirror: Option<&str>,
) -> Result<InstallerSelection> {
    let catalog = load_catalog_from_source()?;
    let app_catalog = load_app_target_catalog_from_source()?;
    let preset = find_preset(&catalog, profile)?;
    let selected_apps = parse_csv(apps_csv.unwrap_or(""));

    let mut selected: BTreeSet<String> = if let Some(raw_override) = packages_override_csv {
        parse_csv(raw_override).into_iter().collect()
    } else {
        preset.default_packages.iter().cloned().collect()
    };
    let mut artifacts = BTreeMap::<String, InstallerDownloadArtifact>::new();
    let mut smoke_commands = BTreeSet::<String>::new();

    for app in &selected_apps {
        let target = find_app_target(&app_catalog, app)?;
        for pkg in packages_for_target(target, &preset.id) {
            selected.insert(pkg.to_string());
        }
        for artifact in download_artifacts_for_target(target, &preset.id) {
            let key = format!("{}|{}", artifact.id, artifact.url);
            artifacts.insert(key, artifact.clone());
        }
        for command in smoke_commands_for_target(target, &preset.id) {
            smoke_commands.insert(command);
        }
    }

    if let Some(raw_include) = include_csv {
        for pkg in parse_csv(raw_include) {
            selected.insert(pkg);
        }
    }

    if let Some(raw_exclude) = exclude_csv {
        for pkg in parse_csv(raw_exclude) {
            selected.remove(&pkg);
        }
    }

    Ok(InstallerSelection {
        profile: preset.id.clone(),
        package_manager: preset.package_manager,
        mirror: mirror.map(|m| m.trim().to_string()).filter(|m| !m.is_empty()),
        selected_apps,
        packages: selected.into_iter().collect(),
        download_artifacts: artifacts.into_values().collect(),
        smoke_commands: smoke_commands.into_iter().collect(),
    })
}

fn parse_csv(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string())
        .collect()
}

fn packages_for_target<'a>(target: &'a InstallerAppTarget, profile_id: &str) -> Vec<&'a str> {
    if let Some(items) = target.packages_by_profile.get(profile_id) {
        return items.iter().map(String::as_str).collect();
    }
    target
        .packages_by_profile
        .get("*")
        .map(|items| items.iter().map(String::as_str).collect())
        .unwrap_or_default()
}

fn download_artifacts_for_target(
    target: &InstallerAppTarget,
    profile_id: &str,
) -> Vec<InstallerDownloadArtifact> {
    if let Some(items) = target.download_artifacts_by_profile.get(profile_id) {
        return items.clone();
    }
    target
        .download_artifacts_by_profile
        .get("*")
        .cloned()
        .unwrap_or_default()
}

fn smoke_commands_for_target(target: &InstallerAppTarget, profile_id: &str) -> Vec<String> {
    if let Some(items) = target.smoke_commands_by_profile.get(profile_id) {
        return items.clone();
    }
    target
        .smoke_commands_by_profile
        .get("*")
        .cloned()
        .unwrap_or_default()
}

fn find_preset<'a>(catalog: &'a InstallerPresetCatalog, profile: &str) -> Result<&'a InstallerPreset> {
    let key = profile.trim().to_ascii_lowercase();
    if let Some(preset) = catalog
        .presets
        .iter()
        .find(|p| p.id.to_ascii_lowercase() == key)
    {
        return Ok(preset);
    }

    let supported = catalog
        .presets
        .iter()
        .map(|p| p.id.clone())
        .collect::<Vec<_>>()
        .join(", ");
    bail!(
        "unknown profile '{}'; supported profiles from {}: {}",
        profile,
        PROFILE_CATALOG_PATH,
        supported
    )
}

fn load_catalog_from_source() -> Result<InstallerPresetCatalog> {
    let path = crate::utils::paths::resolve(PROFILE_CATALOG_PATH);
    let raw = fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {}", path.display(), e))?;
    let catalog: InstallerPresetCatalog = serde_json::from_str(&raw)
        .map_err(|e| anyhow::anyhow!("invalid JSON at {}: {}", path.display(), e))?;
    validate_catalog(&catalog)?;
    Ok(catalog)
}

fn load_app_target_catalog_from_source() -> Result<InstallerAppTargetCatalog> {
    let path = crate::utils::paths::resolve(APP_TARGET_CATALOG_PATH);
    let raw = fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {}", path.display(), e))?;
    let catalog: InstallerAppTargetCatalog = serde_json::from_str(&raw)
        .map_err(|e| anyhow::anyhow!("invalid JSON at {}: {}", path.display(), e))?;
    validate_app_target_catalog(&catalog)?;
    Ok(catalog)
}

fn validate_catalog(catalog: &InstallerPresetCatalog) -> Result<()> {
    if catalog.presets.is_empty() {
        bail!("installer profile catalog is empty")
    }

    let mut ids = BTreeSet::new();
    for preset in &catalog.presets {
        if preset.id.trim().is_empty() {
            bail!("installer profile contains empty id")
        }
        if preset.default_packages.is_empty() {
            bail!("installer profile '{}' has no default packages", preset.id)
        }
        let normalized = preset.id.to_ascii_lowercase();
        if !ids.insert(normalized) {
            bail!("duplicate installer profile id '{}'", preset.id)
        }
    }

    Ok(())
}

fn validate_app_target_catalog(catalog: &InstallerAppTargetCatalog) -> Result<()> {
    if catalog.targets.is_empty() {
        bail!("installer app target catalog is empty")
    }

    let mut ids = BTreeSet::new();
    for target in &catalog.targets {
        if target.id.trim().is_empty() {
            bail!("installer app target contains empty id")
        }
        if target.packages_by_profile.is_empty() {
            bail!(
                "installer app target '{}' has no packages_by_profile entries",
                target.id
            )
        }

        let mut has_any_pkg = false;
        for (profile, pkgs) in &target.packages_by_profile {
            if profile.trim().is_empty() {
                bail!(
                    "installer app target '{}' has empty packages_by_profile key",
                    target.id
                )
            }
            if !pkgs.is_empty() {
                has_any_pkg = true;
            }
        }
        if !has_any_pkg {
            bail!("installer app target '{}' has no package entries", target.id)
        }

        let normalized = target.id.to_ascii_lowercase();
        if !ids.insert(normalized) {
            bail!("duplicate installer app target id '{}'", target.id)
        }

        for artifacts in target.download_artifacts_by_profile.values() {
            for artifact in artifacts {
                if artifact.id.trim().is_empty() {
                    bail!("installer app target '{}' has artifact with empty id", target.id)
                }
                if artifact.url.trim().is_empty() {
                    bail!(
                        "installer app target '{}' has artifact '{}' with empty url",
                        target.id,
                        artifact.id
                    )
                }
                if artifact.destination.trim().is_empty() {
                    bail!(
                        "installer app target '{}' has artifact '{}' with empty destination",
                        target.id,
                        artifact.id
                    )
                }
            }
        }

        for commands in target.smoke_commands_by_profile.values() {
            for command in commands {
                if command.trim().is_empty() {
                    bail!(
                        "installer app target '{}' has empty smoke command entry",
                        target.id
                    )
                }
            }
        }
    }

    Ok(())
}

fn find_app_target<'a>(catalog: &'a InstallerAppTargetCatalog, app: &str) -> Result<&'a InstallerAppTarget> {
    let key = app.trim().to_ascii_lowercase();
    if let Some(target) = catalog
        .targets
        .iter()
        .find(|t| t.id.to_ascii_lowercase() == key)
    {
        return Ok(target);
    }

    let supported = catalog
        .targets
        .iter()
        .map(|t| t.id.clone())
        .collect::<Vec<_>>()
        .join(", ");
    bail!(
        "unknown app target '{}'; supported targets from {}: {}",
        app,
        APP_TARGET_CATALOG_PATH,
        supported
    )
}

pub fn preset_catalog() -> Result<InstallerPresetCatalog> {
    load_catalog_from_source()
}

pub fn write_preset_catalog(out_path: &Path) -> Result<()> {
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&preset_catalog()?)?;
    fs::write(out_path, json)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn include_exclude_overrides_defaults() {
        let selection = resolve_selection(
            "debian",
            None,
            None,
            Some("vlc, git"),
            Some("xfce4"),
            Some("https://deb.debian.org/debian"),
        )
        .expect("selection");

        assert_eq!(selection.profile, "debian");
        assert_eq!(selection.package_manager, PackageManager::Apt);
        assert!(selection.packages.iter().any(|p| p == "vlc"));
        assert!(selection.packages.iter().any(|p| p == "git"));
        assert!(!selection.packages.iter().any(|p| p == "xfce4"));
        assert_eq!(selection.mirror.as_deref(), Some("https://deb.debian.org/debian"));
    }

    #[test]
    fn app_targets_expand_package_set() {
        let selection = resolve_selection(
            "debian",
            Some("python,chrome"),
            None,
            None,
            None,
            None,
        )
        .expect("selection");

        assert!(selection.selected_apps.iter().any(|a| a == "python"));
        assert!(selection.selected_apps.iter().any(|a| a == "chrome"));
        assert!(selection.packages.iter().any(|p| p == "python3-pip"));
        assert!(!selection.smoke_commands.is_empty());
    }
}
