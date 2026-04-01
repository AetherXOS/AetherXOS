use super::*;

#[test_case]
fn runtime_policy_drift_runtime_profile_roundtrip_and_reset() {
    KernelConfig::reset_runtime_overrides();

    let profile = super::RuntimePolicyDriftRuntimeProfile {
        sample_interval_ticks: 5,
        reapply_cooldown_ticks: 13,
    };
    KernelConfig::set_runtime_policy_drift_runtime_profile(Some(profile));

    let got = KernelConfig::runtime_policy_drift_runtime_profile();
    assert_eq!(got, profile);

    KernelConfig::set_runtime_policy_drift_runtime_profile(None);
    let reset = KernelConfig::runtime_policy_drift_runtime_profile();
    assert_eq!(
        reset.sample_interval_ticks,
        crate::generated_consts::GOVERNOR_RUNTIME_POLICY_DRIFT_SAMPLE_INTERVAL_TICKS
    );
    assert_eq!(
        reset.reapply_cooldown_ticks,
        crate::generated_consts::GOVERNOR_RUNTIME_POLICY_DRIFT_REAPPLY_COOLDOWN_TICKS
    );
}

#[test_case]
fn library_runtime_feature_profile_roundtrip_and_reset() {
    KernelConfig::reset_runtime_overrides();

    let profile = super::LibraryRuntimeFeatureProfile {
        boundary_mode: super::BoundaryMode::Compat,
        enforce_core_minimal: false,
        strict_optional_features: false,
        expose_vfs_api: false,
        expose_network_api: false,
        expose_ipc_api: true,
        expose_proc_config_api: true,
        expose_sysctl_api: false,
        libnet_fast_path_run_pump: true,
        libnet_fast_path_collect_transport_snapshot: true,
    };
    KernelConfig::set_library_runtime_feature_profile(Some(profile));

    let got = KernelConfig::library_runtime_feature_profile();
    assert_eq!(got.boundary_mode, super::BoundaryMode::Compat);
    assert!(!got.enforce_core_minimal);
    assert!(!got.strict_optional_features);
    assert!(!got.expose_vfs_api);
    assert!(!got.expose_network_api);
    assert!(got.expose_ipc_api);
    assert!(!KernelConfig::libnet_l2_enabled());
    assert!(!KernelConfig::libnet_l34_enabled());
    assert!(!KernelConfig::libnet_l6_enabled());
    assert!(!KernelConfig::libnet_l7_enabled());
    assert!(!KernelConfig::libnet_fast_path_run_pump());
    assert!(!KernelConfig::libnet_fast_path_collect_transport_snapshot());
    assert_eq!(KernelConfig::libnet_fast_path_pump_budget(), 0);

    KernelConfig::set_library_runtime_feature_profile(None);
    let reset = KernelConfig::library_runtime_feature_profile();
    assert_eq!(
        reset.boundary_mode,
        super::BoundaryMode::from_str(crate::generated_consts::LIBRARY_BOUNDARY_MODE)
    );
    assert_eq!(
        reset.enforce_core_minimal,
        crate::generated_consts::LIBRARY_ENFORCE_CORE_MINIMAL
    );
    assert_eq!(
        reset.strict_optional_features,
        crate::generated_consts::LIBRARY_STRICT_OPTIONAL_FEATURES
    );
    assert_eq!(
        reset.expose_vfs_api,
        crate::generated_consts::LIBRARY_EXPOSE_VFS_API
    );
    assert_eq!(
        reset.expose_network_api,
        crate::generated_consts::LIBRARY_EXPOSE_NETWORK_API
    );
    assert_eq!(
        reset.expose_ipc_api,
        crate::generated_consts::LIBRARY_EXPOSE_IPC_API
    );
}

#[test_case]
fn cargo_defaults_match_runtime_profiles_after_reset() {
    KernelConfig::reset_runtime_overrides();

    assert_eq!(
        KernelConfig::network_runtime_profile(),
        KernelConfig::network_cargo_profile()
    );
    assert_eq!(
        KernelConfig::scheduler_runtime_profile(),
        KernelConfig::scheduler_cargo_profile()
    );
    assert_eq!(
        KernelConfig::driver_network_runtime_profile(),
        KernelConfig::driver_network_cargo_profile()
    );
    assert_eq!(
        KernelConfig::telemetry_runtime_profile(),
        KernelConfig::telemetry_cargo_profile()
    );
    assert_eq!(
        KernelConfig::vfs_runtime_profile(),
        KernelConfig::vfs_cargo_profile()
    );
    assert_eq!(
        KernelConfig::devfs_runtime_profile(),
        KernelConfig::devfs_cargo_profile()
    );
    assert_eq!(
        KernelConfig::runtime_policy_drift_runtime_profile(),
        KernelConfig::runtime_policy_drift_cargo_profile()
    );
    assert_eq!(
        KernelConfig::library_runtime_feature_profile(),
        KernelConfig::library_cargo_feature_profile()
    );
}
