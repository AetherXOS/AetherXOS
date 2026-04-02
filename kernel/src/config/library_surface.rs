use super::{BoundaryMode, CompatSurfaceProfile, KernelConfig};

impl KernelConfig {
    pub fn compat_surface_profile() -> CompatSurfaceProfile {
        let boundary_mode = Self::boundary_mode();
        let expose_proc_config_api = Self::is_proc_config_api_exposed();
        let expose_sysctl_api = Self::is_sysctl_api_exposed();
        let expose_linux_compat_surface = cfg!(feature = "linux_compat")
            && !matches!(boundary_mode, BoundaryMode::Strict)
            && (Self::is_vfs_library_api_exposed()
                || Self::is_network_library_api_exposed()
                || Self::is_ipc_library_api_exposed());

        CompatSurfaceProfile {
            expose_proc_config_api,
            expose_sysctl_api,
            expose_linux_compat_surface,
            attack_surface_budget: Self::compat_attack_surface_budget(),
        }
    }

    pub fn compat_attack_surface_budget() -> u8 {
        let mut budget = 0u8;
        if Self::is_vfs_library_api_exposed() {
            budget = budget.saturating_add(2);
        }
        if Self::is_network_library_api_exposed() {
            budget = budget.saturating_add(2);
        }
        if Self::is_ipc_library_api_exposed() {
            budget = budget.saturating_add(1);
        }
        if Self::is_proc_config_api_exposed() {
            budget = budget.saturating_add(1);
        }
        if Self::is_sysctl_api_exposed() {
            budget = budget.saturating_add(1);
        }
        if cfg!(feature = "linux_compat") {
            budget = budget.saturating_add(2);
        }
        budget
    }

    pub fn should_expose_procfs_surface() -> bool {
        Self::compat_surface_profile().expose_proc_config_api
    }

    pub fn should_expose_sysctl_surface() -> bool {
        Self::compat_surface_profile().expose_sysctl_api
    }

    pub fn should_expose_linux_compat_surface() -> bool {
        Self::compat_surface_profile().expose_linux_compat_surface
    }
}

#[cfg(all(test, target_os = "none"))]
mod tests {
    use super::*;

    #[test_case]
    fn strict_boundary_reduces_compat_surface() {
        KernelConfig::reset_runtime_overrides();
        KernelConfig::set_vfs_library_api_exposed(Some(true));
        KernelConfig::set_proc_config_api_exposed(Some(true));
        KernelConfig::set_sysctl_api_exposed(Some(true));
        KernelConfig::set_library_boundary_mode(Some(BoundaryMode::Strict));

        let profile = KernelConfig::compat_surface_profile();
        assert!(!profile.expose_linux_compat_surface);
        assert!(profile.attack_surface_budget >= 2);

        KernelConfig::reset_runtime_overrides();
    }
}
