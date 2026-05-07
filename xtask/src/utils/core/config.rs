use std::env;
use std::sync::atomic::{AtomicBool, Ordering};

static NON_INTERACTIVE: AtomicBool = AtomicBool::new(false);

/// Centralized runtime configuration helpers for xtask.
/// Read values from environment variables with sane defaults so code
/// throughout the xtask binary can consult a single source of defaults.

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
