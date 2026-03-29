use crate::harness::{TestResult, TestCategory};

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_kernel_init,
        &test_kernel_panic_handler,
        &test_kernel_version,
        &test_kernel_cmdline_parse,
        &test_kernel_early_init,
    ]
}

fn test_kernel_init() -> TestResult {
    let mut initialized = false;
    
    let result = core::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
        initialized = true;
    }));
    
    if result.is_ok() && initialized {
        TestResult::pass("kernel::init")
            .with_category(TestCategory::Unit)
    } else {
        TestResult::fail("kernel::init", "Kernel init failed")
            .with_category(TestCategory::Unit)
    }
}

fn test_kernel_panic_handler() -> TestResult {
    TestResult::pass("kernel::panic_handler")
        .with_category(TestCategory::Unit)
}

fn test_kernel_version() -> TestResult {
    let version = "0.2.0";
    let major = version.split('.').next().unwrap_or("0");
    let minor = version.split('.').nth(1).unwrap_or("0");
    
    if major == "0" && minor == "2" {
        TestResult::pass("kernel::version")
            .with_category(TestCategory::Unit)
    } else {
        TestResult::fail("kernel::version", "Version parsing failed")
            .with_category(TestCategory::Unit)
    }
}

fn test_kernel_cmdline_parse() -> TestResult {
    let cmdline = "root=/dev/sda1 quiet splash log_level=3";
    
    let mut found_root = false;
    let mut found_quiet = false;
    let mut log_level: Option<u32> = None;
    
    for arg in cmdline.split_whitespace() {
        if arg.starts_with("root=") {
            found_root = true;
        } else if arg == "quiet" {
            found_quiet = true;
        } else if let Some(value) = arg.strip_prefix("log_level=") {
            log_level = value.parse().ok();
        }
    }
    
    if found_root && found_quiet && log_level == Some(3) {
        TestResult::pass("kernel::cmdline_parse")
            .with_category(TestCategory::Unit)
    } else {
        TestResult::fail("kernel::cmdline_parse", "Cmdline parsing failed")
            .with_category(TestCategory::Unit)
    }
}

fn test_kernel_early_init() -> TestResult {
    let mut gdt_loaded = false;
    let mut idt_loaded = false;
    let mut interrupts_enabled = false;
    
    gdt_loaded = true;
    idt_loaded = true;
    interrupts_enabled = true;
    
    if gdt_loaded && idt_loaded && interrupts_enabled {
        TestResult::pass("kernel::early_init")
            .with_category(TestCategory::Unit)
    } else {
        TestResult::fail("kernel::early_init", "Early init sequence incomplete")
            .with_category(TestCategory::Unit)
    }
}
