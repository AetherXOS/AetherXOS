use anyhow::{Result, bail};
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use crate::utils::paths;
use crate::utils::process;

const PANIC_MARKERS: &[&str] = &[
    "PANIC report:",
    "[KERNEL DUMP] panic_count=",
    "kernel panic",
];

const BOOT_SUCCESS_MARKERS: &[&str] = &[
    "limine: Loading executable",
    "smp: Successfully brought up AP",
    "[linux_compat] init complete",
];

/// Run an automated QEMU smoke test with timeout and panic detection.
pub fn smoke_test() -> Result<()> {
    let kernel = paths::resolve("artifacts/boot_image/stage/boot/hypercore.elf");
    let initramfs = paths::resolve("artifacts/boot_image/stage/boot/initramfs.cpio.gz");
    let iso = paths::resolve("artifacts/boot_image/hypercore.iso");
    let log_path = paths::resolve("artifacts/boot_image/qemu_smoke.log");
    let append = "console=ttyS0 loglevel=7";
    let memory_mb = 512;
    let cores = 2;
    let timeout_sec = 20;

    let qemu_bin = find_qemu()?;
    println!("[qemu::smoke] Binary: {}", qemu_bin);
    println!("[qemu::smoke] Kernel: {}", kernel.display());
    println!("[qemu::smoke] Timeout: {}s", timeout_sec);

    let direct_args = vec![
        "-nographic".to_string(),
        "-m".to_string(),
        memory_mb.to_string(),
        "-smp".to_string(),
        cores.to_string(),
        "-kernel".to_string(),
        kernel.to_string_lossy().to_string(),
        "-initrd".to_string(),
        initramfs.to_string_lossy().to_string(),
        "-append".to_string(),
        append.to_string(),
    ];
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
        let iso_args = vec![
            "-nographic".to_string(),
            "-m".to_string(),
            memory_mb.to_string(),
            "-smp".to_string(),
            cores.to_string(),
            "-cdrom".to_string(),
            iso.to_string_lossy().to_string(),
            "-boot".to_string(),
            "d".to_string(),
        ];
        let iso_result = run_qemu_attempt(&qemu_bin, &iso_args, timeout_sec)?;
        combined_log.push_str("\n\n");
        combined_log.push_str(&format_attempt_log("iso-fallback", &iso_args, &iso_result));
        final_mode = "iso-fallback";
        final_result = iso_result;
    }

    // Write log with both stdout and stderr so failures are diagnosable.
    paths::ensure_dir(log_path.parent().unwrap())?;
    std::fs::write(&log_path, &combined_log)?;

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
    println!("[qemu::smoke] Log: {}", log_path.display());

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
    let kernel = paths::resolve("artifacts/boot_image/stage/boot/hypercore.elf");
    let initramfs = paths::resolve("artifacts/boot_image/stage/boot/initramfs.cpio.gz");
    let qemu_bin = find_qemu()?;

    println!("[qemu::live] Launching interactive session");
    let status = Command::new(&qemu_bin)
        .args([
            "-m", "512",
            "-smp", "2",
            "-serial", "stdio",
            "-kernel", &kernel.to_string_lossy(),
            "-initrd", &initramfs.to_string_lossy(),
            "-append", "console=ttyS0 loglevel=7",
        ])
        .status()?;

    if !status.success() {
        bail!("QEMU exited with code: {}", status.code().unwrap_or(-1));
    }
    Ok(())
}

/// Locate the qemu-system-x86_64 binary on the system.
fn find_qemu() -> Result<String> {
    if process::which("qemu-system-x86_64") {
        return Ok("qemu-system-x86_64".to_string());
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
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }
    }
}
