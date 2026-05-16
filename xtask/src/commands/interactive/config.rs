use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use crate::utils::{logging, paths};
use inquire::{Select, Text};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PersistentConfig {
    pub last_features: Vec<String>,
    pub last_arch: String,
    pub last_profile: String,
    pub last_bootloader: String,
    pub last_image_format: String,
}

pub fn manage_config() -> Result<()> {
    let options = vec!["Save Current Workspace State", "Load Named Profile", "Reset to Defaults", "Back"];
    let selection = Select::new("Configuration Management", options).prompt()?;

    match selection {
        "Save Current Workspace State" => save_current_state()?,
        "Load Named Profile" => load_named_profile()?,
        "Reset to Defaults" => reset_to_defaults()?,
        _ => {}
    }
    Ok(())
}

pub fn get_active_config() -> Result<PersistentConfig> {
    let path = get_active_config_path()?;
    if !path.exists() {
        return Ok(PersistentConfig::default());
    }
    let content = fs::read_to_string(&path)?;
    let config: PersistentConfig = toml::from_str(&content).unwrap_or_default();
    Ok(config)
}

pub fn save_active_config(config: &PersistentConfig) -> Result<()> {
    let path = get_active_config_path()?;
    let content = toml::to_string_pretty(config)?;
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(&path, content)?;
    Ok(())
}

fn save_current_state() -> Result<()> {
    let name = Text::new("Profile Name (e.g. night-build):").prompt()?;
    let active = get_active_config()?;
    let path = get_profiles_dir()?.join(format!("{}.toml", name));
    
    let content = toml::to_string_pretty(&active)?;
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(&path, content)?;
    
    logging::success("CONFIG", "Profile saved", &[("name", &name), ("path", &path.to_string_lossy())]);
    Ok(())
}

fn load_named_profile() -> Result<()> {
    let dir = get_profiles_dir()?;
    if !dir.exists() {
        logging::warn("CONFIG", "No profiles found", &[]);
        return Ok(());
    }

    let files: Vec<String> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "toml"))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();

    if files.is_empty() {
        logging::warn("CONFIG", "No profiles found", &[]);
        return Ok(());
    }

    let selection = Select::new("Select profile to load", files).prompt()?;
    let path = get_profiles_dir()?.join(&selection);
    let content = fs::read_to_string(&path)?;
    let profile: PersistentConfig = toml::from_str(&content)?;
    
    save_active_config(&profile)?;
    logging::success("CONFIG", "Profile loaded as active configuration", &[("name", &selection)]);
    
    Ok(())
}

fn reset_to_defaults() -> Result<()> {
    save_active_config(&PersistentConfig::default())?;
    logging::success("CONFIG", "Configuration reset to defaults", &[]);
    Ok(())
}

fn get_active_config_path() -> Result<PathBuf> {
    Ok(paths::repo_root().join("xtask").join("active_config.toml"))
}

fn get_profiles_dir() -> Result<PathBuf> {
    Ok(paths::repo_root().join("xtask").join("profiles"))
}
