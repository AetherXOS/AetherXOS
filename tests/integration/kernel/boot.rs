use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_boot_limine_handoff,
        &test_boot_multiboot2,
        &test_boot_cmdline_parse,
        &test_boot_early_init,
        &test_boot_smp_init,
    ]
}

fn test_boot_limine_handoff() -> TestResult {
    TestResult::pass("integration::kernel::boot::limine_handoff")
}

fn test_boot_multiboot2() -> TestResult {
    TestResult::pass("integration::kernel::boot::multiboot2")
}

fn test_boot_cmdline_parse() -> TestResult {
    TestResult::pass("integration::kernel::boot::cmdline_parse")
}

fn test_boot_early_init() -> TestResult {
    TestResult::pass("integration::kernel::boot::early_init")
}

fn test_boot_smp_init() -> TestResult {
    TestResult::pass("integration::kernel::boot::smp_init")
}
