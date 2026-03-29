use super::*;
use crate::config::profiles::VirtualizationPolicyScopeProfile;

impl KernelConfig {
    #[inline(always)]
    fn virtualization_scope_label(runtime_limited: bool, cargo_limited: bool) -> &'static str {
        match (runtime_limited, cargo_limited) {
            (false, false) => POLICY_SCOPE_FULLY_ENABLED,
            (true, false) => POLICY_SCOPE_RUNTIME_LIMITED,
            (false, true) => POLICY_SCOPE_COMPILETIME_LIMITED,
            (true, true) => POLICY_SCOPE_MIXED_LIMITS,
        }
    }

    #[inline(always)]
    fn virtualization_feature_policy_scope(
        runtime_enabled: bool,
        cargo_enabled: bool,
    ) -> &'static str {
        match (runtime_enabled, cargo_enabled) {
            (true, true) => POLICY_SCOPE_FULLY_ENABLED,
            (false, true) => POLICY_SCOPE_RUNTIME_LIMITED,
            (true, false) => POLICY_SCOPE_COMPILETIME_LIMITED,
            (false, false) => POLICY_SCOPE_FULLY_DISABLED,
        }
    }

    pub fn virtualization_execution_policy_scope() -> &'static str {
        let profile = Self::virtualization_execution_policy_profile();
        let runtime_limited = profile.runtime != profile.effective;
        let cargo_limited = profile.cargo != profile.effective;
        Self::virtualization_scope_label(runtime_limited, cargo_limited)
    }

    pub fn virtualization_governor_policy_scope() -> &'static str {
        let profile = Self::virtualization_governor_policy_profile();
        let runtime_limited = profile.runtime != profile.effective;
        let cargo_limited = profile.cargo != profile.effective;
        Self::virtualization_scope_label(runtime_limited, cargo_limited)
    }

    pub fn virtualization_policy_scope() -> &'static str {
        let policy = Self::virtualization_policy_profile();
        let runtime_limited = policy.runtime != policy.effective;
        let cargo_limited = policy.cargo != policy.effective;
        match (runtime_limited, cargo_limited) {
            (false, false)
                if policy.effective.snapshot
                    || policy.effective.nested
                    || policy.effective.time_virtualization
                    || policy.effective.device_passthrough
                    || policy.effective.dirty_logging
                    || policy.effective.live_migration
                    || policy.effective.trap_tracing =>
            {
                POLICY_SCOPE_FULLY_ENABLED
            }
            (false, false) => POLICY_SCOPE_FULLY_DISABLED,
            (true, false) => POLICY_SCOPE_RUNTIME_LIMITED,
            (false, true) => POLICY_SCOPE_COMPILETIME_LIMITED,
            (true, true) => POLICY_SCOPE_MIXED_LIMITS,
        }
    }

    pub fn virtualization_policy_scope_profile() -> VirtualizationPolicyScopeProfile {
        let policy = Self::virtualization_policy_profile();
        VirtualizationPolicyScopeProfile {
            overall: Self::virtualization_policy_scope(),
            entry: Self::virtualization_feature_policy_scope(
                policy.runtime.entry,
                policy.cargo.entry,
            ),
            resume: Self::virtualization_feature_policy_scope(
                policy.runtime.resume,
                policy.cargo.resume,
            ),
            trap_dispatch: Self::virtualization_feature_policy_scope(
                policy.runtime.trap_dispatch,
                policy.cargo.trap_dispatch,
            ),
            nested: Self::virtualization_feature_policy_scope(
                policy.runtime.nested,
                policy.cargo.nested,
            ),
            time_virtualization: Self::virtualization_feature_policy_scope(
                policy.runtime.time_virtualization,
                policy.cargo.time_virtualization,
            ),
            device_passthrough: Self::virtualization_feature_policy_scope(
                policy.runtime.device_passthrough,
                policy.cargo.device_passthrough,
            ),
            snapshot: Self::virtualization_feature_policy_scope(
                policy.runtime.snapshot,
                policy.cargo.snapshot,
            ),
            dirty_logging: Self::virtualization_feature_policy_scope(
                policy.runtime.dirty_logging,
                policy.cargo.dirty_logging,
            ),
            live_migration: Self::virtualization_feature_policy_scope(
                policy.runtime.live_migration,
                policy.cargo.live_migration,
            ),
            trap_tracing: Self::virtualization_feature_policy_scope(
                policy.runtime.trap_tracing,
                policy.cargo.trap_tracing,
            ),
        }
    }
}
