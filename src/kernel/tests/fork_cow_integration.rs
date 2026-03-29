/// Fork CoW Integration Tests
///
/// Executable no_std integration coverage for fork semantics.

#[cfg(test)]
mod tests {
    use super::super::integration_harness::{IntegrationHarness, WaitFlags};

    #[test_case]
    fn fork_creates_child_with_parent_link() {
        let mut harness = IntegrationHarness::new();
        let parent_pid = 7;

        let child = harness.fork(parent_pid).expect("fork should succeed");

        assert!(child.pid > parent_pid, "child pid should be larger than parent pid");
        assert_eq!(child.parent_pid, parent_pid, "parent relationship must be preserved");
    }

    #[test_case]
    fn fork_sets_cow_and_fd_sharing_shape() {
        let mut harness = IntegrationHarness::new();
        let child = harness.fork(11).expect("fork should succeed");

        assert!(child.cow_pages > 0, "child should have CoW pages");
        assert!(child.shared_fd_count >= 1, "child should inherit at least one fd");
        assert!(child.signal_handler_count >= 1, "child should copy signal handlers");
    }

    #[test_case]
    fn fork_child_can_be_reaped_after_exit() {
        let mut harness = IntegrationHarness::new();
        let child = harness.fork(22).expect("fork should succeed");

        harness
            .child_exit(child.pid, 42)
            .expect("child exit should be recorded");

        let waited = harness.wait(child.pid, 0).expect("wait should succeed");
        assert!(matches!(waited, super::super::integration_harness::WaitOutcome::Reaped { .. }));
    }

    #[test_case]
    fn fork_capacity_is_recoverable_after_reap() {
        let mut harness = IntegrationHarness::new();
        let mut pids = [0u32; 32];
        let mut created = 0usize;

        while created < pids.len() {
            let child = harness.fork(99).expect("fork should fill process slots");
            pids[created] = child.pid;
            created += 1;
        }

        assert!(harness.fork(99).is_err(), "fork must fail when all slots are occupied");

        let mut idx = 0usize;
        while idx < pids.len() {
            harness.child_exit(pids[idx], 0).expect("child exit should be recorded");
            harness
                .wait(pids[idx], WaitFlags::NONE)
                .expect("reap should succeed");
            idx += 1;
        }

        assert_eq!(harness.process_count(), 0, "all children should be reaped");
        assert!(harness.fork(99).is_ok(), "fork should succeed again after slots are freed");
    }
}
