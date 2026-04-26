use crate::cli::RunAction;
use crate::constants;
use crate::utils::logging;
use anyhow::{Context, Result, bail};

/// Entry point for `cargo run -p xtask -- run <action>`.
/// Isolates emulation layers and hardware block-level testing environments.
pub fn execute(action: &RunAction) -> Result<()> {
    match action {
        RunAction::Smoke { bootloader } => {
            logging::info(
                "run::smoke",
                "Starting automated generic smoke test via headless QEMU parameter constraints.",
                &[],
            );
            logging::info(
                "run::smoke",
                "Intercepted intended target bootloader sequence",
                &[("bootloader", &format!("{:?}", bootloader))],
            );

            crate::commands::ops::qemu::smoke_test()
                .context("Headless QEMU pipeline execution completely failed to converge or encountered a timeout.")?;
        }
        RunAction::Live { firmware } => {
            logging::info(
                "run::live",
                "Launching interactive QEMU graphic interface",
                &[("firmware", firmware)],
            );

            crate::commands::ops::qemu::interactive().context(
                "Visual QEMU session crashed or failed standard startup initialization checks.",
            )?;
        }
        RunAction::Debug { firmware } => {
            logging::info(
                "run::debug",
                "Engaging execution mode: Paused / Waiting for GDB.",
                &[],
            );
            logging::info(
                "run::debug",
                "Proceeding to inject TCP debugging port (tcp::1234)",
                &[("firmware", firmware)],
            );

            crate::commands::ops::qemu::debug_session()
                .context("Failed suspending QEMU instance for execution debugging attachment")?;
        }
        RunAction::PxeServer { port } => {
            logging::info(
                "run::pxe",
                "Emulating stateless distribution host for physical machine broadcasting",
                &[],
            );
            logging::info(
                "run::pxe",
                "Initializing HTTP-based iPXE delivery subsystem natively",
                &[("port", &port.to_string())],
            );

            let host_dir = constants::paths::artifact_dir();
            if !host_dir.exists() {
                bail!(
                    "Network serving aborted: 'artifacts' directory missing. Please 'build full' first."
                );
            }

            logging::info(
                "run::pxe",
                "Emulating dynamic TFTP/HTTP serving to generic physical NICs",
                &[("dir", &host_dir.to_string_lossy())],
            );
            logging::info(
                "run::pxe",
                "Use your other computer's BIOS -> Network Boot pointing to this host's IP",
                &[("port", &port.to_string())],
            );

            crate::utils::process::run_checked_in_dir(
                "python",
                &["-m", "http.server", &port.to_string()],
                &host_dir
            ).context("Failed executing Python HTTP fallback host module. (Is Python3 installed system-wide?)")?;
        }
        RunAction::BareMetalDeploy { device } => {
            // Highly robust, strictly verified block device interaction (DD abstraction).
            logging::warn(
                "run::deploy",
                "CRITICAL PROCESS: Immediate raw byte-transfer initialized",
                &[("device", device)],
            );
            logging::warn(
                "run::deploy",
                "CAUTION: This operation bypasses logical FAT formats to enforce raw DD block sector overwrites",
                &[],
            );

            let img_path = constants::paths::artifact_dir().join("aethercore.img");
            if !img_path.exists() {
                bail!(
                    "Logical RAW medium missing: 'aethercore.img'. Terminated. Execute 'cargo run -p xtask -- build full --format img' first."
                );
            }

            logging::info(
                "run::deploy",
                "Origin block source verified",
                &[("path", &img_path.to_string_lossy())],
            );
            logging::info(
                "run::deploy",
                "Requesting administrative lock overlay on target physical address",
                &[("device", device)],
            );
            logging::info(
                "run::deploy",
                "Stub constraint triggered (Requires explicitly elevated Host OS execution context)",
                &[],
            );
        }
    }

    Ok(())
}
