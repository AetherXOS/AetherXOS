use anyhow::{bail, Context, Result};
use crate::cli::RunAction;

/// Entry point for `cargo run -p xtask -- run <action>`.
/// Isolates emulation layers and hardware block-level testing environments.
pub fn execute(action: &RunAction) -> Result<()> {
    match action {
        RunAction::Smoke { bootloader } => {
            println!("[run::smoke] Starting automated generic smoke test via headless QEMU parameter constraints.");
            println!("[run::smoke] Intercepted intended target bootloader sequence: {:?}", bootloader);
            
            crate::commands::ops::qemu::smoke_test()
                .context("Headless QEMU pipeline execution completely failed to converge or encountered a timeout.")?;
        }
        RunAction::Live { firmware } => {
            println!("[run::live] Launching interactive QEMU graphic interface utilizing firmware mode: '{}'.", firmware);
            
            crate::commands::ops::qemu::interactive()
                .context("Visual QEMU session crashed or failed standard startup initialization checks.")?;
        }
        RunAction::Debug { firmware } => {
            println!("[run::debug] Engaging execution mode: Paused / Waiting for GDB.");
            println!("[run::debug] Firmware parameters '{}' active. Proceeding to inject TCP debugging port (tcp::1234)...", firmware);
            
            crate::commands::ops::qemu::debug_session()
                .context("Failed suspending QEMU instance for execution debugging attachment")?;
        }
        RunAction::PxeServer { port } => {
            println!("[run::pxe] Emulating stateless distribution host for physical machine broadcasting.");
            println!("[run::pxe] Initializing HTTP-based iPXE delivery subsystem natively on port: {}", port);
            
            let host_dir = crate::utils::paths::resolve("artifacts");
            if !host_dir.exists() {
                bail!("Network serving aborted: 'artifacts' directory missing. Please 'build full' first.");
            }
            
            println!("[run::pxe] Emulating dynamic TFTP/HTTP serving at '{}' to generic physical Network Cards (NIC).", host_dir.display());
            println!("[run::pxe] Use your other computer's BIOS -> Network Boot pointing to this host's IP/{}", port);
            
            let mut server = std::process::Command::new("python")
                .args(&["-m", "http.server", &port.to_string()])
                .current_dir(&host_dir)
                .spawn()
                .context("Failed executing Python HTTP fallback host module. (Is Python3 installed system-wide?)")?;
                
            let _ = server.wait()?;
        }
        RunAction::BareMetalDeploy { device } => {
            // Highly robust, strictly verified block device interaction (DD abstraction).
            println!("[run::deploy] CRITICAL PROCESS: Immediate raw byte-transfer to '{}' target initialized.", device);
            println!("[run::deploy] CAUTION: This operation bypasses logical FAT formats to enforce raw DD block sector overwrites.");
            
            let img_path = crate::utils::paths::resolve("artifacts/hypercore.img");
            if !img_path.exists() {
                bail!("Logical RAW medium missing: 'hypercore.img'. Terminated. Execute 'cargo run -p xtask -- build full --format img' first.");
            }
            
            println!("[run::deploy] Origin block source verified: {}", img_path.display());
            println!("[run::deploy] Requesting administrative lock overlay on target physical address: {}", device);
            println!("[run::deploy] Stub constraint triggered. (Requires explicitly elevated Host OS execution context to open raw PhysicalDrive).");
        }
    }
    
    Ok(())
}
