pub(crate) fn assert_boot_self_tests() {
    let selftest = aethercore::kernel::boot_health::run_boot_self_tests();
    if !selftest.passed() {
        aethercore::klog_error!(
            "Boot self-test failed checks={} failures={} last_error=E{}",
            selftest.checks,
            selftest.failures,
            selftest.last_error_code
        );
        aethercore::kernel::fatal_halt("boot_self_test_failed");
    }
}
