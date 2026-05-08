use crate::cli::RunAction;
use crate::constants;
use crate::utils::logging;
use anyhow::{Context, Result, bail};

pub mod guest;

/// Entry point for `cargo run -p xtask -- run <action>`.
pub fn execute(action: &RunAction) -> Result<()> {
    match action {
        RunAction::Smoke { bootloader } => {
            logging::info("run::smoke", "Starting automated generic smoke test.", &[]);
            logging::info(
                "run::smoke",
                "Target bootloader sequence",
                &[("bootloader", &format!("{:?}", bootloader))],
            );
            crate::commands::ops::qemu::smoke_test().context("Smoke test failed")?;
        }
        RunAction::Live { firmware } => {
            logging::info(
                "run::live",
                "Launching interactive QEMU graphic interface",
                &[("firmware", firmware)],
            );
            crate::commands::ops::qemu::interactive().context("Live session failed")?;
        }
        RunAction::Debug { firmware } => {
            logging::info(
                "run::debug",
                "Engaging execution mode: Paused / Waiting for GDB.",
                &[("firmware", firmware)],
            );
            crate::commands::ops::qemu::debug_session().context("Debug session failed")?;
        }
        RunAction::PxeServer { port } => {
            execute_pxe_server(*port)?;
        }
        RunAction::BareMetalDeploy { device } => {
            execute_bare_metal_deploy(device)?;
        }
        RunAction::Guest {
            distro,
            rootfs,
            download,
            cache,
            refresh,
            attach,
            firmware: _,
        } => {
            guest::launch_guest_session(distro, rootfs, *download, *cache, *refresh, *attach)?;
        }
    }
    Ok(())
}

fn execute_pxe_server(port: u16) -> Result<()> {
    logging::info("run::pxe", "Emulating stateless distribution host", &[]);
    let host_dir = constants::paths::artifact_dir();
    if !host_dir.exists() {
        bail!("'artifacts' directory missing. Please 'build full' first.");
    }
    crate::utils::process::run_checked_in_dir(
        "python",
        &["-m", "http.server", &port.to_string()],
        &host_dir,
    )
    .context("Failed executing Python HTTP fallback host module")?;
    Ok(())
}

fn execute_bare_metal_deploy(device: &str) -> Result<()> {
    logging::warn(
        "run::deploy",
        "CRITICAL PROCESS: Immediate raw byte-transfer initialized",
        &[("device", device)],
    );
    let img_path = constants::paths::artifact_dir().join("aethercore.img");
    if !img_path.exists() {
        bail!("Logical RAW medium missing: 'aethercore.img'.");
    }
    logging::info(
        "run::deploy",
        "Stub constraint triggered (Requires elevated context)",
        &[],
    );
    Ok(())
}
