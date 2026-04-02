use anyhow::{Result, bail};
use serde::Serialize;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use crate::builders::qemu::{iso_boot_args, kernel_boot_args, smoke_timeout_sec};
use crate::constants;
use crate::utils::paths;
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
    success: bool,
    pass: bool,
}

/// Run an automated QEMU smoke test with timeout and panic detection.
pub fn smoke_test() -> Result<()> {
    let outdir = std::env::var("XTASK_OUTDIR").unwrap_or_else(|_| constants::paths::ARTIFACTS_DIR.to_string());
    let kernel = constants::paths::boot_image_stage_kernel();
    let initramfs = constants::paths::boot_image_stage_initramfs();
    let iso = std::env::var("AETHERCORE_QEMU_SMOKE_ISO")
        .map(PathBuf::from)
        .unwrap_or_else(|_| paths::resolve(&format!("{}/aethercore.iso", outdir)));
    let log_path = constants::paths::qemu_smoke_log();
    let append = constants::defaults::run::KERNEL_APPEND;
    let memory_mb = constants::defaults::run::MEMORY_MB;
    let cores = constants::defaults::run::SMP_CORES;
    let timeout_sec = smoke_timeout_sec();

    let qemu_bin = find_qemu()?;
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
        println!(
            "[qemu::smoke] Direct kernel boot rejected by QEMU (PVH note); retrying with ISO"
        );
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
    let pass = !panic_seen && (final_result.success || boot_marker_seen);

    println!("[qemu::smoke] Mode: {}", final_mode);
    println!("[qemu::smoke] Duration: {:.1}s", final_result.elapsed.as_secs_f64());
    println!("[qemu::smoke] Timeout: {}", final_result.timed_out);
    println!("[qemu::smoke] Panic detected: {}", panic_seen);
    println!("[qemu::smoke] Boot marker detected: {}", boot_marker_seen);
    println!("[qemu::smoke] Exit success: {}", final_result.success);
    println!("[qemu::smoke] Text Log: {}", log_path.display());

    // Export Enterprise-Grade CI/CD report set (JUnit + JSON summary)
    let junit_path = constants::paths::qemu_smoke_junit();
    let failure_message = format!("Panic Seen: {} | Timed Out: {}", panic_seen, final_result.timed_out);
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
        success: final_result.success,
        pass,
    };
    report::write_json_report(&summary_path, &summary)?;

    if !pass {
        bail!(
            "QEMU smoke test failed (mode={}, timeout={}, panic={}, success={}, boot_marker={}); log={}",
            final_mode,
            final_result.timed_out,
            panic_seen,
            final_result.success,
            boot_marker_seen,
            log_path.display()
        );
    }

    println!("[qemu::smoke] PASS");
    Ok(())
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

    let (success, timed_out) = match child.wait_timeout(std::time::Duration::from_secs(timeout_sec)) {
        Ok(Some(status)) => (status.success(), false),
        Ok(None) | Err(_) => {
            let _ = child.kill();
            (false, true)
        }
    };

    let stdout = child.stdout.take().map(|mut s| {
        let mut buf = String::new();
        std::io::Read::read_to_string(&mut s, &mut buf).ok();
        buf
    }).unwrap_or_default();
    let stderr = child.stderr.take().map(|mut s| {
        let mut buf = String::new();
        std::io::Read::read_to_string(&mut s, &mut buf).ok();
        buf
    }).unwrap_or_default();

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
    let qemu_bin = find_qemu()?;

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

    let status = Command::new(&qemu_bin)
        .args(args)
        .status()?;

    if !status.success() {
        bail!("QEMU exited with code: {}", status.code().unwrap_or(-1));
    }
    Ok(())
}

/// Locate the qemu-system-x86_64 binary on the system.
fn find_qemu() -> Result<String> {
    if let Some(binary) = process::first_available_binary(&[constants::tools::QEMU_X86_64, constants::tools::QEMU_X86_64_EXE]) {
        return Ok(binary.to_string());
    }
    // Windows fallback: check Program Files
    let pf = std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".to_string());
    let candidate = PathBuf::from(pf).join("qemu").join("qemu-system-x86_64.exe");
    if candidate.exists() {
        return Ok(candidate.to_string_lossy().to_string());
    }
    bail!("qemu-system-x86_64 not found in PATH or Program Files")
}

/// Extension trait for wait_timeout on child processes.
trait WaitTimeout {
    fn wait_timeout(&mut self, duration: std::time::Duration) -> std::io::Result<Option<std::process::ExitStatus>>;
}

impl WaitTimeout for std::process::Child {
    fn wait_timeout(&mut self, duration: std::time::Duration) -> std::io::Result<Option<std::process::ExitStatus>> {
        let start = Instant::now();
        loop {
            match self.try_wait()? {
                Some(status) => return Ok(Some(status)),
                None => {
                    if start.elapsed() >= duration {
                        return Ok(None);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(
                        constants::defaults::run::WAIT_POLL_INTERVAL_MS,
                    ));
                }
            }
        }
    }
}

/// Launches QEMU paused (-S -s) to await a GDB localhost:1234 attachment.
pub fn debug_session() -> Result<()> {
    let kernel = constants::paths::boot_image_stage_kernel();
    let initramfs = constants::paths::boot_image_stage_initramfs();
    let qemu_bin = find_qemu()?;

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
    let status = Command::new(&qemu_bin)
        .args(args)
        .status()?;

    if !status.success() {
        bail!("QEMU debug overlay exited with error code: {}", status.code().unwrap_or(-1));
    }
    Ok(())
}

