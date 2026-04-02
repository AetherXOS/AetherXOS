use anyhow::{bail, Result};
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use crate::constants::{paths as const_paths, tools};
use crate::utils::{context, logging, paths, process};

const PANIC_MARKERS: &[&str] = &[
    "PANIC report:",
    "[KERNEL DUMP] panic_count=",
    "kernel panic",
];

const BOOT_SUCCESS_MARKERS: &[&str] = &[
    "limine: Loading executable",
    "smp: Successfully brought up AP",
    "[linux_compat] init complete",
    "[hyper_init] early userspace bootstrap",
    "[hyper_init] diskfs setup exit status:",
    "[hyper_init] pivot-root setup exit status:",
    "[hyper_init] apt seed exit status:",
    "installer-seed-complete",
];

/// Run an automated QEMU smoke test with timeout and panic detection.
pub fn smoke_test() -> Result<()> {
    let kernel = paths::resolve(&format!("{}/boot/hypercore.elf", const_paths::ARTIFACTS_BOOT_IMAGE_STAGE));
    let initramfs = paths::resolve(const_paths::BOOT_INITRAMFS_OUT);
    let iso = std::env::var("HYPERCORE_QEMU_SMOKE_ISO")
        .map(PathBuf::from)
        .unwrap_or_else(|_| context::out_dir().join("hypercore.iso"));
    let log_path = paths::resolve(&format!("{}/qemu_smoke.log", const_paths::ARTIFACTS_BOOT_IMAGE));
    let append = "console=ttyS0 loglevel=7";
    let memory_mb = 512;
    let cores = 2;
    let timeout_sec = std::env::var("HYPERCORE_QEMU_SMOKE_TIMEOUT_SEC")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(20);

    let qemu_bin = find_qemu()?;
    logging::info(
        "qemu",
        "starting smoke test",
        &[
            ("binary", &qemu_bin),
            ("kernel", &kernel.to_string_lossy()),
            ("iso", &iso.to_string_lossy()),
            ("timeout_sec", &timeout_sec.to_string()),
        ],
    );

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
        logging::warn(
            "qemu",
            "direct kernel boot rejected by QEMU, retrying with ISO",
            &[],
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

    logging::info(
        "qemu",
        "smoke test completed",
        &[
            ("mode", final_mode),
            (
                "duration_sec",
                &format!("{:.1}", final_result.elapsed.as_secs_f64()),
            ),
            ("timed_out", &final_result.timed_out.to_string()),
            ("panic_seen", &panic_seen.to_string()),
            ("boot_marker_seen", &boot_marker_seen.to_string()),
            ("exit_success", &final_result.success.to_string()),
            ("log", &log_path.to_string_lossy()),
        ],
    );

    // Export Enterprise-Grade CI/CD XML structured reports
    let junit_path = paths::resolve("artifacts/qemu_smoke_junit.xml");
    let failure_tag = if pass {
        String::new()
    } else {
        format!("<failure message=\"Boot assertion failed\"><![CDATA[Panic Seen: {} | Timed Out: {}]]></failure>", panic_seen, final_result.timed_out)
    };

    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<testsuites tests="1" failures="{failures}" errors="0" time="{time:.3}">
    <testsuite name="QemuSmokeTest" tests="1" failures="{failures}" errors="0" time="{time:.3}">
        <testcase name="Hypercore_Limine_Boot" classname="kernel.boot" time="{time:.3}">
            {failure_tag}
            <system-out><![CDATA[{stdout}]]></system-out>
            <system-err><![CDATA[{stderr}]]></system-err>
        </testcase>
    </testsuite>
</testsuites>"#,
        failures = if pass { 0 } else { 1 },
        time = final_result.elapsed.as_secs_f64(),
        failure_tag = failure_tag,
        stdout = final_result.stdout.replace("]]>", "]]>]]&gt;<![CDATA["),
        stderr = final_result.stderr.replace("]]>", "]]>]]&gt;<![CDATA["),
    );

    if std::fs::write(&junit_path, xml).is_ok() {
        logging::ready(
            "qemu",
            "junit report exported",
            &junit_path.to_string_lossy(),
        );
    }

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

    logging::ready("qemu", "smoke test passed", &log_path.to_string_lossy());
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

    let (success, timed_out) = match child.wait_timeout(std::time::Duration::from_secs(timeout_sec))
    {
        Ok(Some(status)) => (status.success(), false),
        Ok(None) | Err(_) => {
            let _ = child.kill();
            (false, true)
        }
    };

    let stdout = child
        .stdout
        .take()
        .map(|mut s| {
            let mut buf = String::new();
            std::io::Read::read_to_string(&mut s, &mut buf).ok();
            buf
        })
        .unwrap_or_default();
    let stderr = child
        .stderr
        .take()
        .map(|mut s| {
            let mut buf = String::new();
            std::io::Read::read_to_string(&mut s, &mut buf).ok();
            buf
        })
        .unwrap_or_default();

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

    logging::info("qemu", "launching interactive session", &[]);
    let status = Command::new(&qemu_bin)
        .args([
            "-m",
            "512",
            "-smp",
            "2",
            "-serial",
            "stdio",
            "-kernel",
            &kernel.to_string_lossy(),
            "-initrd",
            &initramfs.to_string_lossy(),
            "-append",
            "console=ttyS0 loglevel=7",
        ])
        .status()?;

    if !status.success() {
        bail!("QEMU exited with code: {}", status.code().unwrap_or(-1));
    }
    Ok(())
}

/// Locate the qemu-system-x86_64 binary on the system.
fn find_qemu() -> Result<String> {
    if process::which(tools::QEMU_X86_64) {
        return Ok(tools::QEMU_X86_64.to_string());
    }
    let pf = std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".to_string());
    let candidate = PathBuf::from(pf)
        .join("qemu")
        .join(tools::QEMU_X86_64_EXE);
    if candidate.exists() {
        return Ok(candidate.to_string_lossy().to_string());
    }
    bail!("{} not found in PATH or Program Files", tools::QEMU_X86_64)
}

/// Extension trait for wait_timeout on child processes.
trait WaitTimeout {
    fn wait_timeout(
        &mut self,
        duration: std::time::Duration,
    ) -> std::io::Result<Option<std::process::ExitStatus>>;
}

impl WaitTimeout for std::process::Child {
    fn wait_timeout(
        &mut self,
        duration: std::time::Duration,
    ) -> std::io::Result<Option<std::process::ExitStatus>> {
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

/// Launches QEMU paused (-S -s) to await a GDB localhost:1234 attachment.
pub fn debug_session() -> Result<()> {
    let kernel = paths::resolve(&format!("{}/boot/hypercore.elf", const_paths::ARTIFACTS_BOOT_IMAGE_STAGE));
    let initramfs = paths::resolve(const_paths::BOOT_INITRAMFS_OUT);
    let qemu_bin = find_qemu()?;

    logging::info("ops::qemu", "QEMU initialized with paused CPU states", &[]);
    logging::ready(
        "ops::qemu",
        "Exposing local debugger port",
        &[("gdb_command", "target remote :1234")],
    );

    let status = Command::new(&qemu_bin)
        .args([
            "-m",
            "512",
            "-smp",
            "2",
            "-kernel",
            &kernel.to_string_lossy(),
            "-initrd",
            &initramfs.to_string_lossy(),
            "-append",
            "console=ttyS0 loglevel=7",
            "-S",
            "-s",
        ])
        .status()?;

    if !status.success() {
        bail!(
            "QEMU debug overlay exited with error code: {}",
            status.code().unwrap_or(-1)
        );
    }
    Ok(())
}
