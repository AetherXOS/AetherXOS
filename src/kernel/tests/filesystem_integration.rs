/// Filesystem Integration Tests
///
/// Executable no_std integration coverage for metadata-style filesystem behavior.

#[cfg(test)]
mod tests {
    use super::super::integration_harness::IntegrationHarness;

    #[test_case]
    fn stat_reports_regular_file_mode_and_size() {
        let harness = IntegrationHarness::new();

        let stat = harness.stat("/tmp/demo", 4096).expect("stat should succeed");

        assert_eq!(stat.mode & 0o170000, 0o100000, "mode should indicate regular file");
        assert_eq!(stat.size, 4096, "stat size must match created file size");
    }

    #[test_case]
    fn stat_provides_non_zero_inode() {
        let harness = IntegrationHarness::new();

        let stat = harness.stat("/tmp/demo2", 128).expect("stat should succeed");

        assert!(stat.inode > 0, "inode must be non-zero");
    }

    #[test_case]
    fn mmap_returns_page_aligned_address() {
        let harness = IntegrationHarness::new();

        let addr = harness.mmap(0, 4096).expect("mmap should succeed");

        assert_eq!(addr % 4096, 0, "mmap address must be page aligned");
    }

    #[test_case]
    fn mmap_rejects_zero_size_request() {
        let harness = IntegrationHarness::new();

        let res = harness.mmap(0, 0);

        assert!(res.is_err(), "mmap with zero size must fail");
    }

    #[test_case]
    fn mmap_strict_fixed_hint_rejects_unaligned_address() {
        let harness = IntegrationHarness::new();

        let res = harness.mmap_with_fixed_hint(0x4003, 4096, true);

        assert!(res.is_err(), "strict hint mode must reject unaligned hint");
    }

    #[test_case]
    fn mmap_strict_fixed_hint_accepts_aligned_address() {
        let harness = IntegrationHarness::new();

        let addr = harness
            .mmap_with_fixed_hint(0x8000, 4096, true)
            .expect("strict mode should accept aligned hint");

        assert_eq!(addr, 0x8000, "aligned hint should be preserved");
    }
}
