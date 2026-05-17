use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

static NON_INTERACTIVE: AtomicBool = AtomicBool::new(false);

/// Centralized runtime configuration helpers for xtask.
/// Read values from environment variables with sane defaults so code
/// throughout the xtask binary can consult a single source of defaults.

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudTheme {
    Cyberpunk,
    Matrix,
    Steel,
    Dracula,
}

impl std::fmt::Display for HudTheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HudTheme::Cyberpunk => write!(f, "Cyberpunk 🟣"),
            HudTheme::Matrix => write!(f, "Matrix 🟢"),
            HudTheme::Steel => write!(f, "Steel 🔵"),
            HudTheme::Dracula => write!(f, "Dracula 🧛"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct XtaskSettings {
    pub tui_hud_enabled: bool,
    pub webhook_enabled: bool,
    pub voice_enabled: bool,
    pub hud_theme: HudTheme,
}

impl Default for XtaskSettings {
    fn default() -> Self {
        Self {
            tui_hud_enabled: false,
            webhook_enabled: true,
            voice_enabled: true,
            hud_theme: HudTheme::Cyberpunk,
        }
    }
}

static SETTINGS: once_cell::sync::Lazy<std::sync::RwLock<XtaskSettings>> = once_cell::sync::Lazy::new(|| {
    let settings = load_settings().unwrap_or_default();
    std::sync::RwLock::new(settings)
});

fn settings_path() -> PathBuf {
    PathBuf::from("artifacts/xtask_settings.json")
}

fn load_settings() -> anyhow::Result<XtaskSettings> {
    let path = settings_path();
    if !path.exists() {
        return Ok(XtaskSettings::default());
    }
    let content = std::fs::read_to_string(&path)?;
    let settings = serde_json::from_str(&content)?;
    Ok(settings)
}

pub fn save_settings(settings: &XtaskSettings) -> anyhow::Result<()> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(settings)?;
    std::fs::write(&path, content)?;
    Ok(())
}

pub fn get_settings() -> XtaskSettings {
    SETTINGS.read().unwrap().clone()
}

pub fn update_settings<F: FnOnce(&mut XtaskSettings)>(f: F) -> anyhow::Result<()> {
    let mut settings = SETTINGS.write().unwrap();
    f(&mut *settings);
    save_settings(&settings)?;
    Ok(())
}

pub fn set_non_interactive(val: bool) {
    NON_INTERACTIVE.store(val, Ordering::Relaxed);
}

pub fn is_non_interactive() -> bool {
    NON_INTERACTIVE.load(Ordering::Relaxed)
        || env::var("XTASK_NONINTERACTIVE").is_ok()
        || env::var("CI").is_ok()
}

pub fn max_download_attempts() -> usize {
    env::var("XTASK_MAX_DOWNLOAD_ATTEMPTS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4)
}

pub fn download_backoff_base_secs() -> u64 {
    env::var("XTASK_DOWNLOAD_BACKOFF_BASE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2)
}

pub fn prefer_wsl_extraction() -> bool {
    env::var("XTASK_PREFER_WSL_EXTRACTION").is_ok()
}
