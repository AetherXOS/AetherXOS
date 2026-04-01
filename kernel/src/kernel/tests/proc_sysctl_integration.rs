/// Proc/Sysctl Integration Tests
///
/// Executable no_std integration coverage for proc and sysctl consistency.

#[cfg(test)]
mod tests {
    use super::super::integration_harness::IntegrationHarness;

    #[test_case]
    fn proc_status_threads_reports_positive_value() {
        let mut harness = IntegrationHarness::new();
        harness.set_proc_status_threads(6);

        let threads = harness
            .proc_status_threads(1234)
            .expect("proc status should return thread count");

        assert_eq!(threads, 6, "Threads field must reflect process thread count");
    }

    #[test_case]
    fn proc_sysctl_pid_max_values_are_consistent_by_default() {
        let harness = IntegrationHarness::new();

        let (proc_val, sysctl_val) = harness.proc_sysctl_pid_max_values();

        assert_eq!(proc_val, sysctl_val, "default pid_max sources should match");
        assert!(
            harness.validate_proc_sysctl_consistency().is_ok(),
            "consistency check should pass when values match"
        );
    }

    #[test_case]
    fn proc_sysctl_consistency_detects_pid_max_divergence() {
        let mut harness = IntegrationHarness::new();
        harness.set_proc_pid_max(65535);
        harness.set_sysctl_pid_max(32768);

        let res = harness.validate_proc_sysctl_consistency();

        assert!(res.is_err(), "divergent pid_max values must fail consistency check");
    }

    #[test_case]
    fn proc_status_rejects_invalid_pid_zero() {
        let harness = IntegrationHarness::new();

        let res = harness.proc_status_threads(0);

        assert!(res.is_err(), "pid 0 should be treated as invalid in proc view");
    }

    #[test_case]
    fn proc_status_threads_clamps_to_minimum_one() {
        let mut harness = IntegrationHarness::new();
        harness.set_proc_status_threads(0);

        let threads = harness
            .proc_status_threads(42)
            .expect("proc status should return clamped thread count");

        assert_eq!(threads, 1, "thread count should clamp to at least one");
    }

    #[test_case]
    fn writable_sysctl_pid_max_accepts_valid_numeric_value() {
        let mut harness = IntegrationHarness::new();

        let written = harness
            .sysctl_write_pid_max_from_str("262144")
            .expect("valid pid_max value should be accepted");

        let (_proc_val, sysctl_val) = harness.proc_sysctl_pid_max_values();
        assert_eq!(written, 262144, "write result should reflect parsed numeric value");
        assert_eq!(sysctl_val, 262144, "sysctl pid_max should persist written value");
    }

    #[test_case]
    fn sysctl_parser_rejects_malformed_numeric_tokens() {
        let mut harness = IntegrationHarness::new();

        let malformed = harness.sysctl_write_pid_max_from_str("12x34");
        let empty = harness.sysctl_write_pid_max_from_str("");

        assert!(malformed.is_err(), "malformed numeric token must be rejected");
        assert!(empty.is_err(), "empty sysctl value must be rejected");
    }

    #[test_case]
    fn readonly_sysctl_write_is_rejected() {
        let harness = IntegrationHarness::new();

        let res = harness.sysctl_write_readonly_key("kernel.ostype", "HyperCore");
        assert!(res.is_err(), "writes to read-only sysctl key must be rejected");
    }
}
