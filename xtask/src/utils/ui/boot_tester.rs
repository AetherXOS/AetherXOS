use std::{fs::File, io::Read, time::{Duration, Instant}};
use anyhow::{Result, Context};
use crate::utils::logging;

pub fn run_boot_test() -> Result<bool> {
    logging::status("TESTER", "Launching Headless QEMU Integration Boot Test...");

    let path = match crate::utils::core::paths::WorkspacePaths::find_boot_image() {
        Some(p) => p,
        None => match crate::utils::core::paths::WorkspacePaths::find_kernel_elf() {
            Some(p) => p,
            None => {
                logging::error("TESTER", "No bootable ISO, image, or kernel binary found. Please build the project first.", &[]);
                return Ok(false);
            }
        }
    };

    let qemu_bin = crate::utils::sys::process::Discovery::qemu_system_x86_64()
        .context("qemu-system-x86_64 not found in PATH")?;

    let log_path = std::path::Path::new("artifacts/boot_test_serial.log");
    let _ = std::fs::remove_file(log_path);
    let _ = File::create(log_path);

    let drive_arg = format!("file={},format=raw", path.display());
    
    // Launch QEMU headlessly and pipe serial output
    let mut child = std::process::Command::new(&qemu_bin)
        .args(&[
            "-m", "1024",
            "-drive", &drive_arg,
            "-serial", "file:artifacts/boot_test_serial.log",
            "-display", "none",
        ])
        .spawn()
        .context("Failed to spawn QEMU headlessly")?;

    logging::info("TESTER", "VM spawned headlessly. Monitoring boot outputs for 5 seconds...", &[]);

    let start = Instant::now();
    let mut success = false;
    let mut crash_detected = false;
    let mut output_lines = Vec::new();

    while start.elapsed() < Duration::from_secs(5) {
        // Check if QEMU exited prematurely (indicates crash or triple fault)
        if let Ok(Some(status)) = child.try_wait() {
            crash_detected = true;
            logging::error("TESTER", &format!("VM crashed or exited prematurely with status: {}", status), &[]);
            break;
        }

        // Read serial log and look for bootstrap signature
        if let Ok(mut file) = File::open(log_path) {
            let mut contents = String::new();
            if file.read_to_string(&mut contents).is_ok() {
                output_lines = contents.lines().map(|s| s.to_string()).collect();
                for line in &output_lines {
                    if line.contains("[INIT]") || line.contains("init") || line.contains("AetherX") || line.contains("ready") || line.contains("Sovereign") {
                        success = true;
                        break;
                    }
                }
            }
        }

        if success {
            break;
        }

        std::thread::sleep(Duration::from_millis(200));
    }

    // Clean up VM
    let _ = child.kill();

    println!("\n========= BOOT TEST REPORT =========");
    if success {
        println!("🟢 VERDICT: PASSED");
        println!("   The kernel booted and initialized successfully!");
    } else if crash_detected {
        println!("🔴 VERDICT: CRASHED");
        println!("   The kernel triple faulted or crashed on startup.");
    } else {
        println!("🟡 VERDICT: TIMEOUT / UNKNOWN");
        println!("   No boot signature detected within 5 seconds.");
    }
    println!("====================================\n");

    if !success {
        // Print the last 10 lines of the serial log
        println!("Last 10 serial output lines:");
        let start_idx = output_lines.len().saturating_sub(10);
        for line in &output_lines[start_idx..] {
            println!("  > {}", line);
        }
        println!();
    }

    Ok(success)
}
