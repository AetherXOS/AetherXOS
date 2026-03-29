use core::fmt::Debug;

pub struct TestResult {
    pub name: &'static str,
    pub passed: bool,
    pub message: Option<&'static str>,
    pub duration_ns: Option<u64>,
}

impl TestResult {
    pub fn pass(name: &'static str) -> Self {
        Self {
            name,
            passed: true,
            message: None,
            duration_ns: None,
        }
    }

    pub fn fail(name: &'static str, message: &'static str) -> Self {
        Self {
            name,
            passed: false,
            message: Some(message),
            duration_ns: None,
        }
    }

    pub fn with_duration(mut self, duration_ns: u64) -> Self {
        self.duration_ns = Some(duration_ns);
        self
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

pub fn test_runner(tests: &[&dyn Fn() -> TestResult]) {
    println!("\n=== AetherXOS Integration Test Suite ===\n");

    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut total_duration_ns = 0u64;

    for test in tests {
        let result = test.run();

        if let Some(duration) = result.duration_ns {
            total_duration_ns += duration;
        }

        if result.passed {
            println!("[PASS] {}", result.name);
            passed += 1;
        } else {
            println!(
                "[FAIL] {} - {}",
                result.name,
                result.message.unwrap_or("Unknown error")
            );
            failed += 1;
        }
    }

    println!("\n=== Test Summary ===");
    println!("Total:  {}", passed + failed);
    println!("Passed: {}", passed);
    println!("Failed: {}", failed);
    println!("Duration: {} ms", total_duration_ns / 1_000_000);

    if failed > 0 {
        panic!("{} tests failed", failed);
    }
}
