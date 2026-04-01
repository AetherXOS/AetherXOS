use super::feature_catalog::LibraryCompileProfile;
use super::{
    CompatSurfaceProfile, ConfigSetError, CoreRuntimeLimits,
    CredentialRuntimeProfile, DevFsRuntimeProfile, DriverNetworkRuntimeProfile, KernelConfig,
    LibraryRuntimeFeatureProfile, NetworkRuntimeProfile, RuntimePolicyDriftRuntimeProfile,
    SchedulerRuntimeProfile, TelemetryRuntimeProfile, VfsRuntimeProfile,
};
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

#[path = "control_plane_support.rs"]
mod support;
#[path = "control_plane_exports.rs"]
mod exports;
#[path = "control_plane_batch.rs"]
mod batch;
#[path = "control_plane_features.rs"]
mod features;
#[path = "control_plane_utils.rs"]
mod utils;


static CONFIG_APPLY_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static CONFIG_APPLY_SUCCESS: AtomicU64 = AtomicU64::new(0);
static CONFIG_APPLY_FAILURES: AtomicU64 = AtomicU64::new(0);
static CONFIG_LAST_ERROR_INDEX: AtomicUsize = AtomicUsize::new(usize::MAX);


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigAuditStats {
    pub apply_attempts: u64,
    pub apply_success: u64,
    pub apply_failures: u64,
    pub last_error_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigBatchApplyError {
    pub index: usize,
    pub key: String,
    pub raw_entry: String,
    pub cause: ConfigSetError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigFeatureControl {
    pub name: &'static str,
    pub category: &'static str,
    pub compile_enabled: bool,
    pub runtime_gate_key: Option<&'static str>,
    pub runtime_gate_available: bool,
    pub effective_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigFeatureCategorySummary {
    pub category: &'static str,
    pub total: usize,
    pub compile_enabled: usize,
    pub runtime_gateable: usize,
    pub effective_enabled: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigLinuxCompatReadiness {
    pub compile_linux_compat: bool,
    pub compile_vfs: bool,
    pub boundary_allows_compat: bool,
    pub vfs_api_exposed: bool,
    pub network_api_exposed: bool,
    pub ipc_api_exposed: bool,
    pub proc_config_api_exposed: bool,
    pub sysctl_api_exposed: bool,
    pub effective_surface_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConfigBlockerSeverity {
    Critical,
    High,
    Medium,
}

impl ConfigBlockerSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::High => "high",
            Self::Medium => "medium",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigLinuxCompatBlocker {
    pub code: &'static str,
    pub severity: ConfigBlockerSeverity,
    pub next_action: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigOverridePreviewEntry {
    pub index: usize,
    pub key: String,
    pub value: Option<String>,
    pub category: Option<&'static str>,
    pub critical: bool,
    pub valid: bool,
    pub cause: Option<ConfigSetError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigOverridePreviewSummary {
    pub total: usize,
    pub valid: usize,
    pub invalid: usize,
    pub critical_touched: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct KernelConfigSnapshot {
    pub core: CoreRuntimeLimits,
    pub network: NetworkRuntimeProfile,
    pub scheduler: SchedulerRuntimeProfile,
    pub driver_network: DriverNetworkRuntimeProfile,
    pub telemetry: TelemetryRuntimeProfile,
    pub vfs: VfsRuntimeProfile,
    pub devfs: DevFsRuntimeProfile,
    pub runtime_policy_drift: RuntimePolicyDriftRuntimeProfile,
    pub library_runtime: LibraryRuntimeFeatureProfile,
    pub library_compile: LibraryCompileProfile,
    pub compat_surface: CompatSurfaceProfile,
    pub credentials: CredentialRuntimeProfile,
}

impl KernelConfig {
    pub fn audit_stats() -> ConfigAuditStats {
        let raw = CONFIG_LAST_ERROR_INDEX.load(Ordering::Relaxed);
        ConfigAuditStats {
            apply_attempts: CONFIG_APPLY_ATTEMPTS.load(Ordering::Relaxed),
            apply_success: CONFIG_APPLY_SUCCESS.load(Ordering::Relaxed),
            apply_failures: CONFIG_APPLY_FAILURES.load(Ordering::Relaxed),
            last_error_index: if raw == usize::MAX { None } else { Some(raw) },
        }
    }

    pub fn snapshot() -> KernelConfigSnapshot {
        KernelConfigSnapshot {
            core: Self::runtime_limits(),
            network: Self::network_runtime_profile(),
            scheduler: Self::scheduler_runtime_profile(),
            driver_network: Self::driver_network_runtime_profile(),
            telemetry: Self::telemetry_runtime_profile(),
            vfs: Self::vfs_runtime_profile(),
            devfs: Self::devfs_runtime_profile(),
            runtime_policy_drift: Self::runtime_policy_drift_runtime_profile(),
            library_runtime: Self::library_runtime_feature_profile(),
            library_compile: Self::library_compile_profile(),
            compat_surface: Self::compat_surface_profile(),
            credentials: Self::credential_runtime_profile(),
        }
    }

    pub fn linux_compat_readiness() -> ConfigLinuxCompatReadiness {
        let compile_linux_compat = cfg!(feature = "linux_compat");
        let compile_vfs = cfg!(feature = "vfs");
        let boundary_allows_compat = !matches!(Self::boundary_mode(), super::BoundaryMode::Strict);
        let vfs_api_exposed = Self::is_vfs_library_api_exposed();
        let network_api_exposed = Self::is_network_library_api_exposed();
        let ipc_api_exposed = Self::is_ipc_library_api_exposed();
        let proc_config_api_exposed = Self::is_proc_config_api_exposed();
        let sysctl_api_exposed = Self::is_sysctl_api_exposed();
        let has_surface_inputs = vfs_api_exposed || network_api_exposed || ipc_api_exposed;
        let effective_surface_enabled = compile_linux_compat && boundary_allows_compat && has_surface_inputs;

        ConfigLinuxCompatReadiness {
            compile_linux_compat,
            compile_vfs,
            boundary_allows_compat,
            vfs_api_exposed,
            network_api_exposed,
            ipc_api_exposed,
            proc_config_api_exposed,
            sysctl_api_exposed,
            effective_surface_enabled,
        }
    }

    pub fn linux_compat_blockers() -> Vec<&'static str> {
        let details = Self::linux_compat_blocker_details();
        let mut out = Vec::with_capacity(details.len());
        for item in details {
            out.push(item.code);
        }
        out
    }

    pub fn linux_compat_blocker_details() -> Vec<ConfigLinuxCompatBlocker> {
        let readiness = Self::linux_compat_readiness();
        let mut blockers = Vec::new();
        if !readiness.compile_linux_compat {
            blockers.push(ConfigLinuxCompatBlocker {
                code: "compile_time_missing_linux_compat_feature",
                severity: ConfigBlockerSeverity::Critical,
                next_action: "enable cargo feature linux_compat and rebuild",
            });
        }
        if !readiness.compile_vfs {
            blockers.push(ConfigLinuxCompatBlocker {
                code: "compile_time_missing_vfs_feature",
                severity: ConfigBlockerSeverity::Critical,
                next_action: "enable cargo feature vfs and rebuild",
            });
        }
        if !readiness.boundary_allows_compat {
            blockers.push(ConfigLinuxCompatBlocker {
                code: "boundary_mode_strict_blocks_compat_surface",
                severity: ConfigBlockerSeverity::High,
                next_action: "set library_boundary_mode to Balanced or Compat at runtime",
            });
        }
        if !readiness.vfs_api_exposed
            && !readiness.network_api_exposed
            && !readiness.ipc_api_exposed
        {
            blockers.push(ConfigLinuxCompatBlocker {
                code: "no_library_surface_exposed_for_compat",
                severity: ConfigBlockerSeverity::High,
                next_action: "enable at least one of vfs_library_api_exposed/network_library_api_exposed/ipc_library_api_exposed",
            });
        }

        blockers.sort_by(|a, b| a.severity.cmp(&b.severity).then_with(|| a.code.cmp(b.code)));
        blockers
    }

    pub fn linux_compat_next_action() -> &'static str {
        let blockers = Self::linux_compat_blocker_details();
        blockers
            .first()
            .map(|item| item.next_action)
            .unwrap_or("linux compatibility surface is ready; proceed with syscall/ABI coverage expansion")
    }

}


