use super::*;

#[test_case]
fn orchestrator_session_prefers_driverkit_when_sidecar_and_liblinux_are_under_pressure() {
    let request = HybridRequest::network(0x1000, 0x200, 0x3000, 0x3000, 45);
    let cfg = SideCarVmConfig::new(3, 2, 128 * 1024 * 1024);

    let mut sidecar_telemetry = SideCarTelemetryStore::new(4);
    sidecar_telemetry.record(
        LinuxShimDeviceKind::Network,
        SideCarTelemetrySample::new(96, 5, 4096, 130),
    );

    let mut liblinux_telemetry = HybridOrchestratorSession::new(8);
    liblinux_telemetry.record_liblinux_dispatch_sample_for_syscall(
        LinuxSyscall::Write,
        LibLinuxDispatchSample::new(8, 8, 0, 0, 8),
    );

    let diagnostics = HybridOrchestrator::plan_with_diagnostics_and_dual_telemetry(
        &request,
        BackendPreference::SideCarFirst,
        cfg,
        Some(&sidecar_telemetry),
        Some(liblinux_telemetry.liblinux_telemetry()),
    );

    assert_eq!(
        diagnostics.attempts.first().map(|attempt| attempt.backend),
        Some(BackendPreference::DriverKitFirst)
    );
    assert!(matches!(diagnostics.selected, Some(HybridExecutionPlan::DriverKit(_))));
}

#[test_case]
fn full_context_diagnostics_demote_unhealthy_driverkit_under_pressure() {
    let request = HybridRequest::network(0x1100, 0x200, 0x3100, 0x3000, 46);
    let cfg = SideCarVmConfig::new(4, 2, 128 * 1024 * 1024);

    let mut sidecar_telemetry = SideCarTelemetryStore::new(4);
    sidecar_telemetry.record(
        LinuxShimDeviceKind::Network,
        SideCarTelemetrySample::new(96, 5, 4096, 130),
    );

    let mut liblinux_session = HybridOrchestratorSession::new(8);
    liblinux_session.record_liblinux_dispatch_sample_for_syscall(
        LinuxSyscall::Write,
        LibLinuxDispatchSample::new(8, 8, 0, 0, 8),
    );

    let health = DriverKitHealthSnapshot {
        class_count: 1,
        binding_count: 1,
        started_count: 0,
        faulted_count: 2,
        quarantined_count: 1,
        dispatch_success_count: 0,
        dispatch_failure_count: 9,
    };

    let diagnostics = HybridOrchestrator::plan_with_diagnostics_with_full_context(
        &request,
        BackendPreference::SideCarFirst,
        cfg,
        Some(&sidecar_telemetry),
        Some(liblinux_session.liblinux_telemetry()),
        Some(health),
    );

    assert_ne!(
        diagnostics.attempts.first().map(|attempt| attempt.backend),
        Some(BackendPreference::DriverKitFirst)
    );
}
