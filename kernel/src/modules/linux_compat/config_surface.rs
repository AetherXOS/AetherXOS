extern crate alloc;

use alloc::string::{String, ToString};
#[cfg(feature = "vfs")]
use core::sync::atomic::AtomicU64;

pub const DEFAULT_COMPAT_SURFACE_MOUNT_PATH: &str = "/proc";
pub const COMPAT_SURFACE_SOURCE_NAME: &str = "aethercore-compat-surface";
#[cfg(feature = "vfs")]
const DEFAULT_COMPAT_SURFACE_REFRESH_INTERVAL_TICKS: u64 = 1024;
#[cfg(feature = "vfs")]
static COMPAT_SURFACE_REFRESH_EPOCH: AtomicU64 = AtomicU64::new(0);

mod config_surface_export;
mod config_surface_paths;
mod config_surface_render;
mod config_surface_vfs;

pub use self::config_surface_export::*;
pub use self::config_surface_paths::*;
pub use self::config_surface_render::*;
pub use self::config_surface_vfs::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatConfigSurfaceKind {
    ProcConfig,
    Sysctl,
}

#[derive(Debug, Clone, Copy)]
pub struct CompatConfigSurfaceSnapshot {
    pub kind: CompatConfigSurfaceKind,
    pub enabled: bool,
    pub attack_surface_budget: u8,
    pub feature_count: usize,
}

impl CompatConfigSurfaceKind {
    pub const fn name(self) -> &'static str {
        match self {
            Self::ProcConfig => "proc_config",
            Self::Sysctl => "sysctl",
        }
    }

    pub const fn mount_path(self) -> &'static str {
        match self {
            Self::ProcConfig => "/proc/aethercore/config",
            Self::Sysctl => "/proc/sys/aethercore",
        }
    }

    pub const fn runtime_gate_key(self) -> &'static str {
        match self {
            Self::ProcConfig => "proc_config_api_exposed",
            Self::Sysctl => "sysctl_api_exposed",
        }
    }
}

pub fn compat_config_surface_snapshot(
    kind: CompatConfigSurfaceKind,
) -> CompatConfigSurfaceSnapshot {
    let compat = crate::config::KernelConfig::compat_surface_profile();
    let enabled = match kind {
        CompatConfigSurfaceKind::ProcConfig => compat.expose_proc_config_api,
        CompatConfigSurfaceKind::Sysctl => compat.expose_sysctl_api,
    };

    CompatConfigSurfaceSnapshot {
        kind,
        enabled,
        attack_surface_budget: compat.attack_surface_budget,
        feature_count: crate::config::KernelConfig::feature_controls().len(),
    }
}

pub fn is_compat_config_surface_enabled(kind: CompatConfigSurfaceKind) -> bool {
    compat_config_surface_snapshot(kind).enabled
}

fn sanitize_path_component(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        let c = ch.to_ascii_lowercase();
        if matches!(c, '/' | '\\' | ':' | ' ' | '.') {
            out.push('_');
        } else {
            out.push(c);
        }
    }
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    out.trim_matches('_').to_string()
}

fn normalize_runtime_style_key(key: &str) -> String {
    let trimmed = key.trim();
    let mut out = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        let c = ch.to_ascii_lowercase();
        if matches!(c, '.' | '-' | ' ' | '/' | ':') {
            out.push('_');
        } else {
            out.push(c);
        }
    }
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    out.trim_matches('_').to_string()
}

fn syscall_abi_platform() -> &'static str {
    #[cfg(target_arch = "x86_64")]
    {
        "x86_64"
    }
    #[cfg(target_arch = "aarch64")]
    {
        "aarch64"
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        "unknown"
    }
}

#[cfg(test)]
mod config_surface_tests;
