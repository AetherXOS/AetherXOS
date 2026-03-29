use alloc::{format, vec::Vec};
use core::fmt::{self, Debug, Display};
use core::ops::{Fn, FnOnce};
use core::option::Option;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::time::Duration;

pub struct TestResult {
    pub name: &'static str,
    pub passed: bool,
    pub message: Option<&'static str>,
    pub duration_ns: u64,
    pub category: TestCategory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestCategory {
    Unit,
    Lint,
    Security,
    Audit,
    Format,
}

impl Display for TestCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TestCategory::Unit => write!(f, "unit"),
            TestCategory::Lint => write!(f, "lint"),
            TestCategory::Security => write!(f, "security"),
            TestCategory::Audit => write!(f, "audit"),
            TestCategory::Format => write!(f, "format"),
        }
    }
}

impl TestResult {
    pub fn pass(name: &'static str) -> Self {
        Self {
            name,
            passed: true,
            message: None,
            duration_ns: 0,
            category: TestCategory::Unit,
        }
    }

    pub fn fail(name: &'static str, message: &'static str) -> Self {
        Self {
            name,
            passed: false,
            message: Some(message),
            duration_ns: 0,
            category: TestCategory::Unit,
        }
    }

    pub fn with_duration(mut self, duration_ns: u64) -> Self {
        self.duration_ns = duration_ns;
        self
    }

    pub fn with_category(mut self, category: TestCategory) -> Self {
        self.category = category;
        self
    }

    pub fn duration_ms(&self) -> f64 {
        self.duration_ns as f64 / 1_000_000.0
    }
}

pub trait Testable: Fn() -> TestResult {
    fn run(&self) -> TestResult;
}

impl<T> Testable for T
where
    T: Fn() -> TestResult,
{
    fn run(&self) -> TestResult {
        self()
    }
}

pub struct TestRunnerConfig {
    pub filter: Option<&'static str>,
    pub verbose: bool,
    pub parallel: bool,
}

impl Default for TestRunnerConfig {
    fn default() -> Self {
        Self {
            filter: None,
            verbose: false,
            parallel: false,
        }
    }
}

pub struct TestSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub total_duration_ns: u64,
    pub by_category: [CategoryStats; 5],
}

pub struct CategoryStats {
    pub category: TestCategory,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
}

impl CategoryStats {
    pub fn new(category: TestCategory) -> Self {
        Self {
            category,
            passed: 0,
            failed: 0,
            skipped: 0,
        }
    }
}

impl TestSummary {
    pub fn new() -> Self {
        Self {
            total: 0,
            passed: 0,
            failed: 0,
            skipped: 0,
            total_duration_ns: 0,
            by_category: [
                CategoryStats::new(TestCategory::Unit),
                CategoryStats::new(TestCategory::Lint),
                CategoryStats::new(TestCategory::Security),
                CategoryStats::new(TestCategory::Audit),
                CategoryStats::new(TestCategory::Format),
            ],
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total == 0 {
            return 100.0;
        }
        (self.passed as f64 / self.total as f64) * 100.0
    }

    pub fn total_duration_ms(&self) -> f64 {
        self.total_duration_ns as f64 / 1_000_000.0
    }
}

static TESTS_RUN: AtomicUsize = AtomicUsize::new(0);
static TESTS_PASSED: AtomicUsize = AtomicUsize::new(0);
static TESTS_FAILED: AtomicUsize = AtomicUsize::new(0);

pub fn reset_counters() {
    TESTS_RUN.store(0, Ordering::Relaxed);
    TESTS_PASSED.store(0, Ordering::Relaxed);
    TESTS_FAILED.store(0, Ordering::Relaxed);
}

pub fn get_run_count() -> usize {
    TESTS_RUN.load(Ordering::Relaxed)
}

pub fn get_passed_count() -> usize {
    TESTS_PASSED.load(Ordering::Relaxed)
}

pub fn get_failed_count() -> usize {
    TESTS_FAILED.load(Ordering::Relaxed)
}

#[inline]
pub fn measure_duration_ns<F: FnOnce() -> TestResult>(f: F) -> TestResult {
    let start = unsafe { core::arch::x86_64::_rdtsc() } as u64;
    let mut result = f();
    let end = unsafe { core::arch::x86_64::_rdtsc() } as u64;
    result.duration_ns = end.saturating_sub(start);
    result
}

pub fn test_runner_with_config(tests: &[&dyn Fn() -> TestResult], config: TestRunnerConfig) {
    println!("\n");
    println!("================================================");
    println!("       AetherXOS FAST Test Suite");
    println!("================================================\n");

    let mut summary = TestSummary::new();
    let mut failed_tests: Vec<&TestResult> = Vec::new();

    for test_fn in tests {
        let result = measure_duration_ns(|| test_fn.run());

        if let Some(filter) = config.filter {
            if !result.name.contains(filter) {
                summary.skipped += 1;
                continue;
            }
        }

        summary.total += 1;
        summary.total_duration_ns += result.duration_ns;

        let cat_idx = result.category as usize;
        if result.passed {
            summary.passed += 1;
            summary.by_category[cat_idx].passed += 1;
            TESTS_PASSED.fetch_add(1, Ordering::Relaxed);
        } else {
            summary.failed += 1;
            summary.by_category[cat_idx].failed += 1;
            failed_tests.push(&result);
            TESTS_FAILED.fetch_add(1, Ordering::Relaxed);
        }

        TESTS_RUN.fetch_add(1, Ordering::Relaxed);

        if config.verbose || !result.passed {
            let status = if result.passed { "PASS" } else { "FAIL" };
            let duration_str = format!("{:.3}ms", result.duration_ms());
            
            if result.passed {
                println!("[{}] {} ({})", status, result.name, duration_str);
            } else {
                println!(
                    "[{}] {} - {} ({})",
                    status,
                    result.name,
                    result.message.unwrap_or("Unknown error"),
                    duration_str
                );
            }
        }
    }

    print_summary(&summary, &failed_tests, config.verbose);

    if summary.failed > 0 {
        panic!("{} tests failed", summary.failed);
    }
}

fn print_summary(summary: &TestSummary, failed_tests: &[&TestResult], verbose: bool) {
    println!("\n");
    println!("================================================");
    println!("                 TEST SUMMARY");
    println!("================================================\n");

    println!("Results:");
    println!("  Total:   {}", summary.total);
    println!("  Passed:  {}", summary.passed);
    println!("  Failed:  {}", summary.failed);
    println!("  Skipped: {}", summary.skipped);
    println!();
    println!("Success Rate: {:.2}%", summary.success_rate());
    println!("Total Duration: {:.2}ms", summary.total_duration_ms());
    println!();

    println!("By Category:");
    for stats in &summary.by_category {
        let total = stats.passed + stats.failed + stats.skipped;
        if total > 0 {
            println!(
                "  {:10} : {} passed, {} failed, {} skipped",
                format!("{}:", stats.category),
                stats.passed,
                stats.failed,
                stats.skipped
            );
        }
    }
    println!();

    if !failed_tests.is_empty() {
        println!("Failed Tests:");
        for result in failed_tests {
            println!(
                "  - {} : {}",
                result.name,
                result.message.unwrap_or("No message")
            );
        }
        println!();
    }

    println!("================================================");
}

pub fn test_runner(tests: &[&dyn Fn() -> TestResult]) {
    let config = TestRunnerConfig {
        filter: None,
        verbose: true,
        parallel: false,
    };
    test_runner_with_config(tests, config);
}

#[macro_export]
macro_rules! assert_true {
    ($cond:expr $(,)?) => {
        if !$cond {
            return $crate::harness::TestResult::fail(
                core::stringify!($cond),
                "Assertion failed: condition was false"
            );
        }
    };
}

#[macro_export]
macro_rules! assert_eq {
    ($left:expr, $right:expr $(,)?) => {
        {
            let left_val = &$left;
            let right_val = &$right;
            if left_val != right_val {
                return $crate::harness::TestResult::fail(
                    core::stringify!($left, $right),
                    "Assertion failed: values not equal"
                );
            }
        }
    };
}

#[macro_export]
macro_rules! assert_ne {
    ($left:expr, $right:expr $(,)?) => {
        {
            let left_val = &$left;
            let right_val = &$right;
            if left_val == right_val {
                return $crate::harness::TestResult::fail(
                    core::stringify!($left, $right),
                    "Assertion failed: values were equal"
                );
            }
        }
    };
}

#[macro_export]
macro_rules! assert_ok {
    ($result:expr $(,)?) => {
        match $result {
            Ok(_) => {}
            Err(_) => {
                return $crate::harness::TestResult::fail(
                    core::stringify!($result),
                    "Expected Ok, got Err"
                );
            }
        }
    };
}

#[macro_export]
macro_rules! assert_err {
    ($result:expr $(,)?) => {
        match $result {
            Ok(_) => {
                return $crate::harness::TestResult::fail(
                    core::stringify!($result),
                    "Expected Err, got Ok"
                );
            }
            Err(_) => {}
        }
    };
}

#[macro_export]
macro_rules! test_case {
    ($name:ident, $body:expr) => {
        pub fn $name() -> $crate::harness::TestResult {
            $body
        }
    };
}
