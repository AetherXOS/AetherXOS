use super::*;
use crate::modules::drivers::{SideCarBootstrapState, SideCarBootstrapPhase};

#[test_case]
fn builds_sidecar_bootstrap_frames() {
    let request = HybridRequest::network(0x1000, 0x100, 0x2000, 0x2000, 40);
    let cfg = SideCarVmConfig::new(7, 2, 256 * 1024 * 1024);
    let frames = HybridOrchestrator::build_sidecar_bootstrap_frames(&request, cfg, 900)
        .expect("frames should be produced");
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].header.request_id, 900);
    assert_eq!(frames[1].header.request_id, 901);
}

#[test_case]
fn submits_sidecar_bootstrap_frames_into_transport() {
    let request = HybridRequest::network(0x1000, 0x100, 0x2000, 0x2000, 55);
    let cfg = SideCarVmConfig::new(9, 2, 128 * 1024 * 1024);
    let frames = HybridOrchestrator::build_sidecar_bootstrap_frames(&request, cfg, 77)
        .expect("frames should be produced");

    let mut transport = InMemorySideCarTransport::new();
    HybridOrchestrator::submit_sidecar_frames(&mut transport, &frames)
        .expect("submit should work");

    let first = transport.pop_wire_frame().expect("first frame exists");
    let second = transport.pop_wire_frame().expect("second frame exists");
    assert_eq!(first.0.request_id, 77);
    assert_eq!(second.0.request_id, 78);
}

#[test_case]
fn drives_sidecar_bootstrap_state_machine() {
    let mut transport = InMemorySideCarTransport::new();
    let mut state = SideCarBootstrapState::new(500, SideCarRetryPolicy::conservative());

    let sent = HybridOrchestrator::drive_sidecar_bootstrap(&mut transport, &mut state, 4, 128, 0)
        .expect("drive should succeed");
    assert!(sent);
    state.mark_success();

    let sent = HybridOrchestrator::drive_sidecar_bootstrap(&mut transport, &mut state, 4, 128, 1)
        .expect("drive should succeed");
    assert!(sent);
    state.mark_success();

    assert_eq!(state.phase, SideCarBootstrapPhase::Completed);
}

#[test_case]
fn advances_sidecar_bootstrap_from_bridge_completion() {
    let mut state = SideCarBootstrapState::new(71, SideCarRetryPolicy::conservative());
    let message = LinuxBridgeMessage::new(
        LinuxBridgeMessageKind::QueryStatus,
        71,
        LinuxBridgePayload::Completion(crate::modules::drivers::hybrid::DriverCompletion::ok(71, 0)),
    );

    let advanced = HybridOrchestrator::advance_sidecar_bootstrap_from_bridge_message(
        &mut state,
        &message,
        0,
    );
    assert!(advanced);
    assert_eq!(state.phase, SideCarBootstrapPhase::ControlNotify);
}
