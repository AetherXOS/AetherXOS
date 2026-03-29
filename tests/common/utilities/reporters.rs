use alloc::string::String;
use alloc::vec::Vec;

pub struct TestReport {
    pub name: String,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub duration_ns: u64,
    pub failures: Vec<FailureInfo>,
}

pub struct FailureInfo {
    pub test_name: String,
    pub message: String,
    pub location: Option<&'static str>,
}

impl TestReport {
    pub fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
            passed: 0,
            failed: 0,
            skipped: 0,
            duration_ns: 0,
            failures: Vec::new(),
        }
    }

    pub fn add_pass(&mut self) {
        self.passed += 1;
    }

    pub fn add_fail(&mut self, test_name: &str, message: &str, location: Option<&'static str>) {
        self.failed += 1;
        self.failures.push(FailureInfo {
            test_name: String::from(test_name),
            message: String::from(message),
            location,
        });
    }

    pub fn add_skip(&mut self) {
        self.skipped += 1;
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.passed + self.failed;
        if total == 0 {
            100.0
        } else {
            (self.passed as f64 / total as f64) * 100.0
        }
    }

    pub fn format_summary(&self) -> String {
        alloc::format!(
            "Test Report: {}\n\
             Passed: {}\n\
             Failed: {}\n\
             Skipped: {}\n\
             Success Rate: {:.2}%\n\
             Duration: {:.2}ms",
            self.name,
            self.passed,
            self.failed,
            self.skipped,
            self.success_rate(),
            self.duration_ns as f64 / 1_000_000.0
        )
    }
}

pub fn format_duration(ns: u64) -> String {
    if ns < 1_000 {
        alloc::format!("{}ns", ns)
    } else if ns < 1_000_000 {
        alloc::format!("{:.2}us", ns as f64 / 1_000.0)
    } else if ns < 1_000_000_000 {
        alloc::format!("{:.2}ms", ns as f64 / 1_000_000.0)
    } else {
        alloc::format!("{:.2}s", ns as f64 / 1_000_000_000.0)
    }
}

pub fn format_bytes(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = KB * 1024;
    const GB: usize = MB * 1024;

    if bytes >= GB {
        alloc::format!("{:.2}GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        alloc::format!("{:.2}MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        alloc::format!("{:.2}KB", bytes as f64 / KB as f64)
    } else {
        alloc::format!("{}B", bytes)
    }
}
