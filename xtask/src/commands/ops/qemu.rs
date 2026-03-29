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

/// Run an automated QEMU smoke test with timeout and panic detection.
pub fn smoke_test() -> Result<()> {
    let kernel = paths::resolve("artifacts/boot_image/stage/boot/hypercore.elf");
    let initramfs = paths::resolve("artifacts/boot_image/stage/boot/initramfs.cpio.gz");
    let log_path = paths::resolve("artifacts/boot_image/qemu_smoke.log");
    let append = "console=ttyS0 loglevel=7";
    let memory_mb = 512;
    let cores = 2;
    let timeout_sec = 20;

    let qemu_bin = find_qemu()?;
    println!("[qemu::smoke] Binary: {}", qemu_bin);
    println!("[qemu::smoke] Kernel: {}", kernel.display());
    println!("[qemu::smoke] Timeout: {}s", timeout_sec);

    let start = Instant::now();
    let mut child = Command::new(&qemu_bin)
        .args([
            "-nographic",
            "-m", &memory_mb.to_string(),
            "-smp", &cores.to_string(),
            "-kernel", &kernel.to_string_lossy(),
            "-initrd", &initramfs.to_string_lossy(),
            "-append", append,
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let output = match child.wait_timeout(std::time::Duration::from_secs(timeout_sec)) {
        Ok(Some(status)) => {
            let out = child.stdout.take().map(|mut s| {
                let mut buf = String::new();
                std::io::Read::read_to_string(&mut s, &mut buf).ok();
                buf
            }).unwrap_or_default();
            (status.success(), false, out)
        }
        Ok(None) | Err(_) => {
            // Timeout or error - kill the process
            let _ = child.kill();
            let out = child.stdout.take().map(|mut s| {
                let mut buf = String::new();
                std::io::Read::read_to_string(&mut s, &mut buf).ok();
                buf
            }).unwrap_or_default();
            (false, true, out)
        }
    };

    let elapsed = start.elapsed();
    let (success, timed_out, stdout) = output;

    // Write log
    paths::ensure_dir(log_path.parent().unwrap())?;
    std::fs::write(&log_path, &stdout)?;

    // Check for panic markers
    let panic_seen = PANIC_MARKERS.iter().any(|m| stdout.contains(m));

    println!("[qemu::smoke] Duration: {:.1}s", elapsed.as_secs_f64());
    println!("[qemu::smoke] Timeout: {}", timed_out);
    println!("[qemu::smoke] Panic detected: {}", panic_seen);
    println!("[qemu::smoke] Log: {}", log_path.display());

    if timed_out || panic_seen || !success {
        bail!(
            "QEMU smoke test failed (timeout={}, panic={}, success={}); log={}",
            timed_out, panic_seen, success, log_path.display()
        );
    }

    println!("[qemu::smoke] PASS");
    Ok(())
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
