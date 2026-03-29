use core::sync::atomic::Ordering;

use super::*;
impl KernelConfig {
    pub fn boundary_mode() -> BoundaryMode {
        match LIBRARY_BOUNDARY_MODE_OVERRIDE.load(Ordering::Relaxed) {
            BOUNDARY_MODE_OVERRIDE_STRICT => BoundaryMode::Strict,
            BOUNDARY_MODE_OVERRIDE_BALANCED => BoundaryMode::Balanced,
            BOUNDARY_MODE_OVERRIDE_COMPAT => BoundaryMode::Compat,
            _ => BoundaryMode::from_str(LIBRARY_BOUNDARY_MODE),
        }
    }

    pub fn is_core_minimal_enforced() -> bool {
        decode_bool_override(
            LIBRARY_ENFORCE_CORE_MINIMAL_OVERRIDE.load(Ordering::Relaxed),
            LIBRARY_ENFORCE_CORE_MINIMAL,
        )
    }

    pub fn is_strict_optional_features_enabled() -> bool {
        decode_bool_override(
            LIBRARY_STRICT_OPTIONAL_FEATURES_OVERRIDE.load(Ordering::Relaxed),
            LIBRARY_STRICT_OPTIONAL_FEATURES,
        )
    }

    pub fn library_max_services() -> usize {
        LIBRARY_MAX_SERVICES
    }

    pub fn should_log_library_inventory() -> bool {
        LIBRARY_VERBOSE_BOOT_INVENTORY
    }

    pub fn is_vfs_library_api_exposed() -> bool {
        decode_bool_override(
            LIBRARY_EXPOSE_VFS_API_OVERRIDE.load(Ordering::Relaxed),
            LIBRARY_EXPOSE_VFS_API,
        )
    }

    pub fn is_network_library_api_exposed() -> bool {
        decode_bool_override(
            LIBRARY_EXPOSE_NETWORK_API_OVERRIDE.load(Ordering::Relaxed),
            LIBRARY_EXPOSE_NETWORK_API,
        )
    }

    pub fn is_ipc_library_api_exposed() -> bool {
        decode_bool_override(
            LIBRARY_EXPOSE_IPC_API_OVERRIDE.load(Ordering::Relaxed),
            LIBRARY_EXPOSE_IPC_API,
        )
    }

    pub fn is_proc_config_api_exposed() -> bool {
        let default_enabled = Self::is_vfs_library_api_exposed()
            && !matches!(Self::boundary_mode(), BoundaryMode::Strict);
        decode_bool_override(
            LIBRARY_EXPOSE_PROC_CONFIG_API_OVERRIDE.load(Ordering::Relaxed),
            default_enabled,
        ) && Self::is_vfs_library_api_exposed()
    }

    pub fn is_sysctl_api_exposed() -> bool {
        let default_enabled = Self::is_vfs_library_api_exposed()
            && !matches!(Self::boundary_mode(), BoundaryMode::Strict);
        decode_bool_override(
            LIBRARY_EXPOSE_SYSCTL_API_OVERRIDE.load(Ordering::Relaxed),
            default_enabled,
        ) && Self::is_vfs_library_api_exposed()
    }

    pub fn security_enforcement_enabled() -> bool {
        let default_enabled = cfg!(any(
            feature = "security",
            feature = "security_acl",
            feature = "security_capabilities",
            feature = "security_sel4",
            feature = "security_null"
        ));
        decode_bool_override(
            SECURITY_ENFORCEMENT_ENABLED_OVERRIDE.load(Ordering::Relaxed),
            default_enabled,
        )
    }

    pub fn capability_enforcement_enabled() -> bool {
        let default_enabled = cfg!(any(
            feature = "capabilities",
            feature = "security_capabilities"
        ));
        decode_bool_override(
            CAPABILITY_ENFORCEMENT_ENABLED_OVERRIDE.load(Ordering::Relaxed),
            default_enabled,
        ) && Self::security_enforcement_enabled()
    }

    pub fn multi_user_enabled() -> bool {
        decode_bool_override(MULTI_USER_ENABLED_OVERRIDE.load(Ordering::Relaxed), true)
    }

    pub fn credential_enforcement_enabled() -> bool {
        decode_bool_override(
            CREDENTIAL_ENFORCEMENT_ENABLED_OVERRIDE.load(Ordering::Relaxed),
            true,
        ) && Self::multi_user_enabled()
    }

    pub fn libnet_l2_enabled() -> bool {
        Self::is_network_library_api_exposed() && LIBNET_L2_ENABLED
    }

    pub fn libnet_l34_enabled() -> bool {
        Self::is_network_library_api_exposed() && LIBNET_L34_ENABLED
    }

    pub fn libnet_l6_enabled() -> bool {
        Self::is_network_library_api_exposed() && LIBNET_L6_ENABLED
    }

    pub fn libnet_l7_enabled() -> bool {
        Self::is_network_library_api_exposed() && LIBNET_L7_ENABLED
    }

    pub fn libnet_fast_path_default_strategy() -> &'static str {
        LIBNET_FAST_PATH_DEFAULT_STRATEGY
    }

    pub fn libnet_fast_path_strategy() -> LibNetFastPathStrategy {
        LibNetFastPathStrategy::from_str(LIBNET_FAST_PATH_DEFAULT_STRATEGY)
    }

    pub fn libnet_fast_path_run_pump() -> bool {
        Self::is_network_library_api_exposed()
            && decode_bool_override(
                LIBNET_FAST_PATH_RUN_PUMP_OVERRIDE.load(Ordering::Relaxed),
                LIBNET_FAST_PATH_RUN_PUMP,
            )
    }

    pub fn linux_release() -> &'static str {
        DEFAULT_LINUX_RELEASE
    }

    pub fn linux_version() -> &'static str {
        DEFAULT_LINUX_VERSION
    }

    pub fn libnet_fast_path_collect_transport_snapshot() -> bool {
        Self::is_network_library_api_exposed()
            && decode_bool_override(
                LIBNET_FAST_PATH_COLLECT_TRANSPORT_SNAPSHOT_OVERRIDE.load(Ordering::Relaxed),
                LIBNET_FAST_PATH_COLLECT_TRANSPORT_SNAPSHOT,
            )
    }

    pub fn libnet_fast_path_pump_budget() -> usize {
        if !Self::is_network_library_api_exposed() {
            return 0;
        }
        LIBNET_FAST_PATH_PUMP_BUDGET.max(MIN_LIBNET_FAST_PATH_PUMP_BUDGET)
    }

    pub fn telemetry_runtime_summary_enabled() -> bool {
        Self::is_telemetry_enabled()
            && decode_bool_override(
                TELEMETRY_RUNTIME_SUMMARY_OVERRIDE.load(Ordering::Relaxed),
                TELEMETRY_RUNTIME_SUMMARY,
            )
    }

    pub fn telemetry_vfs_enabled() -> bool {
        Self::is_telemetry_enabled()
            && decode_bool_override(
                TELEMETRY_VFS_OVERRIDE.load(Ordering::Relaxed),
                TELEMETRY_ENABLE_VFS,
            )
    }

    pub fn telemetry_virtualization_enabled() -> bool {
        Self::telemetry_runtime_summary_enabled()
            && decode_bool_override(
                TELEMETRY_VIRTUALIZATION_OVERRIDE.load(Ordering::Relaxed),
                true,
            )
    }

    pub fn telemetry_platform_lifecycle_enabled() -> bool {
        Self::telemetry_virtualization_enabled()
            && decode_bool_override(
                TELEMETRY_PLATFORM_LIFECYCLE_OVERRIDE.load(Ordering::Relaxed),
                true,
            )
    }

    pub fn telemetry_network_enabled() -> bool {
        Self::is_telemetry_enabled()
            && decode_bool_override(
                TELEMETRY_NETWORK_OVERRIDE.load(Ordering::Relaxed),
                TELEMETRY_ENABLE_NETWORK,
            )
    }

    pub fn telemetry_ipc_enabled() -> bool {
        Self::is_telemetry_enabled()
            && decode_bool_override(
                TELEMETRY_IPC_OVERRIDE.load(Ordering::Relaxed),
                TELEMETRY_ENABLE_IPC,
            )
    }

    pub fn telemetry_scheduler_enabled() -> bool {
        Self::is_telemetry_enabled()
            && decode_bool_override(
                TELEMETRY_SCHEDULER_OVERRIDE.load(Ordering::Relaxed),
                TELEMETRY_ENABLE_SCHEDULER,
            )
    }

    pub fn telemetry_security_enabled() -> bool {
        Self::is_telemetry_enabled()
            && decode_bool_override(
                TELEMETRY_SECURITY_OVERRIDE.load(Ordering::Relaxed),
                TELEMETRY_ENABLE_SECURITY,
            )
    }

    pub fn telemetry_power_enabled() -> bool {
        Self::is_telemetry_enabled()
            && decode_bool_override(
                TELEMETRY_POWER_OVERRIDE.load(Ordering::Relaxed),
                TELEMETRY_ENABLE_POWER,
            )
    }

    pub fn telemetry_drivers_enabled() -> bool {
        Self::is_telemetry_enabled()
            && decode_bool_override(
                TELEMETRY_DRIVERS_OVERRIDE.load(Ordering::Relaxed),
                TELEMETRY_ENABLE_DRIVERS,
            )
    }

    pub fn debug_trace_enabled() -> bool {
        let compile_default = Self::is_advanced_debug_enabled();
        decode_bool_override(
            DEBUG_TRACE_ENABLED_OVERRIDE.load(Ordering::Relaxed),
            compile_default,
        )
    }

    pub fn serial_early_debug_enabled() -> bool {
        decode_bool_override(
            SERIAL_EARLY_DEBUG_ENABLED_OVERRIDE.load(Ordering::Relaxed),
            Self::debug_trace_enabled(),
        )
    }

    pub fn should_emit_early_serial_line(line: &str) -> bool {
        if !line.starts_with("[EARLY SERIAL]") {
            return true;
        }
        Self::serial_early_debug_enabled()
    }

    // Granular category-level observability gates
    // These combine compile-time feature flags (via is_category_enabled_compile_time)
    // with runtime overrides for flexible control

    pub fn is_observability_category_enabled(category: crate::config::ObservabilityCategory) -> bool {
        // First check if disabled at runtime override (value 1 = false)
        let override_val = match category {
            crate::config::ObservabilityCategory::Core => OBSERVABILITY_CORE_OVERRIDE.load(Ordering::Relaxed),
            crate::config::ObservabilityCategory::Boot => OBSERVABILITY_BOOT_OVERRIDE.load(Ordering::Relaxed),
            crate::config::ObservabilityCategory::Loader => OBSERVABILITY_LOADER_OVERRIDE.load(Ordering::Relaxed),
            crate::config::ObservabilityCategory::Task => OBSERVABILITY_TASK_OVERRIDE.load(Ordering::Relaxed),
            crate::config::ObservabilityCategory::Memory => OBSERVABILITY_MEMORY_OVERRIDE.load(Ordering::Relaxed),
            crate::config::ObservabilityCategory::Scheduler => OBSERVABILITY_SCHEDULER_OVERRIDE.load(Ordering::Relaxed),
            crate::config::ObservabilityCategory::Fault => OBSERVABILITY_FAULT_OVERRIDE.load(Ordering::Relaxed),
            crate::config::ObservabilityCategory::Driver => OBSERVABILITY_DRIVER_OVERRIDE.load(Ordering::Relaxed),
            crate::config::ObservabilityCategory::Io => OBSERVABILITY_IO_OVERRIDE.load(Ordering::Relaxed),
            crate::config::ObservabilityCategory::Network => OBSERVABILITY_NETWORK_OVERRIDE.load(Ordering::Relaxed),
        };

        // Compile-time default (from feature flags)
        let compile_default = crate::config::is_category_enabled_compile_time(category);

        // Final decision: runtime override > compile-time default > global debug_trace_enabled
        decode_bool_override(override_val, compile_default) && Self::debug_trace_enabled()
    }
}
