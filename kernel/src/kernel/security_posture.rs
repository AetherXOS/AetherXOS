use alloc::vec::Vec;

use crate::config::{BoundaryMode, KernelConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityDeploymentContext {
    DevelopmentFlex,
    StagingCompat,
    ProductionHardened,
}

impl SecurityDeploymentContext {
    pub fn as_str(self) -> &'static str {
        match self {
            SecurityDeploymentContext::DevelopmentFlex => "development-flex",
            SecurityDeploymentContext::StagingCompat => "staging-compat",
            SecurityDeploymentContext::ProductionHardened => "production-hardened",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecurityPostureSnapshot {
    pub deployment_context: SecurityDeploymentContext,
    pub boundary_mode: BoundaryMode,
    pub security_enforcement_enabled: bool,
    pub capability_enforcement_enabled: bool,
    pub multi_user_enabled: bool,
    pub library_expose_vfs_api: bool,
    pub library_expose_network_api: bool,
    pub library_expose_ipc_api: bool,
    pub library_expose_proc_config_api: bool,
    pub library_expose_sysctl_api: bool,
    pub exec_elf_require_absolute_interp_path: bool,
    pub exec_elf_enforce_interp_path_sanitization: bool,
    pub exec_elf_enforce_system_loader_paths: bool,
    pub exec_elf_enforce_segment_congruence: bool,
    pub exec_auxv_enforce_handoff_contract: bool,
    pub exec_auxv_require_phdr_triplet: bool,
    pub namespace_support_enabled: bool,
    pub namespace_lifecycle_sane: bool,
    pub namespace_policy_passed: bool,
    pub namespace_creates: u64,
    pub namespace_destroys: u64,
    pub namespace_unshare_calls: u64,
    pub namespace_setns_calls: u64,
    pub expose_linux_compat_surface: bool,
    pub compat_attack_surface_budget: u8,
    pub syscall_contract_checks: u32,
    pub syscall_contract_failures: u32,
    pub syscall_contract_passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityStrictGateReport {
    pub passed: bool,
    pub deployment_context: SecurityDeploymentContext,
    pub reasons: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityReleaseGateDecision {
    pub blocked: bool,
    pub deployment_context: SecurityDeploymentContext,
    pub reasons: Vec<&'static str>,
}

#[inline(always)]
fn detect_deployment_context(
    boundary_mode: BoundaryMode,
    security_enforcement_enabled: bool,
    capability_enforcement_enabled: bool,
    multi_user_enabled: bool,
) -> SecurityDeploymentContext {
    if matches!(boundary_mode, BoundaryMode::Strict)
        && security_enforcement_enabled
        && capability_enforcement_enabled
        && multi_user_enabled
    {
        return SecurityDeploymentContext::ProductionHardened;
    }
    if matches!(boundary_mode, BoundaryMode::Balanced) && security_enforcement_enabled {
        return SecurityDeploymentContext::StagingCompat;
    }
    SecurityDeploymentContext::DevelopmentFlex
}

#[inline(always)]
pub fn current_snapshot() -> SecurityPostureSnapshot {
    let namespace_stats = crate::kernel::namespaces::namespace_stats();
    let syscall_contract = crate::kernel::syscall_contract::run_syscall_contract_self_test();
    let boundary_mode = KernelConfig::boundary_mode();
    let security_enforcement_enabled = KernelConfig::security_enforcement_enabled();
    let capability_enforcement_enabled = KernelConfig::capability_enforcement_enabled();
    let multi_user_enabled = KernelConfig::multi_user_enabled();
    let deployment_context = detect_deployment_context(
        boundary_mode,
        security_enforcement_enabled,
        capability_enforcement_enabled,
        multi_user_enabled,
    );
    let compat_surface = KernelConfig::compat_surface_profile();
    let namespace_support_enabled = namespace_stats.creates >= 7
        || namespace_stats.unshare_calls > 0
        || namespace_stats.setns_calls > 0;
    let namespace_lifecycle_sane = namespace_stats.creates >= namespace_stats.destroys;
    let namespace_policy_passed = namespace_support_enabled
        && namespace_lifecycle_sane
        && namespace_stats.creates >= 7;

    SecurityPostureSnapshot {
        deployment_context,
        boundary_mode,
        security_enforcement_enabled,
        capability_enforcement_enabled,
        multi_user_enabled,
        library_expose_vfs_api: KernelConfig::is_vfs_library_api_exposed(),
        library_expose_network_api: KernelConfig::is_network_library_api_exposed(),
        library_expose_ipc_api: KernelConfig::is_ipc_library_api_exposed(),
        library_expose_proc_config_api: KernelConfig::is_proc_config_api_exposed(),
        library_expose_sysctl_api: KernelConfig::is_sysctl_api_exposed(),
        exec_elf_require_absolute_interp_path: KernelConfig::exec_elf_require_absolute_interp_path(),
        exec_elf_enforce_interp_path_sanitization: KernelConfig::exec_elf_enforce_interp_path_sanitization(),
        exec_elf_enforce_system_loader_paths: KernelConfig::exec_elf_enforce_system_loader_paths(),
        exec_elf_enforce_segment_congruence: KernelConfig::exec_elf_enforce_segment_congruence(),
        exec_auxv_enforce_handoff_contract: KernelConfig::exec_auxv_enforce_handoff_contract(),
        exec_auxv_require_phdr_triplet: KernelConfig::exec_auxv_require_phdr_triplet(),
        namespace_support_enabled,
        namespace_lifecycle_sane,
        namespace_policy_passed,
        namespace_creates: namespace_stats.creates,
        namespace_destroys: namespace_stats.destroys,
        namespace_unshare_calls: namespace_stats.unshare_calls,
        namespace_setns_calls: namespace_stats.setns_calls,
        expose_linux_compat_surface: compat_surface.expose_linux_compat_surface,
        compat_attack_surface_budget: compat_surface.attack_surface_budget,
        syscall_contract_checks: syscall_contract.checks,
        syscall_contract_failures: syscall_contract.failures,
        syscall_contract_passed: syscall_contract.passed(),
    }
}

#[inline(always)]
pub fn strict_profile_gate_report() -> SecurityStrictGateReport {
    let snapshot = current_snapshot();
    let mut reasons = Vec::new();

    if !matches!(snapshot.boundary_mode, BoundaryMode::Strict) {
        reasons.push("library_boundary_mode_not_strict");
    }
    if !snapshot.security_enforcement_enabled {
        reasons.push("security_enforcement_disabled");
    }
    if !snapshot.capability_enforcement_enabled {
        reasons.push("capability_enforcement_disabled");
    }
    if !snapshot.multi_user_enabled {
        reasons.push("multi_user_disabled");
    }
    if !snapshot.exec_elf_require_absolute_interp_path {
        reasons.push("interp_absolute_path_relaxed");
    }
    if !snapshot.exec_elf_enforce_interp_path_sanitization {
        reasons.push("interp_path_sanitization_relaxed");
    }
    if !snapshot.exec_elf_enforce_system_loader_paths {
        reasons.push("system_loader_paths_relaxed");
    }
    if !snapshot.exec_elf_enforce_segment_congruence {
        reasons.push("segment_congruence_relaxed");
    }
    if !snapshot.exec_auxv_enforce_handoff_contract {
        reasons.push("auxv_handoff_contract_relaxed");
    }
    if !snapshot.exec_auxv_require_phdr_triplet {
        reasons.push("auxv_phdr_triplet_relaxed");
    }
    if !snapshot.syscall_contract_passed {
        reasons.push("syscall_contract_failed");
    }
    if !snapshot.namespace_support_enabled {
        reasons.push("namespace_isolation_features_not_compiled");
    }
    if !snapshot.namespace_lifecycle_sane {
        reasons.push("namespace_lifecycle_counters_inconsistent");
    }
    if matches!(snapshot.deployment_context, SecurityDeploymentContext::ProductionHardened)
        && !snapshot.namespace_policy_passed
    {
        reasons.push("namespace_policy_not_satisfied_for_prod");
    }
    if matches!(snapshot.deployment_context, SecurityDeploymentContext::ProductionHardened)
        && snapshot.expose_linux_compat_surface
    {
        reasons.push("production_context_exposes_linux_compat_surface");
    }
    if snapshot.compat_attack_surface_budget > 6 {
        reasons.push("compat_attack_surface_budget_above_guardrail");
    }

    SecurityStrictGateReport {
        passed: reasons.is_empty(),
        deployment_context: snapshot.deployment_context,
        reasons,
    }
}

#[inline(always)]
pub fn release_gate_decision() -> SecurityReleaseGateDecision {
    let strict = strict_profile_gate_report();
    let blocked = !strict.passed
        && matches!(
            strict.deployment_context,
            SecurityDeploymentContext::StagingCompat
                | SecurityDeploymentContext::ProductionHardened
        );

    SecurityReleaseGateDecision {
        blocked,
        deployment_context: strict.deployment_context,
        reasons: strict.reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn current_snapshot_includes_policy_and_loader_controls() {
        crate::config::KernelConfig::reset_runtime_overrides();

        let snapshot = current_snapshot();
        assert_eq!(snapshot.boundary_mode, KernelConfig::boundary_mode());
        assert_eq!(snapshot.library_expose_vfs_api, KernelConfig::is_vfs_library_api_exposed());
        assert_eq!(snapshot.library_expose_network_api, KernelConfig::is_network_library_api_exposed());
        assert_eq!(snapshot.library_expose_ipc_api, KernelConfig::is_ipc_library_api_exposed());
        assert_eq!(snapshot.exec_elf_require_absolute_interp_path, KernelConfig::exec_elf_require_absolute_interp_path());
        assert_eq!(snapshot.exec_auxv_enforce_handoff_contract, KernelConfig::exec_auxv_enforce_handoff_contract());
        assert_eq!(snapshot.syscall_contract_passed, snapshot.syscall_contract_failures == 0);
        assert!(matches!(
            snapshot.deployment_context,
            SecurityDeploymentContext::DevelopmentFlex
                | SecurityDeploymentContext::StagingCompat
                | SecurityDeploymentContext::ProductionHardened
        ));
        assert!(snapshot.namespace_creates >= snapshot.namespace_destroys);
    }

    #[test_case]
    fn strict_profile_gate_report_collects_relaxed_controls() {
        crate::config::KernelConfig::reset_runtime_overrides();
        crate::config::KernelConfig::set_security_enforcement_enabled(Some(false));
        crate::config::KernelConfig::set_capability_enforcement_enabled(Some(false));
        crate::config::KernelConfig::set_multi_user_enabled(Some(false));
        crate::config::KernelConfig::set_exec_elf_require_absolute_interp_path(Some(false));
        crate::config::KernelConfig::set_exec_auxv_enforce_handoff_contract(Some(false));

        let report = strict_profile_gate_report();
        assert!(!report.passed);
        assert!(report.reasons.contains(&"security_enforcement_disabled"));
        assert!(report.reasons.contains(&"capability_enforcement_disabled"));
        assert!(report.reasons.contains(&"multi_user_disabled"));
        assert!(report.reasons.contains(&"interp_absolute_path_relaxed"));
        assert!(report.reasons.contains(&"auxv_handoff_contract_relaxed"));
        assert!(matches!(
            report.deployment_context,
            SecurityDeploymentContext::DevelopmentFlex
                | SecurityDeploymentContext::StagingCompat
                | SecurityDeploymentContext::ProductionHardened
        ));

        crate::config::KernelConfig::reset_runtime_overrides();
    }

    #[test_case]
    fn release_gate_blocks_for_staging_or_prod_when_strict_gate_fails() {
        crate::config::KernelConfig::reset_runtime_overrides();
        crate::config::KernelConfig::set_library_boundary_mode(Some(BoundaryMode::Balanced));
        crate::config::KernelConfig::set_security_enforcement_enabled(Some(false));

        let decision = release_gate_decision();
        assert!(decision.blocked);
        assert!(matches!(
            decision.deployment_context,
            SecurityDeploymentContext::StagingCompat | SecurityDeploymentContext::ProductionHardened
        ));
        assert!(!decision.reasons.is_empty());

        crate::config::KernelConfig::reset_runtime_overrides();
    }
}