use anyhow::{Result, bail};
use std::path::PathBuf;
use std::process::Command;

use crate::builders::qemu::{iso_boot_args, kernel_boot_args, smoke_timeout_sec};
use crate::constants;
use crate::utils::{context, process, report};
use crate::utils::report::JunitSingleCaseReport;

pub mod smoke;
pub mod runner;

pub fn smoke_test() -> Result<()> {
    let outdir = context::out_dir();
    let kernel = constants::paths::boot_image_stage_kernel();
    let initramfs = constants::paths::boot_image_stage_initramfs();
    let iso = std::env::var("AETHERCORE_QEMU_SMOKE_ISO")
        .map(PathBuf::from).unwrap_or_else(|_| outdir.join("aethercore.iso"));
    let log_path = constants::paths::qemu_smoke_log();
    let timeout_sec = smoke_timeout_sec();

    let qemu_bin = process::find_qemu_system_x86_64()
        .ok_or_else(|| anyhow::anyhow!("qemu-system-x86_64 not found"))?;

    let direct_args = kernel_boot_args(
        constants::defaults::run::MEMORY_MB,
        constants::defaults::run::SMP_CORES,
        &kernel.to_string_lossy(),
        &initramfs.to_string_lossy(),
        constants::defaults::run::KERNEL_APPEND,
        true,
    );
    let direct = runner::run_qemu_attempt(&qemu_bin, &direct_args, timeout_sec)?;
    
    let mut combined_log = runner::format_attempt_log("direct-kernel", &direct_args, &direct);
    let mut final_mode = "direct-kernel";
    let mut final_result = direct;

    if final_result.stderr.contains("Error loading uncompressed kernel") && iso.exists() {
        let iso_args = iso_boot_args(constants::defaults::run::MEMORY_MB, constants::defaults::run::SMP_CORES, &iso.to_string_lossy(), true);
        let iso_result = runner::run_qemu_attempt(&qemu_bin, &iso_args, timeout_sec)?;
        combined_log.push_str("\n\n");
        combined_log.push_str(&runner::format_attempt_log("iso-fallback", &iso_args, &iso_result));
        final_mode = "iso-fallback";
        final_result = iso_result;
    }

    report::write_text_report(&log_path, &combined_log)?;

    let stream = format!("{}\n{}", final_result.stdout, final_result.stderr);
    let panic_seen = smoke::PANIC_MARKERS.iter().any(|m| stream.contains(m));
    let boot_marker_seen = smoke::BOOT_SUCCESS_MARKERS.iter().any(|m| stream.contains(m));
    let pass = !panic_seen && (final_result.success || boot_marker_seen);

    // Write reports
    let failure_message = format!(
        "Mode: {} | Panic Seen: {} | Timed Out: {}",
        final_mode, panic_seen, final_result.timed_out
    );
    let junit = JunitSingleCaseReport {
        suite_name: "QemuSmokeTest",
        case_name: "Aether_X_OS_Boot",
        class_name: "kernel.boot",
        duration_secs: final_result.elapsed.as_secs_f64(),
        passed: pass,
        failure_message: Some(&failure_message),
        stdout: &final_result.stdout,
        stderr: &final_result.stderr,
    };
    report::write_junit_single_case(&constants::paths::qemu_smoke_junit(), &junit)?;

    if !pass { bail!("QEMU smoke test failed; log={}", log_path.display()); }
    Ok(())
}

pub fn interactive() -> Result<()> {
    let qemu_bin = process::find_qemu_system_x86_64().ok_or_else(|| anyhow::anyhow!("qemu not found"))?;
    let args = kernel_boot_args(
        constants::defaults::run::MEMORY_MB,
        constants::defaults::run::SMP_CORES,
        &constants::paths::boot_image_stage_kernel().to_string_lossy(),
        &constants::paths::boot_image_stage_initramfs().to_string_lossy(),
        constants::defaults::run::KERNEL_APPEND,
        false,
    );
    let status = Command::new(&qemu_bin).args(args).status()?;
    if !status.success() { bail!("QEMU exited with code: {}", status.code().unwrap_or(-1)); }
    Ok(())
}

pub fn debug_session() -> Result<()> {
    let qemu_bin = process::find_qemu_system_x86_64().ok_or_else(|| anyhow::anyhow!("qemu not found"))?;
    let mut args = kernel_boot_args(
        constants::defaults::run::MEMORY_MB,
        constants::defaults::run::SMP_CORES,
        &constants::paths::boot_image_stage_kernel().to_string_lossy(),
        &constants::paths::boot_image_stage_initramfs().to_string_lossy(),
        constants::defaults::run::KERNEL_APPEND,
        false,
    );
    args.extend(["-S".to_string(), "-s".to_string()]);
    let status = Command::new(&qemu_bin).args(args).status()?;
    if !status.success() { bail!("QEMU debug session failed"); }
    Ok(())
}
