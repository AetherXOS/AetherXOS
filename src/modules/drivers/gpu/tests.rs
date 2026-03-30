use super::*;

#[test_case]
fn gpu_stack_state_transitions_to_initialized() {
    init_gpu_stack();
    let snapshot = gpu_stack_snapshot();
    assert_eq!(snapshot.state, GpuStackState::Initialized);
}

#[test_case]
fn gpu_heartbeat_is_recorded() {
    note_gpu_heartbeat(1234);
    assert_eq!(gpu_stack_snapshot().heartbeat_ticks, 1234);
}

#[test_case]
fn gpu_desktop_path_readiness_requires_kms_and_input() {
    init_gpu_stack();
    mark_kms_ready();
    assert!(!is_desktop_session_ready());

    mark_input_ready();
    assert!(is_desktop_session_ready());
}

#[test_case]
fn gpu_desktop_path_readiness_requires_non_none_backend() {
    configure_gpu_backend_for_desktop(GpuBackend::None, true, true);
    assert!(!gpu_backend_supports_acceleration());
    assert!(!is_desktop_session_ready());

    configure_gpu_backend_for_desktop(GpuBackend::VirtIoGpu, true, true);
    assert!(gpu_backend_supports_acceleration());
    assert!(is_desktop_session_ready());
}

#[test_case]
fn gpu_health_is_critical_without_initialization() {
    configure_gpu_backend_for_desktop(GpuBackend::None, false, false);
    let report = evaluate_gpu_health(100);
    assert_eq!(report.tier, GpuHealthTier::Critical);
    assert_eq!(
        recommended_gpu_health_action(report),
        GpuHealthAction::PreferFramebufferFallback
    );
}

#[test_case]
fn gpu_health_detects_stale_heartbeat() {
    configure_gpu_backend_for_desktop(GpuBackend::VirtIoGpu, true, true);
    note_gpu_heartbeat(1);
    let stale_now = gpu_health_thresholds().max_heartbeat_staleness_ticks + 2;
    let report = evaluate_gpu_health(stale_now);
    assert_eq!(report.tier, GpuHealthTier::Degraded);
    assert_eq!(
        recommended_gpu_health_action(report),
        GpuHealthAction::ReinitializeStack
    );
}
