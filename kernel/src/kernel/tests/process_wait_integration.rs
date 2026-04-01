/// Process Wait Integration Tests
///
/// Executable no_std integration coverage for wait/reaping semantics.

#[cfg(test)]
mod tests {
    use super::super::integration_harness::{
        IntegrationHarness, WaitFlags, WaitOutcome, STATUS_EXITED_FLAG,
    };

    #[test_case]
    fn wait_with_wnohang_returns_running_for_live_child() {
        let mut harness = IntegrationHarness::new();
        let child = harness.fork(33).expect("fork should succeed");

        let outcome = harness
            .wait(child.pid, WaitFlags::WNOHANG)
            .expect("wait should succeed");

        assert_eq!(outcome, WaitOutcome::Running, "WNOHANG should not block on live child");
    }

    #[test_case]
    fn wait_reaps_exited_child_with_status() {
        let mut harness = IntegrationHarness::new();
        let child = harness.fork(44).expect("fork should succeed");
        harness.child_exit(child.pid, 42).expect("child exit should be recorded");

        let outcome = harness.wait(child.pid, WaitFlags::NONE).expect("wait should succeed");

        match outcome {
            WaitOutcome::Reaped { pid, status } => {
                assert_eq!(pid, child.pid, "reaped pid must match child pid");
                assert_eq!(status & STATUS_EXITED_FLAG, STATUS_EXITED_FLAG, "exit bit must be set");
                assert_eq!(status & 0xff, 42, "exit code must be encoded in low byte");
            }
            WaitOutcome::Running => panic!("expected reaped child"),
        }
    }

    #[test_case]
    fn child_exit_delivers_sigchld_observation() {
        let mut harness = IntegrationHarness::new();
        let child = harness.fork(55).expect("fork should succeed");

        assert!(!harness.sigchld_observed(), "SIGCHLD should be false before exit");
        harness.child_exit(child.pid, 0).expect("child exit should be recorded");
        assert!(harness.sigchld_observed(), "SIGCHLD should be observed after exit");
    }

    #[test_case]
    fn wait_accepts_extended_flag_combinations() {
        let mut harness = IntegrationHarness::new();
        let child = harness.fork(66).expect("fork should succeed");

        let flags = WaitFlags::WNOHANG | WaitFlags::WUNTRACED | WaitFlags::WCONTINUED;
        let outcome = harness.wait(child.pid, flags).expect("wait should succeed");

        assert_eq!(outcome, WaitOutcome::Running, "live child should remain running with extended flags");
    }

    #[test_case]
    fn wait_rejects_unknown_flag_bits() {
        let mut harness = IntegrationHarness::new();
        let child = harness.fork(77).expect("fork should succeed");

        let res = harness.wait(child.pid, 1 << 31);
        assert!(res.is_err(), "unknown wait flag bits must be rejected");
    }

    #[test_case]
    fn wait_rejects_unknown_pid() {
        let mut harness = IntegrationHarness::new();
        let _child = harness.fork(88).expect("fork should succeed");

        let res = harness.wait(999_999, WaitFlags::NONE);
        assert!(res.is_err(), "waiting on unknown pid must fail");
    }
}
