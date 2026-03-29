use core::sync::atomic::Ordering;

use super::*;

impl KernelConfig {
    pub fn merge_virtualization_profiles(
        runtime: VirtualizationRuntimeProfile,
        cargo: VirtualizationRuntimeProfile,
    ) -> VirtualizationRuntimeProfile {
        VirtualizationRuntimeProfile {
            telemetry: runtime.telemetry && cargo.telemetry,
            platform_lifecycle: runtime.platform_lifecycle && cargo.platform_lifecycle,
            entry: runtime.entry && cargo.entry,
            resume: runtime.resume && cargo.resume,
            trap_dispatch: runtime.trap_dispatch && cargo.trap_dispatch,
            nested: runtime.nested && cargo.nested,
            time_virtualization: runtime.time_virtualization && cargo.time_virtualization,
            device_passthrough: runtime.device_passthrough && cargo.device_passthrough,
            snapshot: runtime.snapshot && cargo.snapshot,
            dirty_logging: runtime.dirty_logging && cargo.dirty_logging,
            live_migration: runtime.live_migration && cargo.live_migration,
            trap_tracing: runtime.trap_tracing && cargo.trap_tracing,
        }
    }

    pub fn set_virtualization_snapshot_enabled(value: Option<bool>) {
        VIRTUALIZATION_SNAPSHOT_OVERRIDE.store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_virtualization_entry_enabled(value: Option<bool>) {
        VIRTUALIZATION_ENTRY_OVERRIDE.store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_virtualization_resume_enabled(value: Option<bool>) {
        VIRTUALIZATION_RESUME_OVERRIDE.store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_virtualization_trap_dispatch_enabled(value: Option<bool>) {
        VIRTUALIZATION_TRAP_DISPATCH_OVERRIDE.store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_virtualization_nested_enabled(value: Option<bool>) {
        VIRTUALIZATION_NESTED_OVERRIDE.store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_virtualization_time_virtualization_enabled(value: Option<bool>) {
        VIRTUALIZATION_TIME_VIRTUALIZATION_OVERRIDE
            .store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_virtualization_device_passthrough_enabled(value: Option<bool>) {
        VIRTUALIZATION_DEVICE_PASSTHROUGH_OVERRIDE
            .store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_virtualization_dirty_logging_enabled(value: Option<bool>) {
        VIRTUALIZATION_DIRTY_LOGGING_OVERRIDE.store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_virtualization_live_migration_enabled(value: Option<bool>) {
        VIRTUALIZATION_LIVE_MIGRATION_OVERRIDE
            .store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_virtualization_trap_tracing_enabled(value: Option<bool>) {
        VIRTUALIZATION_TRAP_TRACING_OVERRIDE.store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn virtualization_runtime_profile() -> VirtualizationRuntimeProfile {
        VirtualizationRuntimeProfile {
            telemetry: Self::telemetry_virtualization_enabled(),
            platform_lifecycle: Self::telemetry_platform_lifecycle_enabled(),
            entry: Self::virtualization_entry_enabled(),
            resume: Self::virtualization_resume_enabled(),
            trap_dispatch: Self::virtualization_trap_dispatch_enabled(),
            nested: Self::virtualization_nested_enabled(),
            time_virtualization: Self::virtualization_time_virtualization_enabled(),
            device_passthrough: Self::virtualization_device_passthrough_enabled(),
            snapshot: Self::virtualization_snapshot_enabled(),
            dirty_logging: Self::virtualization_dirty_logging_enabled(),
            live_migration: Self::virtualization_live_migration_enabled(),
            trap_tracing: Self::virtualization_trap_tracing_enabled(),
        }
    }

    pub fn virtualization_cargo_profile() -> VirtualizationRuntimeProfile {
        VirtualizationRuntimeProfile {
            telemetry: TELEMETRY_RUNTIME_SUMMARY,
            platform_lifecycle: TELEMETRY_RUNTIME_SUMMARY,
            entry: true,
            resume: true,
            trap_dispatch: true,
            nested: true,
            time_virtualization: true,
            device_passthrough: true,
            snapshot: true,
            dirty_logging: true,
            live_migration: true,
            trap_tracing: true,
        }
    }

    pub fn virtualization_effective_profile() -> VirtualizationRuntimeProfile {
        Self::merge_virtualization_profiles(
            Self::virtualization_runtime_profile(),
            Self::virtualization_cargo_profile(),
        )
    }

    pub fn virtualization_policy_profile() -> VirtualizationPolicyProfile {
        let runtime = Self::virtualization_runtime_profile();
        let cargo = Self::virtualization_cargo_profile();
        VirtualizationPolicyProfile {
            runtime,
            cargo,
            effective: Self::merge_virtualization_profiles(runtime, cargo),
        }
    }

    pub fn set_virtualization_runtime_profile(value: Option<VirtualizationRuntimeProfile>) {
        apply_profile_override(
            value,
            |profile| {
                Self::set_telemetry_virtualization_enabled(Some(profile.telemetry));
                Self::set_telemetry_platform_lifecycle_enabled(Some(profile.platform_lifecycle));
                Self::set_virtualization_entry_enabled(Some(profile.entry));
                Self::set_virtualization_resume_enabled(Some(profile.resume));
                Self::set_virtualization_trap_dispatch_enabled(Some(profile.trap_dispatch));
                Self::set_virtualization_nested_enabled(Some(profile.nested));
                Self::set_virtualization_time_virtualization_enabled(Some(
                    profile.time_virtualization,
                ));
                Self::set_virtualization_device_passthrough_enabled(Some(
                    profile.device_passthrough,
                ));
                Self::set_virtualization_snapshot_enabled(Some(profile.snapshot));
                Self::set_virtualization_dirty_logging_enabled(Some(profile.dirty_logging));
                Self::set_virtualization_live_migration_enabled(Some(profile.live_migration));
                Self::set_virtualization_trap_tracing_enabled(Some(profile.trap_tracing));
            },
            || {
                Self::set_telemetry_virtualization_enabled(None);
                Self::set_telemetry_platform_lifecycle_enabled(None);
                Self::set_virtualization_entry_enabled(None);
                Self::set_virtualization_resume_enabled(None);
                Self::set_virtualization_trap_dispatch_enabled(None);
                Self::set_virtualization_nested_enabled(None);
                Self::set_virtualization_time_virtualization_enabled(None);
                Self::set_virtualization_device_passthrough_enabled(None);
                Self::set_virtualization_snapshot_enabled(None);
                Self::set_virtualization_dirty_logging_enabled(None);
                Self::set_virtualization_live_migration_enabled(None);
                Self::set_virtualization_trap_tracing_enabled(None);
            },
        );
    }
}
