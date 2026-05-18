use once_cell::sync::Lazy;
use std::sync::Mutex;

pub struct XTaskConfig {
    pub non_interactive: bool,
    pub log_level: String,
    pub web_port: u16,
    pub collab_port: u16,
}

impl Default for XTaskConfig {
    fn default() -> Self {
        Self {
            non_interactive: std::env::var("XTASK_NON_INTERACTIVE").is_ok(),
            log_level: std::env::var("XTASK_LOG_LEVEL").unwrap_or_else(|_| "INFO".to_string()),
            web_port: 8000,
            collab_port: 9000,
        }
    }
}

pub static CONFIG: Lazy<Mutex<XTaskConfig>> = Lazy::new(|| Mutex::new(XTaskConfig::default()));

pub fn is_non_interactive() -> bool {
    CONFIG.lock().map(|c| c.non_interactive).unwrap_or(false)
}
