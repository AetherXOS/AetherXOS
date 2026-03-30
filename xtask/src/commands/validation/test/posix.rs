use anyhow::{Result, bail};
use serde::Serialize;
use std::fs;
use std::process::{Command, Stdio};
use std::time::Instant;

use crate::utils::{paths, report};

#[derive(Serialize)]
struct StepResult {
    name: String,
    ok: bool,
    return_code: i32,
    duration_sec: f64,
    stdout_tail: String,
    stderr_tail: String,
}

#[derive(Serialize)]
struct PosixSummary {
    ok: bool,
    steps: usize,
    failed_steps: usize,
    deep_test_count: usize,
}

pub fn run_gate() -> Result<()> {
    println!("[test::posix] Running POSIX deep tests conformance gate (native)");

    let _root = paths::repo_root();
    let out_dir = paths::resolve("reports/posix_conformance");
    paths::ensure_dir(&out_dir)?;
    let host_target = crate::utils::cargo::detect_host_triple().ok();

    let mut steps = Vec::new();

    let mut host_check_default = vec![
        "cargo".to_string(),
        "check".to_string(),
        "--lib".to_string(),
        "--features".to_string(),
        "posix_deep_tests".to_string(),
    ];
    if let Some(target) = host_target.as_deref() {
        host_check_default.push("--target".to_string());
        host_check_default.push(target.to_string());
    }
    steps.push(run_cmd(
        &host_check_default,
        "cargo check (lib, posix_deep_tests)"
    ));

    let mut host_check_feature_bundle = vec![
        "cargo".to_string(),
        "check".to_string(),
        "--lib".to_string(),
        "--features".to_string(),
        "posix_deep_tests,posix_fs,posix_process,posix_net,posix_signal,posix_time".to_string(),
    ];
    if let Some(target) = host_target.as_deref() {
        host_check_feature_bundle.push("--target".to_string());
        host_check_feature_bundle.push(target.to_string());
    }
    steps.push(run_cmd(
        &host_check_feature_bundle,
        "cargo check (lib, deep POSIX feature bundle)"
    ));

    let deep_test_count = discover_deep_tests()?;
    let discover_ok = deep_test_count > 0;
    steps.push(StepResult {
        name: "discover posix deep #[test] cases".to_string(),
        ok: discover_ok,
        return_code: if discover_ok { 0 } else { 1 },
        duration_sec: 0.0,
        stdout_tail: format!("deep_test_count={}", deep_test_count),
        stderr_tail: String::new(),
    });

    let ok_all = steps.iter().all(|s| s.ok);
    let failed_steps = steps.iter().filter(|s| !s.ok).count();

    let summary = PosixSummary {
        ok: ok_all,
        steps: steps.len(),
        failed_steps,
        deep_test_count,
    };

    let payload = serde_json::json!({
        "summary": summary,
        "steps": steps,
    });
    report::write_json_report(&out_dir.join("summary.json"), &payload)?;

    let mut md = format!(
        "# POSIX Conformance Gate\n\n- ok: `{}`\n- steps: `{}`\n- failed_steps: `{}`\n- deep_test_count: `{}`\n\n## Steps\n\n",
        ok_all, summary.steps, failed_steps, deep_test_count
    );
    for step in &steps {
        md.push_str(&format!(
            "- `{}` => ok `{}` rc `{}` duration `{:.2}s`\n",
            step.name, step.ok, step.return_code, step.duration_sec
        ));
    }
    fs::write(out_dir.join("summary.md"), md)?;

    println!("[test::posix] {} ({} deep tests)", if ok_all { "PASS" } else { "FAIL" }, deep_test_count);
    if !ok_all {
        bail!("POSIX conformance gate failed.");
    }
    Ok(())
}

fn run_cmd(args: &[String], name: &str) -> StepResult {
    let start = Instant::now();
    let root = paths::repo_root();

    let output = Command::new(&args[0])
        .args(args.iter().skip(1))
        .current_dir(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    let duration_sec = start.elapsed().as_secs_f64();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            StepResult {
                name: name.to_string(),
                ok: out.status.success(),
                return_code: out.status.code().unwrap_or(1),
                duration_sec,
                stdout_tail: tail_lines(&stdout, 40),
                stderr_tail: tail_lines(&stderr, 40),
            }
        }
        Err(e) => StepResult {
            name: name.to_string(), ok: false, return_code: 1, duration_sec,
            stdout_tail: String::new(), stderr_tail: format!("Error: {}", e),
        },
    }
}

fn tail_lines(text: &str, n: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let skip = if lines.len() > n { lines.len() - n } else { 0 };
    lines.into_iter().skip(skip).collect::<Vec<_>>().join("\n")
}

fn discover_deep_tests() -> Result<usize> {
    let root = paths::repo_root();
    let deep_dir = root.join("src/modules/posix/tests_deep");
    if !deep_dir.exists() { return Ok(0); }

    let mut count = 0;
    let re = regex::Regex::new(r"^\s*#\[\s*(test|test_case)\s*\]\s*$")?;

    for entry in walkdir::WalkDir::new(&deep_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "rs").unwrap_or(false))
    {
        let text = fs::read_to_string(entry.path())?;
        for line in text.lines() {
            if re.is_match(line) { count += 1; }
        }
    }
    Ok(count)
}
