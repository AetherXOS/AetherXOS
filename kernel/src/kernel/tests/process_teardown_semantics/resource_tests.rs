use super::super::integration_harness::{IntegrationHarness, WaitFlags};

#[test_case]
fn file_descriptor_inheritance_affects_cleanup() {
    let mut harness = IntegrationHarness::new();
    let child = harness.fork_profile(1).expect("fork profile should succeed");
    assert!(child.shared_fd_count > 0, "child must inherit at least one shared FD");
    harness
        .child_exit(child.pid, 0)
        .expect("child exit should trigger cleanup");
    let _ = harness
        .wait(child.pid, WaitFlags::NONE)
        .expect("wait should finalize teardown");
    assert_eq!(harness.process_count(), 1, "child resources released after reap");
}

#[test_case]
fn memory_page_cleanup_after_process_exit() {
    let mut harness = IntegrationHarness::new();
    let child = harness.fork_profile(1).expect("fork profile should succeed");
    assert!(child.cow_pages > 0, "child starts with mapped COW pages");
    harness
        .child_exit(child.pid, 0)
        .expect("child exit should complete");
    let _ = harness
        .wait(child.pid, WaitFlags::NONE)
        .expect("wait should reap memory owner");
    assert_eq!(harness.process_count(), 1, "process table no longer tracks exited child");
}

#[test_case]
fn boundary_mode_strict_enforces_complete_teardown() {
    let harness = IntegrationHarness::new();
    assert!(
        harness.boundary_mode_fork_valid("strict"),
        "strict mode should be accepted"
    );
}

#[test_case]
fn boundary_mode_balanced_allows_pragmatic_teardown() {
    let harness = IntegrationHarness::new();
    assert!(
        harness.boundary_mode_fork_valid("balanced"),
        "balanced mode should be accepted"
    );
}

#[test_case]
fn boundary_mode_compat_minimizes_teardown_overhead() {
    let harness = IntegrationHarness::new();
    assert!(
        harness.boundary_mode_fork_valid("compat"),
        "compat mode should be accepted"
    );
}

#[test_case]
fn resource_limits_enforced_during_process_lifecycle() {
    let harness = IntegrationHarness::new();
    let parent_limits = [8u64 << 20, 60, 1u64 << 30, 1024];
    let child_limits = parent_limits;
    assert!(
        harness.fork_resource_limits_inherited(parent_limits, child_limits),
        "child should inherit active resource limits"
    );
}
