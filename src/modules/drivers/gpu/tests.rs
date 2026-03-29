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
