use super::*;

#[test_case]
fn session_service_registration_and_readiness_flow() {
    let name = "org.aethercore.logind.test";
    let _ = register_session_service(name, true);
    assert!(mark_session_service_ready(name).is_ok());
    assert!(heartbeat_session_service(name, 42).is_ok());

    let services = list_session_services();
    let svc = services
        .iter()
        .find(|s| s.name == name)
        .expect("service snapshot");
    assert_eq!(svc.state, SessionServiceState::Ready);
    assert_eq!(svc.last_heartbeat_tick, 42);
}

#[test_case]
fn degraded_service_with_autorestart_returns_to_starting() {
    let name = "org.aethercore.udevd.test";
    let _ = register_session_service(name, true);
    assert!(mark_session_service_degraded(name).is_ok());

    let services = list_session_services();
    let svc = services
        .iter()
        .find(|s| s.name == name)
        .expect("service snapshot");
    assert_eq!(svc.state, SessionServiceState::Starting);
    assert!(svc.restart_count >= 1);
}
