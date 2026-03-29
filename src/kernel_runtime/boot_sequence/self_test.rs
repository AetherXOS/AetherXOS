pub(crate) fn assert_boot_self_tests() {
    let selftest = hypercore::kernel::boot_health::run_boot_self_tests();
    if !selftest.passed() {
        hypercore::klog_error!(
            "Boot self-test failed checks={} failures={} last_error=E{}",
            selftest.checks,
            selftest.failures,
            selftest.last_error_code
        );
        hypercore::kernel::fatal_halt("boot_self_test_failed");
    }
}
