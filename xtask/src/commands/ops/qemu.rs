use anyhow::{Result, bail};
use serde::Serialize;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use crate::builders::qemu::{iso_boot_args, kernel_boot_args, smoke_timeout_sec};
use crate::constants;
use crate::utils::context;
use crate::utils::process;
use crate::utils::report;
use crate::utils::report::JunitSingleCaseReport;

const PANIC_MARKERS: &[&str] = &[
    "PANIC report:",
    "[KERNEL DUMP] panic_count=",
    "kernel panic",
];

const BOOT_SUCCESS_MARKERS: &[&str] = &[
    "limine: Loading executable",
    "smp: Successfully brought up AP",
    "[linux_compat] init complete",
    "[aether_init] early userspace bootstrap",
    "[aether_init] diskfs setup exit status:",
    "[aether_init] pivot-root setup exit status:",
    "[aether_init] apt seed exit status:",
    "installer-seed-complete",
];

#[derive(Serialize)]
struct QemuSmokeSummary {
    mode: String,
    duration_sec: f64,
    timed_out: bool,
    panic_seen: bool,
    boot_marker_seen: bool,
    interrupt_health_ok: bool,
    success: bool,
    pass: bool,
}

#[derive(Debug, Clone, Copy)]
struct InterruptHealth {
    total: u64,
    timer: u64,
    non_timer: u64,
    dropped: u64,
    dispatch_attempted: u64,
    dispatch_handled: u64,
}

/// Run an automated QEMU smoke test with timeout and panic detection.
pub fn smoke_test() -> Result<()> {
    let outdir = context::out_dir();
    let kernel = constants::paths::boot_image_stage_kernel();
    let initramfs = constants::paths::boot_image_stage_initramfs();
    let iso = std::env::var("AETHERCORE_QEMU_SMOKE_ISO")
        .map(PathBuf::from)
        .unwrap_or_else(|_| outdir.join("aethercore.iso"));
    let log_path = constants::paths::qemu_smoke_log();
    let append = constants::defaults::run::KERNEL_APPEND;
    let memory_mb = constants::defaults::run::MEMORY_MB;
    let cores = constants::defaults::run::SMP_CORES;
    let timeout_sec = smoke_timeout_sec();

    let qemu_bin = process::find_qemu_system_x86_64()
        .ok_or_else(|| anyhow::anyhow!("qemu-system-x86_64 not found in PATH or Program Files"))?;
    println!("[qemu::smoke] Binary: {}", qemu_bin);
    println!("[qemu::smoke] Kernel: {}", kernel.display());
    println!("[qemu::smoke] ISO fallback: {}", iso.display());
    println!("[qemu::smoke] Timeout: {}s", timeout_sec);

    let direct_args = kernel_boot_args(
        memory_mb,
        cores,
        &kernel.to_string_lossy(),
        &initramfs.to_string_lossy(),
        append,
        true,
    );
    let direct = run_qemu_attempt(&qemu_bin, &direct_args, timeout_sec)?;

    let mut combined_log = format_attempt_log("direct-kernel", &direct_args, &direct);
    let mut final_mode = "direct-kernel";
    let mut final_result = direct;

    let pvh_elf_note_error = final_result
        .stderr
        .contains("Error loading uncompressed kernel without PVH ELF Note");

    if pvh_elf_note_error && iso.exists() {
        println!("[qemu::smoke] Direct kernel boot rejected by QEMU (PVH note); retrying with ISO");
        let iso_args = iso_boot_args(memory_mb, cores, &iso.to_string_lossy(), true);
        let iso_result = run_qemu_attempt(&qemu_bin, &iso_args, timeout_sec)?;
        combined_log.push_str("\n\n");
        combined_log.push_str(&format_attempt_log("iso-fallback", &iso_args, &iso_result));
        final_mode = "iso-fallback";
        final_result = iso_result;
    }

    // Write text log with both stdout and stderr so failures are diagnosable.
    report::write_text_report(&log_path, &combined_log)?;

    let stream = format!("{}\n{}", final_result.stdout, final_result.stderr);
    let panic_seen = PANIC_MARKERS.iter().any(|m| stream.contains(m));
    let boot_marker_seen = BOOT_SUCCESS_MARKERS.iter().any(|m| stream.contains(m));
    let interrupt_health = detect_interrupt_health(&stream);
    let interrupt_health_ok = interrupt_health
        .map(validate_interrupt_health)
        .unwrap_or(false);
    let pass = !panic_seen
        && interrupt_health_ok
        && (final_result.success || boot_marker_seen);

    println!("[qemu::smoke] Mode: {}", final_mode);
    println!(
        "[qemu::smoke] Duration: {:.1}s",
        final_result.elapsed.as_secs_f64()
    );
    println!("[qemu::smoke] Timeout: {}", final_result.timed_out);
    println!("[qemu::smoke] Panic detected: {}", panic_seen);
    println!("[qemu::smoke] Boot marker detected: {}", boot_marker_seen);
    println!("[qemu::smoke] Interrupt health asserted: {}", interrupt_health_ok);
    println!("[qemu::smoke] Exit success: {}", final_result.success);
    println!("[qemu::smoke] Text Log: {}", log_path.display());

    // Export Enterprise-Grade CI/CD report set (JUnit + JSON summary)
    let junit_path = constants::paths::qemu_smoke_junit();
    let failure_message = format!(
        "Panic Seen: {} | Timed Out: {} | Interrupt Health: {}",
        panic_seen,
        final_result.timed_out,
        interrupt_health_ok
    );
    let junit = JunitSingleCaseReport {
        suite_name: "QemuSmokeTest",
        case_name: "Aether_X_OS_Limine_Boot",
        class_name: "kernel.boot",
        duration_secs: final_result.elapsed.as_secs_f64(),
        passed: pass,
        failure_message: Some(&failure_message),
        stdout: &final_result.stdout,
        stderr: &final_result.stderr,
    };
    report::write_junit_single_case(&junit_path, &junit)?;

    let summary_path = constants::paths::qemu_smoke_json();
    let summary = QemuSmokeSummary {
        mode: final_mode.to_string(),
        duration_sec: final_result.elapsed.as_secs_f64(),
        timed_out: final_result.timed_out,
        panic_seen,
        boot_marker_seen,
        interrupt_health_ok,
        success: final_result.success,
        pass,
    };
    report::write_json_report(&summary_path, &summary)?;

    if !pass {
        bail!(
            "QEMU smoke test failed (mode={}, timeout={}, panic={}, success={}, boot_marker={}, interrupt_health={}); log={}",
            final_mode,
            final_result.timed_out,
            panic_seen,
            final_result.success,
            boot_marker_seen,
            interrupt_health_ok,
            log_path.display()
        );
    }

    println!("[qemu::smoke] PASS");
    Ok(())
}

fn detect_interrupt_health(stream: &str) -> Option<InterruptHealth> {
    if let Some(line) = stream
        .lines()
        .rev()
        .find(|line| line.contains("x86_64 irq stats:"))
    {
        let total = extract_u64_kv(line, "total")?;
        let timer = extract_u64_kv(line, "timer")?;
        let non_timer = extract_u64_kv(line, "non_timer")?;
        let dropped = extract_u64_kv(line, "dropped")?;
        let dispatch_attempted = extract_u64_kv(line, "dispatch_attempted")?;
        let dispatch_handled = extract_u64_kv(line, "dispatch_handled")?;
        return Some(InterruptHealth {
            total,
            timer,
            non_timer,
            dropped,
            dispatch_attempted,
            dispatch_handled,
        });
    }

    if let Some(line) = stream
        .lines()
        .rev()
        .find(|line| line.contains("AArch64 exception stats:"))
    {
        let total = extract_u64_kv(line, "irq_total")?;
        let timer = extract_u64_kv(line, "timer_irq")?;
        let non_timer = total.saturating_sub(timer);
        let dispatch_attempted = total;
        let dispatch_handled = total;
        let dropped = 0;
        return Some(InterruptHealth {
            total,
            timer,
            non_timer,
            dropped,
            dispatch_attempted,
            dispatch_handled,
        });
    }

    None
}

fn validate_interrupt_health(health: InterruptHealth) -> bool {
    if health.total < health.timer.saturating_add(health.non_timer) {
        return false;
    }
    if health.dispatch_attempted < health.dispatch_handled {
        return false;
    }
    if health.dropped > health.total {
        return false;
    }
    true
}

fn extract_u64_kv(line: &str, key: &str) -> Option<u64> {
    let needle = format!("{}=", key);
    let start = line.find(&needle)? + needle.len();
    let rest = &line[start..];
    let end = rest.find(' ').unwrap_or(rest.len());
    rest[..end].trim_end_matches(',').parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
    use super::{detect_interrupt_health, validate_interrupt_health, InterruptHealth};

    #[test]
    fn detects_x86_interrupt_health_from_runtime_line() {
        let log = "x86_64 irq stats: total=15 timer=10 non_timer=5 dropped=1 dispatch_attempted=14 dispatch_handled=13 timer_dropped=0";
        let health = detect_interrupt_health(log).expect("x86 health should be parsed");

        assert_eq!(health.total, 15);
        assert_eq!(health.timer, 10);
        assert_eq!(health.non_timer, 5);
        assert_eq!(health.dropped, 1);
        assert_eq!(health.dispatch_attempted, 14);
        assert_eq!(health.dispatch_handled, 13);
        assert!(validate_interrupt_health(health));
    }

    #[test]
    fn detects_aarch64_interrupt_health_from_exception_stats_line() {
        let log = "AArch64 exception stats: sync=0 fiq=0 serror=0 user_abort=0 kernel_abort=0 user_fatal_sync=0 user_fatal_async=0 kernel_fatal_async=0 irq_total=22 irq_spurious=1 irq_storm_windows=0 irq_suppressed=0 timer_irq=11 timer_jitter=0 irq_track_limit=256 irq_hot=30 irq_hot_total=11 irq_hot_storms=0 irq_hot_suppressed=0 gic_pmr=255";
        let health = detect_interrupt_health(log).expect("aarch64 health should be parsed");

        assert_eq!(health.total, 22);
        assert_eq!(health.timer, 11);
        assert_eq!(health.non_timer, 11);
        assert_eq!(health.dispatch_attempted, 22);
        assert_eq!(health.dispatch_handled, 22);
        assert!(validate_interrupt_health(health));
    }

    #[test]
    fn rejects_invalid_interrupt_health_relations() {
        let invalid = InterruptHealth {
            total: 5,
            timer: 4,
            non_timer: 3,
            dropped: 0,
            dispatch_attempted: 5,
            dispatch_handled: 6,
        };
        assert!(!validate_interrupt_health(invalid));
    }
}

struct AttemptResult {
    success: bool,
    timed_out: bool,
    stdout: String,
    stderr: String,
    elapsed: std::time::Duration,
}

fn run_qemu_attempt(qemu_bin: &str, args: &[String], timeout_sec: u64) -> Result<AttemptResult> {
    let start = Instant::now();
    let mut child = Command::new(qemu_bin)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let (success, timed_out) = match process::wait_child_with_timeout(
        &mut child,
        std::time::Duration::from_secs(timeout_sec),
        std::time::Duration::from_millis(constants::defaults::run::WAIT_POLL_INTERVAL_MS),
    ) {
        Ok(Some(status)) => (status.success(), false),
        Ok(None) | Err(_) => {
            let _ = child.kill();
            (false, true)
        }
    };

    let stdout = process::read_optional_pipe_to_string(child.stdout.take());
    let stderr = process::read_optional_pipe_to_string(child.stderr.take());

    Ok(AttemptResult {
        success,
        timed_out,
        stdout,
        stderr,
        elapsed: start.elapsed(),
    })
}

fn format_attempt_log(mode: &str, args: &[String], result: &AttemptResult) -> String {
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

/// Launch an interactive QEMU session with display.
pub fn interactive() -> Result<()> {
    let kernel = constants::paths::boot_image_stage_kernel();
    let initramfs = constants::paths::boot_image_stage_initramfs();
    let qemu_bin = process::find_qemu_system_x86_64()
        .ok_or_else(|| anyhow::anyhow!("qemu-system-x86_64 not found in PATH or Program Files"))?;

    println!("[qemu::live] Launching interactive session");
    let mut args = kernel_boot_args(
        constants::defaults::run::MEMORY_MB,
        constants::defaults::run::SMP_CORES,
        &kernel.to_string_lossy(),
        &initramfs.to_string_lossy(),
        constants::defaults::run::KERNEL_APPEND,
        false,
    );
    args.splice(4..4, ["-serial".to_string(), "stdio".to_string()]);

    let status = Command::new(&qemu_bin).args(args).status()?;

    if !status.success() {
        bail!("QEMU exited with code: {}", status.code().unwrap_or(-1));
    }
    Ok(())
}

/// Launches QEMU paused (-S -s) to await a GDB localhost:1234 attachment.
pub fn debug_session() -> Result<()> {
    let kernel = constants::paths::boot_image_stage_kernel();
    let initramfs = constants::paths::boot_image_stage_initramfs();
    let qemu_bin = process::find_qemu_system_x86_64()
        .ok_or_else(|| anyhow::anyhow!("qemu-system-x86_64 not found in PATH or Program Files"))?;

    println!("[qemu::debug] QEMU initialized with paused CPU states.");
    println!("[qemu::debug] Exposing local debugger port... Run 'target remote :1234' on GDB");
    let mut args = kernel_boot_args(
        constants::defaults::run::MEMORY_MB,
        constants::defaults::run::SMP_CORES,
        &kernel.to_string_lossy(),
        &initramfs.to_string_lossy(),
        constants::defaults::run::KERNEL_APPEND,
        false,
    );
    args.extend(["-S".to_string(), "-s".to_string()]);
    let status = Command::new(&qemu_bin).args(args).status()?;

    if !status.success() {
        bail!(
            "QEMU debug overlay exited with error code: {}",
            status.code().unwrap_or(-1)
        );
    }
    Ok(())
}
