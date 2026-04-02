use crate::cli::SetupAction;
use crate::constants::tools;
use crate::utils::logging;
use crate::utils::paths;
use anyhow::{bail, Context, Result};
use std::process::Command;

/// Entry layer for generic system orchestrations, toolchain validations, and automated resource provisionings.
pub fn execute(action: &SetupAction) -> Result<()> {
    match action {
        SetupAction::Audit => {
            audit_host_environment().context("Host machine evaluation capability failed")?;
        }
        SetupAction::Repair | SetupAction::Bootstrap => {
            logging::info("setup::bootstrap", "Initiating zero-dependency automated remediation sequence.", &[]);
            provision_host_environment().context("Strict host provisioning failed")?;
            fetch_limine_binaries().context("Bootloader synchronization failed")?;
        }
        SetupAction::FetchBootloader => {
            fetch_limine_binaries()
                .context("Bootloader binary synchronization workflow collapsed")?;
        }
        SetupAction::Toolchain => {
            provision_cross_compiler().context("Cross-compiler synchronization failed")?;
        }
    }

    Ok(())
}

/// Evaluates the existing host workstation for critical OSDev tooling (QEMU, xorriso, cross-compilers).
fn audit_host_environment() -> Result<()> {
    logging::info("setup::audit", "Scanning host workstation architecture and active PATHs...", &[]);

    let required_bins = vec![tools::QEMU_X86_64, tools::XORRISO, tools::RUSTC, tools::CARGO];
    let mut missing = 0;

    for bin in required_bins {
        if crate::utils::process::which(bin) {
            logging::info("setup::audit", &format!("Verified binary executable inline: {}", bin), &[]);
        } else {
            logging::error("setup::audit", &format!("Critical system dependency missing: {}", bin), &[]);
            missing += 1;
        }
    }

    if missing > 0 {
        bail!("Environment audit concluded with {} missing severe dependencies. Run 'cargo run -p xtask -- setup bootstrap' to inherently resolve these.", missing);
    } else {
        logging::info("setup::audit", "Workstation meets all structural limits for OS engineering.", &[]);
    }

    Ok(())
}

/// Automatically acquires missing system packages via isolated host package managers (WinGet / Scoop / Brew / APT / Pacman).
fn provision_host_environment() -> Result<()> {
    logging::info(
        "setup::provision",
        "Negotiating missing binary tools acquisition dynamically across host architectures...",
        &[],
    );

    let is_windows = cfg!(windows);
    let is_macos = cfg!(target_os = "macos");
    let is_linux = cfg!(target_os = "linux");

    // ------------------------------------------------------------------------------------------------ //
    // 1. QEMU EMULATOR PROVISIONING
    // ------------------------------------------------------------------------------------------------ //
    if !crate::utils::process::which(tools::QEMU_X86_64)
        && !crate::utils::process::which(tools::QEMU_X86_64_EXE)
    {
        logging::info(
            "setup::provision",
            "QEMU architecture missing. Attempting automated host-based installation.",
            &[],
        );
        if is_windows {
            logging::info(
                "setup::provision",
                "Discovered Windows Host. Engaging 'winget' automated deployment.",
                &[],
            );
            let _ = Command::new("winget")
                .args(&[
                    "install",
                    "--id",
                    "SoftwareFreedomConservancy.QEMU",
                    "-e",
                    "--accept-package-agreements",
                    "--accept-source-agreements",
                ])
                .status();
        } else if is_macos {
            logging::info(
                "setup::provision",
                "Discovered MacOS Host. Engaging 'brew' automated deployment.",
                &[],
            );
            let _ = Command::new("brew").args(&["install", "qemu"]).status();
        } else if is_linux {
            logging::info(
                "setup::provision",
                "Discovered Linux Host. Searching for active package daemon...",
                &[],
            );
            if crate::utils::process::which("apt-get") {
                let _ = Command::new("sudo")
                    .args(&["apt-get", "install", "-y", "qemu-system-x86"])
                    .status();
            } else if crate::utils::process::which("pacman") {
                let _ = Command::new("sudo")
                    .args(&["pacman", "-S", "--noconfirm", "qemu"])
                    .status();
            }
        }
    }

    // ------------------------------------------------------------------------------------------------ //
    // 2. XORRISO IMAGE MANIPULATOR PROVISIONING
    // ------------------------------------------------------------------------------------------------ //
    if !crate::utils::process::which(tools::XORRISO)
        && !crate::utils::process::which(&format!("{}.exe", tools::XORRISO))
    {
        logging::info(
            "setup::provision",
            "Xorriso dependency missing. Attempting structural acquisition.",
            &[],
        );
        if is_windows {
            logging::info(
                "setup::provision",
                "Windows detected. Attempting to use Scoop CLI for xorriso injection...",
                &[],
            );
            if crate::utils::process::which("scoop") {
                let _ = Command::new("scoop").args(&["install", "xorriso"]).status();
            } else {
                logging::warn("setup::provision", "Please install 'scoop' (scoop.sh) to automatically acquire xorriso on Windows without MSYS2.", &[]);
            }
        } else if is_macos {
            let _ = Command::new("brew").args(&["install", "xorriso"]).status();
        } else if is_linux {
            if crate::utils::process::which("apt-get") {
                let _ = Command::new("sudo")
                    .args(&["apt-get", "install", "-y", "xorriso"])
                    .status();
            } else if crate::utils::process::which("pacman") {
                let _ = Command::new("sudo")
                    .args(&["pacman", "-S", "--noconfirm", "libisoburn"])
                    .status();
            }
        }
    }

    logging::info(
        "setup::provision",
        "Host evaluation layout locked. Native dependencies should be established.",
        &[],
    );
    paths::ensure_dir(&paths::resolve("artifacts/host_tools/bin"))?;

    Ok(())
}

/// Handles explicit OS Target Toolchain management (x86_64-elf / aarch64-elf boundaries).
fn provision_cross_compiler() -> Result<()> {
    logging::info(
        "setup::toolchain",
        "Initiating provisioning logic for GNU/LLVM Cross-Compilation toolchains.",
        &[],
    );
    logging::info(
        "setup::toolchain",
        "Rust inherently manages primary system compiling via #![no_std].",
        &[],
    );
    logging::info(
        "setup::toolchain",
        "Dedicated GCC extraction would be placed within 'artifacts/host_tools/cross/'.",
        &[],
    );
    Ok(())
}

/// Automates synchronization of Limine EFI/BIOS binaries from upstream sources.
/// Allows cross-platform construction of bootable ISOs without manual configuration.
fn fetch_limine_binaries() -> Result<()> {
    logging::info(
        "setup::fetch",
        "Connecting to upstream vendor registries for Limine payload distribution...",
        &[],
    );

    // Explicit static binary locations managed by the upstream Limine project
    let repos = vec![
        ("limine-bios.sys", "https://raw.githubusercontent.com/limine-bootloader/limine/v7.0-branch-binary/limine-bios.sys"),
        ("limine-bios-cd.bin", "https://raw.githubusercontent.com/limine-bootloader/limine/v7.0-branch-binary/limine-bios-cd.bin"),
        ("limine-uefi-cd.bin", "https://raw.githubusercontent.com/limine-bootloader/limine/v7.0-branch-binary/limine-uefi-cd.bin"),
        ("BOOTX64.EFI", "https://raw.githubusercontent.com/limine-bootloader/limine/v7.0-branch-binary/BOOTX64.EFI"),
    ];

    let dest_dir = paths::resolve("artifacts/limine/bin");
    paths::ensure_dir(&dest_dir)
        .context("Failed establishing directory boundaries for limiting vendored bins")?;

    for (filename, url) in repos {
        let dest_file = dest_dir.join(filename);
        logging::info("setup::fetch", "Streaming object via cURL", &[("filename", filename)]);

        let status = Command::new("curl")
            .args(&["-L", "-o", dest_file.to_str().unwrap_or(""), url])
            .status()
            .context(
                "Host cURL invocation sequence failed. Ensure 'curl' exists statically in PATH.",
            )?;

        if !status.success() {
            bail!("Remote host denied binary download or connection dropped forcefully.");
        }
    }

    logging::ready(
        "setup::fetch",
        "Synchronization sequence successful. OS wrapper mechanisms updated to latest stable protocol.",
        &dest_dir.to_string_lossy(),
    );
    Ok(())
}
