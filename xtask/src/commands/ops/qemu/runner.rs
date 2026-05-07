use anyhow::Result;
use std::process::Command;
use std::time::{Instant, Duration};
use crate::utils::process;
use crate::constants;

pub struct AttemptResult {
    pub success: bool,
    pub timed_out: bool,
    pub stdout: String,
    pub stderr: String,
    pub elapsed: Duration,
}

pub fn run_qemu_attempt(qemu_bin: &str, args: &[String], timeout_sec: u64) -> Result<AttemptResult> {
    let start = Instant::now();
    let mut child = Command::new(qemu_bin)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let (success, timed_out) = match process::wait_child_with_timeout(
        &mut child,
        Duration::from_secs(timeout_sec),
        Duration::from_millis(constants::defaults::run::WAIT_POLL_INTERVAL_MS),
    ) {
        Ok(Some(status)) => (status.success(), false),
        Ok(None) | Err(_) => {
            let _ = child.kill();
            (false, true)
        }
    };

    let stdout = process::read_optional_pipe_to_string(child.stdout.take());
    let stderr = process::read_optional_pipe_to_string(child.stderr.take());

    Ok(AttemptResult { success, timed_out, stdout, stderr, elapsed: start.elapsed() })
}

pub fn format_attempt_log(mode: &str, args: &[String], result: &AttemptResult) -> String {
    format!(
        "=== mode: {mode} ===\nargs: {args}\nsuccess: {success}\ntimeout: {timeout}\nduration_sec: {duration:.3}\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}\n",
        mode = mode,
        args = args.join(" "),
        success = result.success,
        timeout = result.timed_out,
        duration = result.elapsed.as_secs_f64(),
        stdout = result.stdout,
        stderr = result.stderr,
    )
}
