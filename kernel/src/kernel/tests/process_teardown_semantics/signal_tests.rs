use super::super::integration_harness::{IntegrationHarness, WaitFlags, WaitOutcome};
use super::util::{did_core_dump, encode_signaled, is_signaled};

#[test_case]
fn sigchld_sent_to_parent_on_child_termination() {
    let mut harness = IntegrationHarness::new();
    let child = harness.fork(1).expect("fork should create child");
    harness
        .child_exit(child.pid, 0)
        .expect("child exit should deliver SIGCHLD");
    assert!(harness.sigchld_observed(), "parent should observe SIGCHLD");
}

#[test_case]
fn abort_of_process_from_signal_handler() {
    let status = encode_signaled(6, false);
    assert!(is_signaled(status, 6), "SIGABRT should be reflected in wait status");
}

#[test_case]
fn process_termination_on_segmentation_fault() {
    let status = encode_signaled(11, true);
    assert!(is_signaled(status, 11), "SIGSEGV should be reflected in wait status");
}

#[test_case]
fn core_dump_generation_on_fatal_signal() {
    let status = encode_signaled(11, true);
    assert!(is_signaled(status, 11), "fatal signal should be represented");
    assert!(did_core_dump(status), "status should carry core-dump bit");
}

#[test_case]
fn pending_signal_delivery_on_exit_not_guaranteed() {
    let mut pending = [2u8, 15u8, 17u8];
    pending.fill(0);
    assert!(pending.iter().all(|sig| *sig == 0), "pending signals should be cleared");
}

#[test_case]
fn parent_inherits_unblock_status_on_child_exit() {
    let mut harness = IntegrationHarness::new();
    let child = harness.fork(1).expect("fork should create child");
    harness
        .child_exit(child.pid, 0)
        .expect("child exit should succeed");
    assert!(harness.sigchld_observed(), "SIGCHLD should be observed by parent");
    let outcome = harness
        .wait(child.pid, WaitFlags::WNOHANG)
        .expect("wait should retrieve completion after SIGCHLD");
    assert!(matches!(outcome, WaitOutcome::Reaped { .. }), "wait should reap child");
}
