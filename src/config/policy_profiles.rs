use core::sync::atomic::Ordering;

use super::*;

impl KernelConfig {
    pub fn set_debug_trace_enabled(value: Option<bool>) {
        DEBUG_TRACE_ENABLED_OVERRIDE.store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_serial_early_debug_enabled(value: Option<bool>) {
        SERIAL_EARLY_DEBUG_ENABLED_OVERRIDE.store(encode_bool_override(value), Ordering::Relaxed);
    }

    // Setters for granular category-level observability control
    pub fn set_observability_category_enabled(
        category: crate::config::ObservabilityCategory,
        value: Option<bool>,
    ) {
        let encoded = encode_bool_override(value);
        match category {
            crate::config::ObservabilityCategory::Core => {
                OBSERVABILITY_CORE_OVERRIDE.store(encoded, Ordering::Relaxed)
            }
            crate::config::ObservabilityCategory::Boot => {
                OBSERVABILITY_BOOT_OVERRIDE.store(encoded, Ordering::Relaxed)
            }
            crate::config::ObservabilityCategory::Loader => {
                OBSERVABILITY_LOADER_OVERRIDE.store(encoded, Ordering::Relaxed)
            }
            crate::config::ObservabilityCategory::Task => {
                OBSERVABILITY_TASK_OVERRIDE.store(encoded, Ordering::Relaxed)
            }
            crate::config::ObservabilityCategory::Memory => {
                OBSERVABILITY_MEMORY_OVERRIDE.store(encoded, Ordering::Relaxed)
            }
            crate::config::ObservabilityCategory::Scheduler => {
                OBSERVABILITY_SCHEDULER_OVERRIDE.store(encoded, Ordering::Relaxed)
            }
            crate::config::ObservabilityCategory::Fault => {
                OBSERVABILITY_FAULT_OVERRIDE.store(encoded, Ordering::Relaxed)
            }
            crate::config::ObservabilityCategory::Driver => {
                OBSERVABILITY_DRIVER_OVERRIDE.store(encoded, Ordering::Relaxed)
            }
            crate::config::ObservabilityCategory::Io => {
                OBSERVABILITY_IO_OVERRIDE.store(encoded, Ordering::Relaxed)
            }
            crate::config::ObservabilityCategory::Network => {
                OBSERVABILITY_NETWORK_OVERRIDE.store(encoded, Ordering::Relaxed)
            }
        }
    }

    pub fn set_telemetry_runtime_summary_enabled(value: Option<bool>) {
        TELEMETRY_RUNTIME_SUMMARY_OVERRIDE.store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_telemetry_vfs_enabled(value: Option<bool>) {
        TELEMETRY_VFS_OVERRIDE.store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn virtualization_snapshot_enabled() -> bool {
        Self::is_virtualization_enabled()
            && decode_bool_override(
                VIRTUALIZATION_SNAPSHOT_OVERRIDE.load(Ordering::Relaxed),
                true,
            )
    }

    pub fn virtualization_entry_enabled() -> bool {
        Self::is_virtualization_enabled()
            && decode_bool_override(VIRTUALIZATION_ENTRY_OVERRIDE.load(Ordering::Relaxed), true)
    }

    pub fn virtualization_resume_enabled() -> bool {
        Self::is_virtualization_enabled()
            && decode_bool_override(VIRTUALIZATION_RESUME_OVERRIDE.load(Ordering::Relaxed), true)
    }

    pub fn virtualization_trap_dispatch_enabled() -> bool {
        Self::is_virtualization_enabled()
            && decode_bool_override(
                VIRTUALIZATION_TRAP_DISPATCH_OVERRIDE.load(Ordering::Relaxed),
                true,
            )
    }

    pub fn virtualization_nested_enabled() -> bool {
        Self::is_virtualization_enabled()
            && decode_bool_override(VIRTUALIZATION_NESTED_OVERRIDE.load(Ordering::Relaxed), true)
    }

    pub fn virtualization_time_virtualization_enabled() -> bool {
        Self::is_virtualization_enabled()
            && decode_bool_override(
                VIRTUALIZATION_TIME_VIRTUALIZATION_OVERRIDE.load(Ordering::Relaxed),
                true,
            )
    }

    pub fn virtualization_device_passthrough_enabled() -> bool {
        Self::is_virtualization_enabled()
            && decode_bool_override(
                VIRTUALIZATION_DEVICE_PASSTHROUGH_OVERRIDE.load(Ordering::Relaxed),
                true,
            )
    }

    pub fn virtualization_dirty_logging_enabled() -> bool {
        Self::is_virtualization_enabled()
            && decode_bool_override(
                VIRTUALIZATION_DIRTY_LOGGING_OVERRIDE.load(Ordering::Relaxed),
                true,
            )
    }

    pub fn virtualization_live_migration_enabled() -> bool {
        Self::is_virtualization_enabled()
            && decode_bool_override(
                VIRTUALIZATION_LIVE_MIGRATION_OVERRIDE.load(Ordering::Relaxed),
                true,
            )
    }

    pub fn virtualization_trap_tracing_enabled() -> bool {
        Self::is_virtualization_enabled()
            && decode_bool_override(
                VIRTUALIZATION_TRAP_TRACING_OVERRIDE.load(Ordering::Relaxed),
                true,
            )
    }

    pub fn set_telemetry_virtualization_enabled(value: Option<bool>) {
        TELEMETRY_VIRTUALIZATION_OVERRIDE.store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_telemetry_platform_lifecycle_enabled(value: Option<bool>) {
        TELEMETRY_PLATFORM_LIFECYCLE_OVERRIDE.store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_telemetry_network_enabled(value: Option<bool>) {
        TELEMETRY_NETWORK_OVERRIDE.store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_telemetry_ipc_enabled(value: Option<bool>) {
        TELEMETRY_IPC_OVERRIDE.store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_telemetry_scheduler_enabled(value: Option<bool>) {
        TELEMETRY_SCHEDULER_OVERRIDE.store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_telemetry_security_enabled(value: Option<bool>) {
        TELEMETRY_SECURITY_OVERRIDE.store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_telemetry_power_enabled(value: Option<bool>) {
        TELEMETRY_POWER_OVERRIDE.store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_telemetry_drivers_enabled(value: Option<bool>) {
        TELEMETRY_DRIVERS_OVERRIDE.store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_library_boundary_mode(value: Option<BoundaryMode>) {
        let encoded = match value {
            None => BOUNDARY_MODE_OVERRIDE_DEFAULT,
            Some(BoundaryMode::Strict) => BOUNDARY_MODE_OVERRIDE_STRICT,
            Some(BoundaryMode::Balanced) => BOUNDARY_MODE_OVERRIDE_BALANCED,
            Some(BoundaryMode::Compat) => BOUNDARY_MODE_OVERRIDE_COMPAT,
        };
        LIBRARY_BOUNDARY_MODE_OVERRIDE.store(encoded, Ordering::Relaxed);
    }

    pub fn set_core_minimal_enforced(value: Option<bool>) {
        LIBRARY_ENFORCE_CORE_MINIMAL_OVERRIDE.store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_strict_optional_features_enabled(value: Option<bool>) {
        LIBRARY_STRICT_OPTIONAL_FEATURES_OVERRIDE
            .store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_vfs_library_api_exposed(value: Option<bool>) {
        LIBRARY_EXPOSE_VFS_API_OVERRIDE.store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_network_library_api_exposed(value: Option<bool>) {
        LIBRARY_EXPOSE_NETWORK_API_OVERRIDE.store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_ipc_library_api_exposed(value: Option<bool>) {
        LIBRARY_EXPOSE_IPC_API_OVERRIDE.store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_proc_config_api_exposed(value: Option<bool>) {
        LIBRARY_EXPOSE_PROC_CONFIG_API_OVERRIDE
            .store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_sysctl_api_exposed(value: Option<bool>) {
        LIBRARY_EXPOSE_SYSCTL_API_OVERRIDE.store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_security_enforcement_enabled(value: Option<bool>) {
        SECURITY_ENFORCEMENT_ENABLED_OVERRIDE.store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_capability_enforcement_enabled(value: Option<bool>) {
        CAPABILITY_ENFORCEMENT_ENABLED_OVERRIDE
            .store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_multi_user_enabled(value: Option<bool>) {
        MULTI_USER_ENABLED_OVERRIDE.store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_credential_enforcement_enabled(value: Option<bool>) {
        CREDENTIAL_ENFORCEMENT_ENABLED_OVERRIDE
            .store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_libnet_fast_path_run_pump(value: Option<bool>) {
        LIBNET_FAST_PATH_RUN_PUMP_OVERRIDE.store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_libnet_fast_path_collect_transport_snapshot(value: Option<bool>) {
        LIBNET_FAST_PATH_COLLECT_TRANSPORT_SNAPSHOT_OVERRIDE
            .store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn library_runtime_feature_profile() -> LibraryRuntimeFeatureProfile {
        LibraryRuntimeFeatureProfile {
            boundary_mode: Self::boundary_mode(),
            enforce_core_minimal: Self::is_core_minimal_enforced(),
            strict_optional_features: Self::is_strict_optional_features_enabled(),
            expose_vfs_api: Self::is_vfs_library_api_exposed(),
            expose_network_api: Self::is_network_library_api_exposed(),
            expose_ipc_api: Self::is_ipc_library_api_exposed(),
            expose_proc_config_api: Self::is_proc_config_api_exposed(),
            expose_sysctl_api: Self::is_sysctl_api_exposed(),
            libnet_fast_path_run_pump: Self::libnet_fast_path_run_pump(),
            libnet_fast_path_collect_transport_snapshot:
                Self::libnet_fast_path_collect_transport_snapshot(),
        }
    }

    pub fn credential_runtime_profile() -> CredentialRuntimeProfile {
        CredentialRuntimeProfile {
            security_enforcement: Self::security_enforcement_enabled(),
            capability_enforcement: Self::capability_enforcement_enabled(),
            multi_user: Self::multi_user_enabled(),
            credential_enforcement: Self::credential_enforcement_enabled(),
        }
    }

    pub fn library_cargo_feature_profile() -> LibraryRuntimeFeatureProfile {
        LibraryRuntimeFeatureProfile {
            boundary_mode: BoundaryMode::from_str(LIBRARY_BOUNDARY_MODE),
            enforce_core_minimal: LIBRARY_ENFORCE_CORE_MINIMAL,
            strict_optional_features: LIBRARY_STRICT_OPTIONAL_FEATURES,
            expose_vfs_api: LIBRARY_EXPOSE_VFS_API,
            expose_network_api: LIBRARY_EXPOSE_NETWORK_API,
            expose_ipc_api: LIBRARY_EXPOSE_IPC_API,
            expose_proc_config_api: LIBRARY_EXPOSE_VFS_API
                && !matches!(
                    BoundaryMode::from_str(LIBRARY_BOUNDARY_MODE),
                    BoundaryMode::Strict
                ),
            expose_sysctl_api: LIBRARY_EXPOSE_VFS_API
                && !matches!(
                    BoundaryMode::from_str(LIBRARY_BOUNDARY_MODE),
                    BoundaryMode::Strict
                ),
            libnet_fast_path_run_pump: LIBNET_FAST_PATH_RUN_PUMP,
            libnet_fast_path_collect_transport_snapshot:
                LIBNET_FAST_PATH_COLLECT_TRANSPORT_SNAPSHOT,
        }
    }

    pub fn telemetry_runtime_profile() -> TelemetryRuntimeProfile {
        TelemetryRuntimeProfile {
            enabled: Self::is_telemetry_enabled(),
            runtime_summary: Self::telemetry_runtime_summary_enabled(),
            virtualization: Self::telemetry_virtualization_enabled(),
            platform_lifecycle: Self::telemetry_platform_lifecycle_enabled(),
            vfs: Self::telemetry_vfs_enabled(),
            network: Self::telemetry_network_enabled(),
            ipc: Self::telemetry_ipc_enabled(),
            scheduler: Self::telemetry_scheduler_enabled(),
            security: Self::telemetry_security_enabled(),
            power: Self::telemetry_power_enabled(),
            drivers: Self::telemetry_drivers_enabled(),
            debug_trace: Self::debug_trace_enabled(),
            early_serial_debug: Self::serial_early_debug_enabled(),
            history_len: Self::telemetry_history_len(),
            log_level_num: Self::log_level_num(),
        }
    }

    pub fn telemetry_cargo_profile() -> TelemetryRuntimeProfile {
        TelemetryRuntimeProfile {
            enabled: TELEMETRY_ENABLED,
            runtime_summary: TELEMETRY_RUNTIME_SUMMARY,
            virtualization: true,
            platform_lifecycle: true,
            vfs: TELEMETRY_ENABLE_VFS,
            network: TELEMETRY_ENABLE_NETWORK,
            ipc: TELEMETRY_ENABLE_IPC,
            scheduler: TELEMETRY_ENABLE_SCHEDULER,
            security: TELEMETRY_ENABLE_SECURITY,
            power: TELEMETRY_ENABLE_POWER,
            drivers: TELEMETRY_ENABLE_DRIVERS,
            debug_trace: KernelConfig::is_advanced_debug_enabled(),
            early_serial_debug: KernelConfig::is_advanced_debug_enabled(),
            history_len: DEFAULT_TELEMETRY_HISTORY_LEN.max(1),
            log_level_num: DEFAULT_TELEMETRY_LOG_LEVEL_NUM,
        }
    }

    pub fn set_telemetry_runtime_profile(value: Option<TelemetryRuntimeProfile>) {
        apply_profile_override(
            value,
            |profile| {
                Self::set_telemetry_enabled(Some(profile.enabled));
                Self::set_telemetry_runtime_summary_enabled(Some(profile.runtime_summary));
                Self::set_telemetry_virtualization_enabled(Some(profile.virtualization));
                Self::set_telemetry_platform_lifecycle_enabled(Some(profile.platform_lifecycle));
                Self::set_telemetry_vfs_enabled(Some(profile.vfs));
                Self::set_telemetry_network_enabled(Some(profile.network));
                Self::set_telemetry_ipc_enabled(Some(profile.ipc));
                Self::set_telemetry_scheduler_enabled(Some(profile.scheduler));
                Self::set_telemetry_security_enabled(Some(profile.security));
                Self::set_telemetry_power_enabled(Some(profile.power));
                Self::set_telemetry_drivers_enabled(Some(profile.drivers));
                Self::set_debug_trace_enabled(Some(profile.debug_trace));
                Self::set_serial_early_debug_enabled(Some(profile.early_serial_debug));
                Self::set_telemetry_history_len(Some(profile.history_len));
                Self::set_log_level_num(Some(profile.log_level_num));
            },
            || {
                Self::set_telemetry_enabled(None);
                Self::set_telemetry_runtime_summary_enabled(None);
                Self::set_telemetry_virtualization_enabled(None);
                Self::set_telemetry_platform_lifecycle_enabled(None);
                Self::set_telemetry_vfs_enabled(None);
                Self::set_telemetry_network_enabled(None);
                Self::set_telemetry_ipc_enabled(None);
                Self::set_telemetry_scheduler_enabled(None);
                Self::set_telemetry_security_enabled(None);
                Self::set_telemetry_power_enabled(None);
                Self::set_telemetry_drivers_enabled(None);
                Self::set_debug_trace_enabled(None);
                Self::set_serial_early_debug_enabled(None);
                Self::set_telemetry_history_len(None);
                Self::set_log_level_num(None);
            },
        );
    }

    pub fn set_library_runtime_feature_profile(value: Option<LibraryRuntimeFeatureProfile>) {
        apply_profile_override(
            value,
            |profile| {
                Self::set_library_boundary_mode(Some(profile.boundary_mode));
                Self::set_core_minimal_enforced(Some(profile.enforce_core_minimal));
                Self::set_strict_optional_features_enabled(Some(profile.strict_optional_features));
                Self::set_vfs_library_api_exposed(Some(profile.expose_vfs_api));
                Self::set_network_library_api_exposed(Some(profile.expose_network_api));
                Self::set_ipc_library_api_exposed(Some(profile.expose_ipc_api));
                Self::set_proc_config_api_exposed(Some(profile.expose_proc_config_api));
                Self::set_sysctl_api_exposed(Some(profile.expose_sysctl_api));
                Self::set_libnet_fast_path_run_pump(Some(profile.libnet_fast_path_run_pump));
                Self::set_libnet_fast_path_collect_transport_snapshot(Some(
                    profile.libnet_fast_path_collect_transport_snapshot,
                ));
            },
            || {
                Self::set_library_boundary_mode(None);
                Self::set_core_minimal_enforced(None);
                Self::set_strict_optional_features_enabled(None);
                Self::set_vfs_library_api_exposed(None);
                Self::set_network_library_api_exposed(None);
                Self::set_ipc_library_api_exposed(None);
                Self::set_proc_config_api_exposed(None);
                Self::set_sysctl_api_exposed(None);
                Self::set_libnet_fast_path_run_pump(None);
                Self::set_libnet_fast_path_collect_transport_snapshot(None);
            },
        );
    }

    pub fn set_credential_runtime_profile(value: Option<CredentialRuntimeProfile>) {
        apply_profile_override(
            value,
            |profile| {
                Self::set_security_enforcement_enabled(Some(profile.security_enforcement));
                Self::set_capability_enforcement_enabled(Some(profile.capability_enforcement));
                Self::set_multi_user_enabled(Some(profile.multi_user));
                Self::set_credential_enforcement_enabled(Some(profile.credential_enforcement));
            },
            || {
                Self::set_security_enforcement_enabled(None);
                Self::set_capability_enforcement_enabled(None);
                Self::set_multi_user_enabled(None);
                Self::set_credential_enforcement_enabled(None);
            },
        );
    }
}
