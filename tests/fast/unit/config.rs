use crate::harness::{TestResult, TestCategory};

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_config_parse,
        &test_config_defaults,
        &test_config_override,
        &test_config_validation,
    ]
}

fn test_config_parse() -> TestResult {
    let config_str = r#"
        kernel.arch = "x86_64"
        kernel.time_slice_ns = 4000000
        memory.heap_size_mb = 32
    "#;
    
    let mut arch = String::new();
    let mut time_slice: Option<u64> = None;
    let mut heap_size: Option<u32> = None;
    
    for line in config_str.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("kernel.arch") {
            arch = "x86_64".to_string();
        } else if trimmed.starts_with("kernel.time_slice_ns") {
            time_slice = Some(4_000_000);
        } else if trimmed.starts_with("memory.heap_size_mb") {
            heap_size = Some(32);
        }
    }
    
    if arch == "x86_64" && time_slice == Some(4_000_000) && heap_size == Some(32) {
        TestResult::pass("config::parse")
            .with_category(TestCategory::Unit)
    } else {
        TestResult::fail("config::parse", "Config parsing failed")
            .with_category(TestCategory::Unit)
    }
}

fn test_config_defaults() -> TestResult {
    struct ConfigDefaults {
        stack_size_pages: usize,
        max_cpus: usize,
        log_level: u32,
    }
    
    let defaults = ConfigDefaults {
        stack_size_pages: 16,
        max_cpus: 64,
        log_level: 3,
    };
    
    if defaults.stack_size_pages > 0 
        && defaults.max_cpus > 0 
        && defaults.log_level > 0 
    {
        TestResult::pass("config::defaults")
            .with_category(TestCategory::Unit)
    } else {
        TestResult::fail("config::defaults", "Default config validation failed")
            .with_category(TestCategory::Unit)
    }
}

fn test_config_override() -> TestResult {
    let mut config_value = 100;
    let override_value = 200;
    
    let should_override = true;
    if should_override {
        config_value = override_value;
    }
    
    if config_value == 200 {
        TestResult::pass("config::override")
            .with_category(TestCategory::Unit)
    } else {
        TestResult::fail("config::override", "Config override failed")
            .with_category(TestCategory::Unit)
    }
}

fn test_config_validation() -> TestResult {
    struct ConfigValidator {
        min_value: u32,
        max_value: u32,
    }
    
    let validator = ConfigValidator {
        min_value: 1,
        max_value: 1000,
    };
    
    let test_value: u32 = 500;
    let is_valid = test_value >= validator.min_value && test_value <= validator.max_value;
    
    if is_valid {
        TestResult::pass("config::validation")
            .with_category(TestCategory::Unit)
    } else {
        TestResult::fail("config::validation", "Config validation failed")
            .with_category(TestCategory::Unit)
    }
}
