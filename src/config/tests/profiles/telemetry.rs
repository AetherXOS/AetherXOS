use super::*;

#[test_case]
fn telemetry_runtime_profile_roundtrip_and_reset() {
    KernelConfig::reset_runtime_overrides();

    let profile = super::TelemetryRuntimeProfile {
        enabled: true,
        runtime_summary: false,
        virtualization: true,
        platform_lifecycle: false,
        vfs: true,
        network: false,
        ipc: true,
        scheduler: false,
        security: true,
        power: false,
        drivers: true,
        debug_trace: true,
        early_serial_debug: true,
        history_len: 2048,
        log_level_num: 4,
    };
    KernelConfig::set_telemetry_runtime_profile(Some(profile));

    let got = KernelConfig::telemetry_runtime_profile();
    assert_eq!(got, profile);

    KernelConfig::set_telemetry_runtime_profile(None);
    let reset = KernelConfig::telemetry_runtime_profile();
    assert_eq!(reset.enabled, crate::generated_consts::TELEMETRY_ENABLED);
    assert_eq!(
        reset.runtime_summary,
        crate::generated_consts::TELEMETRY_RUNTIME_SUMMARY
    );
    assert_eq!(
        reset.virtualization,
        crate::generated_consts::TELEMETRY_RUNTIME_SUMMARY
    );
    assert_eq!(
        reset.platform_lifecycle,
        crate::generated_consts::TELEMETRY_RUNTIME_SUMMARY
    );
    assert_eq!(reset.vfs, crate::generated_consts::TELEMETRY_ENABLE_VFS);
    assert_eq!(
        reset.network,
        crate::generated_consts::TELEMETRY_ENABLE_NETWORK
    );
    assert_eq!(reset.ipc, crate::generated_consts::TELEMETRY_ENABLE_IPC);
    assert_eq!(
        reset.scheduler,
        crate::generated_consts::TELEMETRY_ENABLE_SCHEDULER
    );
    assert_eq!(
        reset.security,
        crate::generated_consts::TELEMETRY_ENABLE_SECURITY
    );
    assert_eq!(reset.power, crate::generated_consts::TELEMETRY_ENABLE_POWER);
    assert_eq!(
        reset.drivers,
        crate::generated_consts::TELEMETRY_ENABLE_DRIVERS
    );
    assert_eq!(
        reset.history_len,
        crate::generated_consts::TELEMETRY_HISTORY_LEN
    );
    assert_eq!(reset.log_level_num, crate::generated_consts::LOG_LEVEL_NUM);
    assert_eq!(
        reset.debug_trace,
        KernelConfig::is_advanced_debug_enabled()
    );
    assert_eq!(
        reset.early_serial_debug,
        KernelConfig::is_advanced_debug_enabled()
    );
}
