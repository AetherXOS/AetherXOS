use anyhow::{Context, Result};
use serde::Serialize;
use std::path::Path;

pub struct JunitSingleCaseReport<'a> {
    pub suite_name: &'a str,
    pub case_name: &'a str,
    pub class_name: &'a str,
    pub duration_secs: f64,
    pub passed: bool,
    pub failure_message: Option<&'a str>,
    pub stdout: &'a str,
    pub stderr: &'a str,
}

fn ensure_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create report directory: {}", parent.display()))?;
    }
    Ok(())
}

/// Write a JSON report to disk, creating parent directories as needed.
#[allow(dead_code)]
pub fn write_json_report<T: Serialize>(path: &Path, data: &T) -> Result<()> {
    ensure_parent(path)?;
    let json = serde_json::to_string_pretty(data)
        .context("Failed to serialize report to JSON")?;
    std::fs::write(path, json)
        .with_context(|| format!("Failed to write report: {}", path.display()))?;
    println!("[report] Written: {}", path.display());
    Ok(())
}

#[allow(dead_code)]
pub fn write_text_report(path: &Path, content: &str) -> Result<()> {
    ensure_parent(path)?;
    std::fs::write(path, content)
        .with_context(|| format!("Failed to write text report: {}", path.display()))?;
    println!("[report] Written: {}", path.display());
    Ok(())
}

#[allow(dead_code)]
pub fn write_junit_single_case(
    path: &Path,
    report: &JunitSingleCaseReport<'_>,
) -> Result<()> {
    ensure_parent(path)?;
    let failures = if report.passed { 0 } else { 1 };
    let failure_tag = if report.passed {
        String::new()
    } else {
        format!(
            "<failure message=\"{}\"></failure>",
            report.failure_message.unwrap_or("test failure")
        )
    };

    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<testsuites tests="1" failures="{failures}" errors="0" time="{time:.3}">
    <testsuite name="{suite_name}" tests="1" failures="{failures}" errors="0" time="{time:.3}">
        <testcase name="{case_name}" classname="{class_name}" time="{time:.3}">
            {failure_tag}
            <system-out><![CDATA[{stdout}]]></system-out>
            <system-err><![CDATA[{stderr}]]></system-err>
        </testcase>
    </testsuite>
</testsuites>"#,
        failures = failures,
        time = report.duration_secs,
        suite_name = report.suite_name,
        case_name = report.case_name,
        class_name = report.class_name,
        failure_tag = failure_tag,
        stdout = report.stdout.replace("]]>", "]]>]]&gt;<![CDATA["),
        stderr = report.stderr.replace("]]>", "]]>]]&gt;<![CDATA["),
    );

    std::fs::write(path, xml)
        .with_context(|| format!("Failed to write junit report: {}", path.display()))?;
    println!("[report] Written: {}", path.display());
    Ok(())
}

/// Returns the current UTC timestamp as an ISO-8601 string.
#[allow(dead_code)]
pub fn utc_now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::{write_json_report, write_junit_single_case, write_text_report, JunitSingleCaseReport};
    use serde::Serialize;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[derive(Serialize)]
    struct GoldenJsonReport {
        name: &'static str,
        value: u32,
    }

    fn test_path(filename: &str) -> PathBuf {
        let nonce = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time must be after unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("xtask_report_tests_{}_{}", ts, nonce));
        fs::create_dir_all(&dir).expect("test temp dir must be creatable");
        dir.join(filename)
    }

    #[test]
    fn write_text_report_matches_golden() {
        let path = test_path("report.txt");
        let content = "line-1\nline-2\n";

        write_text_report(&path, content).expect("text report must be written");

        let written = fs::read_to_string(&path).expect("text report must be readable");
        assert_eq!(written, content);
    }

    #[test]
    fn write_json_report_matches_golden() {
        let path = test_path("report.json");
        let payload = GoldenJsonReport {
            name: "smoke",
            value: 42,
        };

        write_json_report(&path, &payload).expect("json report must be written");

        let written = fs::read_to_string(&path).expect("json report must be readable");
        let expected = "{\n  \"name\": \"smoke\",\n  \"value\": 42\n}";
        assert_eq!(written, expected);
    }

    #[test]
    fn write_junit_report_matches_golden_failure_case() {
        let path = test_path("report.xml");
        let report = JunitSingleCaseReport {
            suite_name: "qemu_smoke",
            case_name: "boot",
            class_name: "ops.qemu",
            duration_secs: 1.5,
            passed: false,
            failure_message: Some("boot timed out"),
            stdout: "hello",
            stderr: "fail ]]> sentinel",
        };

        write_junit_single_case(&path, &report).expect("junit report must be written");

        let written = fs::read_to_string(&path).expect("junit report must be readable");
        let expected = r#"<?xml version="1.0" encoding="UTF-8"?>
<testsuites tests="1" failures="1" errors="0" time="1.500">
    <testsuite name="qemu_smoke" tests="1" failures="1" errors="0" time="1.500">
        <testcase name="boot" classname="ops.qemu" time="1.500">
            <failure message="boot timed out"></failure>
            <system-out><![CDATA[hello]]></system-out>
            <system-err><![CDATA[fail ]]>]]&gt;<![CDATA[ sentinel]]></system-err>
        </testcase>
    </testsuite>
</testsuites>"#;
        assert_eq!(written, expected);
    }
}
