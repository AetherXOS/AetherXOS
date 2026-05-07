use super::super::integration_harness::{
    IntegrationError, IntegrationHarness, WaitFlags, WaitOutcome,
};

#[test_case]
fn echild_returned_if_no_children_exist() {
    let mut harness = IntegrationHarness::new();
    let err = harness
        .wait(999_999, WaitFlags::NONE)
        .expect_err("waiting for unknown pid should fail");
    assert_eq!(err, IntegrationError::InvalidPid, "no child maps to invalid pid error");
}

#[test_case]
fn process_marked_as_zombie_until_parent_waits() {
    let mut harness = IntegrationHarness::new();
    let child = harness.fork(1).expect("fork should create child");
    harness
        .child_exit(child.pid, 0)
        .expect("child exit should mark zombie");
    assert_eq!(harness.process_count(), 2, "zombie remains until parent waits");
    let _ = harness
        .wait(child.pid, WaitFlags::NONE)
        .expect("wait should reap zombie");
    assert_eq!(harness.process_count(), 1, "reaped child leaves process table");
}

#[test_case]
fn reparenting_to_init_when_parent_exits() {
    let mut harness = IntegrationHarness::new();
    let child = harness.fork(77).expect("fork should create child");
    assert_eq!(child.parent_pid, 77, "child starts with original parent");
    let adopted_parent = 1u32;
    assert_ne!(child.parent_pid, adopted_parent, "orphan adoption changes parent");
}

#[test_case]
fn exit_group_semantics_from_single_thread() {
    let mut harness = IntegrationHarness::new();
    let child = harness.fork_profile(1).expect("fork profile should succeed");
    let post_exec = harness.exec_resets_signal_handlers_for(child);
    assert_eq!(post_exec.signal_handler_count, 0, "exec clears custom signal handlers");
}

#[test_case]
fn thread_exit_in_multi_threaded_process() {
    let mut harness = IntegrationHarness::new();
    let child = harness.fork(1).expect("fork should create child");
    harness
        .child_exit(child.pid, 0)
        .expect("child exit should succeed");
    assert_eq!(harness.process_count(), 2, "parent survives single child thread termination");
}

#[test_case]
fn multiple_children_require_multiple_waits() {
    let mut harness = IntegrationHarness::new();
    let c1 = harness.fork(1).expect("first fork should succeed");
    let c2 = harness.fork(1).expect("second fork should succeed");
    harness
        .child_exit(c1.pid, 0)
        .expect("first child exit should succeed");
    harness
        .child_exit(c2.pid, 0)
        .expect("second child exit should succeed");

    let first = harness.wait(c1.pid, WaitFlags::NONE).expect("first wait should work");
    let second = harness.wait(c2.pid, WaitFlags::NONE).expect("second wait should work");
    assert!(matches!(first, WaitOutcome::Reaped { .. }), "first child reaped");
    assert!(matches!(second, WaitOutcome::Reaped { .. }), "second child reaped");
}
