use super::*;

#[test_case]
fn virtualization_runtime_profile_roundtrip_and_reset() {
    KernelConfig::reset_runtime_overrides();

    let profile = super::VirtualizationRuntimeProfile {
        telemetry: true,
        platform_lifecycle: false,
        entry: true,
        resume: false,
        trap_dispatch: true,
        nested: false,
        time_virtualization: true,
        device_passthrough: false,
        snapshot: false,
        dirty_logging: true,
        live_migration: false,
        trap_tracing: true,
    };
    KernelConfig::set_virtualization_runtime_profile(Some(profile));

    let got = KernelConfig::virtualization_runtime_profile();
    assert_eq!(got, profile);

    KernelConfig::set_virtualization_runtime_profile(None);
    let reset = KernelConfig::virtualization_runtime_profile();
    assert_eq!(
        reset.telemetry,
        crate::generated_consts::TELEMETRY_RUNTIME_SUMMARY
    );
    assert_eq!(
        reset.platform_lifecycle,
        crate::generated_consts::TELEMETRY_RUNTIME_SUMMARY
    );
    assert!(reset.entry);
    assert!(reset.resume);
    assert!(reset.trap_dispatch);
    assert!(reset.nested);
    assert!(reset.time_virtualization);
    assert!(reset.device_passthrough);
    assert!(reset.snapshot);
    assert!(reset.dirty_logging);
    assert!(reset.live_migration);
    assert!(reset.trap_tracing);
}

#[test_case]
fn virtualization_profile_merge_applies_compile_time_caps() {
    let runtime = super::VirtualizationRuntimeProfile {
        telemetry: true,
        platform_lifecycle: true,
        entry: true,
        resume: false,
        trap_dispatch: true,
        nested: true,
        time_virtualization: false,
        device_passthrough: true,
        snapshot: true,
        dirty_logging: true,
        live_migration: true,
        trap_tracing: true,
    };
    let cargo = super::VirtualizationRuntimeProfile {
        telemetry: true,
        platform_lifecycle: false,
        entry: true,
        resume: true,
        trap_dispatch: false,
        nested: false,
        time_virtualization: true,
        device_passthrough: false,
        snapshot: false,
        dirty_logging: true,
        live_migration: false,
        trap_tracing: true,
    };

    let effective = KernelConfig::merge_virtualization_profiles(runtime, cargo);
    assert_eq!(
        effective,
        super::VirtualizationRuntimeProfile {
            telemetry: true,
            platform_lifecycle: false,
            entry: true,
            resume: false,
            trap_dispatch: false,
            nested: false,
            time_virtualization: false,
            device_passthrough: false,
            snapshot: false,
            dirty_logging: true,
            live_migration: false,
            trap_tracing: true,
        }
    );
}

#[test_case]
fn virtualization_policy_profile_exposes_runtime_cargo_and_effective_views() {
    KernelConfig::reset_runtime_overrides();
    KernelConfig::set_virtualization_runtime_profile(Some(super::VirtualizationRuntimeProfile {
        telemetry: true,
        platform_lifecycle: false,
        entry: false,
        resume: true,
        trap_dispatch: true,
        nested: false,
        time_virtualization: true,
        device_passthrough: false,
        snapshot: false,
        dirty_logging: true,
        live_migration: false,
        trap_tracing: true,
    }));

    let profile = KernelConfig::virtualization_policy_profile();
    assert!(!profile.runtime.snapshot);
    assert!(!profile.runtime.entry);
    assert!(!profile.runtime.nested);
    assert!(profile.cargo.entry);
    assert!(!profile.effective.entry);
    assert!(!profile.effective.nested);
    assert!(profile.cargo.snapshot);
    assert!(!profile.effective.snapshot);
    assert!(!profile.runtime.live_migration);
    assert!(!profile.effective.live_migration);

    KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn virtualization_policy_scope_profile_reports_per_feature_limits() {
    KernelConfig::reset_runtime_overrides();
    KernelConfig::set_virtualization_entry_enabled(Some(false));
    KernelConfig::set_virtualization_nested_enabled(Some(false));
    KernelConfig::set_virtualization_time_virtualization_enabled(Some(false));

    let scope = KernelConfig::virtualization_policy_scope_profile();
    assert_eq!(scope.overall, "runtime-limited");
    assert_eq!(scope.entry, "runtime-limited");
    assert_eq!(scope.resume, "fully-enabled");
    assert_eq!(scope.trap_dispatch, "fully-enabled");
    assert_eq!(scope.nested, "runtime-limited");
    assert_eq!(scope.time_virtualization, "runtime-limited");
    assert_eq!(scope.device_passthrough, "fully-enabled");
    assert_eq!(scope.snapshot, "fully-enabled");
    assert_eq!(scope.live_migration, "fully-enabled");

    KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn virtualization_execution_profile_roundtrip_and_effective_view() {
    KernelConfig::reset_runtime_overrides();
    KernelConfig::set_virtualization_execution_policy_profile(Some(
        super::VirtualizationExecutionProfile {
            scheduling_class: crate::config::VirtualizationExecutionClass::LatencyCritical,
        },
    ));

    let runtime = KernelConfig::virtualization_execution_profile();
    let policy = KernelConfig::virtualization_execution_policy_profile();
    assert_eq!(
        runtime.scheduling_class,
        crate::config::VirtualizationExecutionClass::LatencyCritical
    );
    assert_eq!(
        policy.runtime.scheduling_class,
        crate::config::VirtualizationExecutionClass::LatencyCritical
    );
    assert_eq!(
        policy.cargo.scheduling_class,
        crate::config::VirtualizationExecutionClass::Balanced
    );
    assert_eq!(
        policy.effective.scheduling_class,
        crate::config::VirtualizationExecutionClass::LatencyCritical
    );
    assert_eq!(
        KernelConfig::virtualization_execution_policy_scope(),
        "runtime-limited"
    );

    KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn virtualization_execution_profile_accepts_runtime_key_string_updates() {
    KernelConfig::reset_runtime_overrides();

    KernelConfig::set_by_key_str("virtualization_execution_profile", Some("LatencyCritical"))
        .expect("virtualization_execution_profile runtime key should parse");

    let runtime = KernelConfig::virtualization_execution_profile();
    assert_eq!(
        runtime.scheduling_class,
        crate::config::VirtualizationExecutionClass::LatencyCritical
    );

    KernelConfig::set_by_key_str("virtualization_execution_profile", None)
        .expect("virtualization_execution_profile runtime key reset should work");

    let reset = KernelConfig::virtualization_execution_profile();
    assert_eq!(
        reset.scheduling_class,
        crate::config::VirtualizationExecutionClass::Balanced
    );
}

#[test_case]
fn virtualization_governor_profile_roundtrip_and_effective_view() {
    KernelConfig::reset_runtime_overrides();
    KernelConfig::set_virtualization_governor_policy_profile(Some(
        super::VirtualizationGovernorProfile {
            governor_class: crate::config::VirtualizationGovernorClass::Performance,
        },
    ));

    let runtime = KernelConfig::virtualization_governor_profile();
    let policy = KernelConfig::virtualization_governor_policy_profile();
    assert_eq!(
        runtime.governor_class,
        crate::config::VirtualizationGovernorClass::Performance
    );
    assert_eq!(
        policy.runtime.governor_class,
        crate::config::VirtualizationGovernorClass::Performance
    );
    assert_eq!(
        policy.cargo.governor_class,
        crate::config::VirtualizationGovernorClass::Balanced
    );
    assert_eq!(
        policy.effective.governor_class,
        crate::config::VirtualizationGovernorClass::Performance
    );
    assert_eq!(
        KernelConfig::virtualization_governor_policy_scope(),
        "runtime-limited"
    );

    KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn virtualization_governor_profile_accepts_runtime_key_string_updates() {
    KernelConfig::reset_runtime_overrides();

    KernelConfig::set_by_key_str("virtualization_governor_profile", Some("Performance"))
        .expect("virtualization_governor_profile runtime key should parse");

    let runtime = KernelConfig::virtualization_governor_profile();
    assert_eq!(
        runtime.governor_class,
        crate::config::VirtualizationGovernorClass::Performance
    );

    KernelConfig::set_by_key_str("virtualization_governor_profile", None)
        .expect("virtualization_governor_profile runtime key reset should work");

    let reset = KernelConfig::virtualization_governor_profile();
    assert_eq!(
        reset.governor_class,
        crate::config::VirtualizationGovernorClass::Balanced
    );
}
