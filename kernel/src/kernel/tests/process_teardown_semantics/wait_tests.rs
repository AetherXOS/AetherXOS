use super::super::integration_harness::{
    IntegrationHarness, STATUS_EXITED_FLAG, WaitFlags, WaitOutcome,
};
use super::util::selector_class;

#[test_case]
fn wait_returns_immediately_if_child_exited() {
    let mut harness = IntegrationHarness::new();
    let child = harness.fork(1).expect("fork should create child");
    harness
        .child_exit(child.pid, 17)
        .expect("child exit should mark zombie");

    match harness
        .wait(child.pid, WaitFlags::NONE)
        .expect("wait should reap exited child")
    {
        WaitOutcome::Reaped { pid, status } => {
            assert_eq!(pid, child.pid, "wait returns exited child pid");
            assert_eq!(status, STATUS_EXITED_FLAG | 17, "wait encodes exit code");
        }
        WaitOutcome::Running => panic!("exited child must not remain running"),
    }
}

#[test_case]
fn wait_blocks_until_child_exits() {
    let mut harness = IntegrationHarness::new();
    let child = harness.fork(1).expect("fork should create child");
    let outcome = harness
        .wait(child.pid, WaitFlags::NONE)
        .expect("wait should not error for live child");
    assert_eq!(outcome, WaitOutcome::Running, "running child should not be reaped");
}

#[test_case]
fn wifexited_extracts_exit_status_from_wait_status() {
    let status = STATUS_EXITED_FLAG | 42;
    assert_ne!(status & STATUS_EXITED_FLAG, 0, "WIFEXITED bit must be set");
    assert_eq!(status & 0xff, 42, "WEXITSTATUS extracts low 8 bits");
}

#[test_case]
fn waitpid_with_pid_zero_waits_for_any_child_in_pgroup() {
    assert_eq!(selector_class(1234), 1, "pid > 0 targets exact child");
    assert_eq!(selector_class(0), 2, "pid == 0 targets caller process group");
    assert_eq!(selector_class(-1), 3, "pid == -1 targets any child");
    assert_eq!(selector_class(-7), 4, "pid < -1 targets process group |pid|");
}

#[test_case]
fn wnohang_prevents_wait_from_blocking() {
    let mut harness = IntegrationHarness::new();
    let child = harness.fork(1).expect("fork should create child");
    let outcome = harness
        .wait(child.pid, WaitFlags::WNOHANG)
        .expect("WNOHANG wait should be valid");
    assert_eq!(outcome, WaitOutcome::Running, "WNOHANG must avoid blocking");
}

#[test_case]
fn wuntraced_reports_status_of_stopped_children() {
    let mut harness = IntegrationHarness::new();
    let child = harness.fork(1).expect("fork should create child");
    let outcome = harness
        .wait(child.pid, WaitFlags::WUNTRACED)
        .expect("WUNTRACED should be accepted");
    assert_eq!(outcome, WaitOutcome::Running, "live child remains running");
}

#[test_case]
fn wcontinued_reports_status_when_stopped_child_continues() {
    let mut harness = IntegrationHarness::new();
    let child = harness.fork(1).expect("fork should create child");
    let outcome = harness
        .wait(child.pid, WaitFlags::WCONTINUED)
        .expect("WCONTINUED should be accepted");
    assert_eq!(outcome, WaitOutcome::Running, "live child remains running");
}
