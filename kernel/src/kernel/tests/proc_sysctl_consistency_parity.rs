/// Proc/Sysctl Consistency Parity Tests
///
/// Validates procfs/sysctl behavior expected by distro tooling and container stacks.

#[cfg(test)]
mod tests {
    use super::super::integration_harness::IntegrationHarness;

    /// TestCase: proc mount exposes core nodes
    #[test_case]
    fn proc_mount_exposes_core_nodes() {
        let harness = IntegrationHarness::new();
        assert!(
            harness.proc_root_contains_core_nodes(),
            "/proc should expose self, stat, meminfo, uptime"
        );
    }

    /// TestCase: /proc/self resolves to caller pid
    #[test_case]
    fn proc_self_resolves_to_current_pid() {
        let harness = IntegrationHarness::new();
        let pid = harness
            .resolve_proc_self_pid(1337)
            .expect("proc self resolution should succeed for non-zero pid");
        assert_eq!(pid, 1337, "/proc/self must resolve to caller PID");
    }

    /// TestCase: /proc/pid/stat layout remains stable
    #[test_case]
    fn proc_pid_stat_layout_remains_stable() {
        let harness = IntegrationHarness::new();
        let field_count = harness
            .proc_pid_stat_field_count(100)
            .expect("stat should be generated for valid pid");
        assert!(
            field_count >= 40,
            "proc stat fields should keep Linux-compatible ordering"
        );
    }

    /// TestCase: /proc/pid/status includes identity fields
    #[test_case]
    fn proc_pid_status_includes_identity_fields() {
        let harness = IntegrationHarness::new();
        assert!(
            harness
                .proc_pid_status_has_identity_fields(100)
                .expect("status should be generated for valid pid"),
            "status should include Name, State, Tgid, Pid, PPid, Uid, Gid"
        );
    }

    /// TestCase: /proc/meminfo reports non-negative counters
    #[test_case]
    fn proc_meminfo_reports_non_negative_counters() {
        let harness = IntegrationHarness::new();
        assert!(
            harness.proc_meminfo_reports_non_negative_counters(),
            "memory counters should be parseable and non-negative"
        );
    }

    /// TestCase: /proc/uptime is monotonic
    #[test_case]
    fn proc_uptime_is_monotonic() {
        let mut harness = IntegrationHarness::new();
        let first = harness.read_proc_uptime_seconds();
        let second = harness.read_proc_uptime_seconds();
        assert!(second >= first, "uptime should not go backwards between reads");
    }

    /// TestCase: /proc/sys net subtree is discoverable
    #[test_case]
    fn proc_sys_net_subtree_is_discoverable() {
        let harness = IntegrationHarness::new();
        assert!(
            harness.proc_sys_net_visible(),
            "net sysctl subtree should be visible to tooling"
        );
    }

    /// TestCase: read-only sysctl rejects writes
    #[test_case]
    fn readonly_sysctl_rejects_writes() {
        let harness = IntegrationHarness::new();
        let res = harness.sysctl_write_readonly_key("kernel.ostype", "changed");
        assert!(res.is_err(), "read-only sysctl keys should return permission error on write");
    }

    /// TestCase: writable sysctl roundtrip keeps value
    #[test_case]
    fn writable_sysctl_roundtrip_keeps_value() {
        let mut harness = IntegrationHarness::new();
        let wrote = harness
            .sysctl_write_pid_max_from_str("131072")
            .expect("valid sysctl value should write");
        let (_proc_val, sysctl_val) = harness.proc_sysctl_pid_max_values();
        assert_eq!(wrote, 131072, "write should parse exact numeric value");
        assert_eq!(sysctl_val, 131072, "write then read should return same normalized value");
    }

    /// TestCase: sysctl parser rejects malformed input
    #[test_case]
    fn sysctl_parser_rejects_malformed_input() {
        let mut harness = IntegrationHarness::new();
        let malformed = harness.sysctl_write_pid_max_from_str("not-a-number");
        assert!(malformed.is_err(), "malformed tokens should return EINVAL-like failure");
    }

    /// TestCase: namespace-aware proc visibility
    #[test_case]
    fn namespace_aware_proc_visibility() {
        let harness = IntegrationHarness::new();
        let (first, second) = harness
            .namespace_visible_pids(1000)
            .expect("namespace visibility query should succeed");
        assert!(
            first >= 1000 && second >= first,
            "PID namespaces should filter visible process entries"
        );
    }

    /// TestCase: boundary strict mode validates proc/sysctl access
    #[test_case]
    fn boundary_strict_mode_validates_proc_sysctl_access() {
        let harness = IntegrationHarness::new();
        assert!(
            harness.boundary_mode_proc_sysctl_valid("strict"),
            "strict mode should enforce stronger proc/sysctl checks"
        );
    }

    /// TestCase: boundary balanced mode provides standard behavior
    #[test_case]
    fn boundary_balanced_mode_provides_standard_behavior() {
        let harness = IntegrationHarness::new();
        assert!(
            harness.boundary_mode_proc_sysctl_valid("balanced"),
            "balanced mode should preserve mainstream Linux behavior"
        );
    }

    /// TestCase: boundary compat mode allows reduced overhead
    #[test_case]
    fn boundary_compat_mode_allows_reduced_overhead() {
        let harness = IntegrationHarness::new();
        assert!(
            harness.boundary_mode_proc_sysctl_valid("compat"),
            "compat mode should keep behavior while reducing validation overhead"
        );
    }
}
