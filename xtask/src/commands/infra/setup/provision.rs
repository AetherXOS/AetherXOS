use super::platform::{HostPlatform, ProvisionPlan, detect_platform, ensure_tool_with_plan};
use crate::constants;
use crate::utils::paths;
use anyhow::Result;

fn qemu_plan() -> ProvisionPlan {
    ProvisionPlan {
        tool: "qemu",
        windows: Some(&[
            "winget",
            "install",
            "--id",
            "SoftwareFreedomConservancy.QEMU",
            "-e",
            "--accept-package-agreements",
            "--accept-source-agreements",
        ]),
        macos: Some(&["brew", "install", "qemu"]),
        linux_apt: Some(&["apt-get", "install", "-y", "qemu-system-x86"]),
        linux_pacman: Some(&["pacman", "-S", "--noconfirm", "qemu"]),
    }
}

fn xorriso_plan() -> ProvisionPlan {
    ProvisionPlan {
        tool: "xorriso",
        windows: Some(&["scoop", "install", "xorriso"]),
        macos: Some(&["brew", "install", "xorriso"]),
        linux_apt: Some(&["apt-get", "install", "-y", "xorriso"]),
        linux_pacman: Some(&["pacman", "-S", "--noconfirm", "libisoburn"]),
    }
}

fn provision_qemu(platform: HostPlatform) {
    ensure_tool_with_plan(
        &[
            constants::tools::QEMU_X86_64,
            constants::tools::QEMU_X86_64_EXE,
        ],
        "[setup::provision] QEMU architecture missing. Attempting automated host-based installation.",
        &qemu_plan(),
        platform,
        None,
    );
}

fn provision_xorriso(platform: HostPlatform) {
    ensure_tool_with_plan(
        &[constants::tools::XORRISO, constants::tools::XORRISO_EXE],
        "[setup::provision] Xorriso dependency missing. Attempting structural acquisition.",
        &xorriso_plan(),
        platform,
        Some((
            "scoop",
            "[setup::provision] WARNING: Please install 'scoop' (scoop.sh) to automatically acquire xorriso on Windows without MSYS2.",
        )),
    );
}

/// Automatically acquires missing system packages via isolated host package managers (WinGet / Scoop / Brew / APT / Pacman).
pub(crate) fn provision_host_environment() -> Result<()> {
    println!(
        "[setup::provision] Negotiating missing binary tools acquisition dynamically across host architectures..."
    );

    let platform = detect_platform();
    provision_qemu(platform);
    provision_xorriso(platform);

    println!(
        "[setup::provision] Host evaluation layout locked. Native dependencies should be established."
    );
    paths::ensure_dir(&constants::paths::host_tools_bin())?;

    Ok(())
}

/// Handles explicit OS Target Toolchain management (x86_64-elf / aarch64-elf boundaries).
pub(crate) fn provision_cross_compiler() -> Result<()> {
    println!(
        "[setup::toolchain] Initiating provisioning logic for GNU/LLVM Cross-Compilation toolchains."
    );
    println!("[setup::toolchain] Rust inherently manages primary system compiling via #![no_std].");
    println!(
        "[setup::toolchain] Dedicated GCC extraction would be placed within 'artifacts/host_tools/cross/'."
    );
    Ok(())
}
