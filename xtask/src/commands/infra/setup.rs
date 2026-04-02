use anyhow::{Context, Result, bail};
use crate::cli::SetupAction;
use crate::constants;
use crate::utils::paths;
use crate::utils::process;

struct ProvisionPlan {
    tool: &'static str,
    windows: Option<&'static [&'static str]>,
    macos: Option<&'static [&'static str]>,
    linux_apt: Option<&'static [&'static str]>,
    linux_pacman: Option<&'static [&'static str]>,
}

fn run_provision_plan(plan: &ProvisionPlan, is_windows: bool, is_macos: bool, is_linux: bool) {
    if is_windows {
        if let Some(args) = plan.windows {
            let _ = process::run_best_effort(args[0], &args[1..]);
        }
        return;
    }

    if is_macos {
        if let Some(args) = plan.macos {
            let _ = process::run_best_effort(args[0], &args[1..]);
        }
        return;
    }

    if is_linux {
        if process::which("apt-get") {
            if let Some(args) = plan.linux_apt {
                let _ = process::run_best_effort("sudo", args);
            }
        } else if process::which("pacman") {
            if let Some(args) = plan.linux_pacman {
                let _ = process::run_best_effort("sudo", args);
            }
        }
    }
}

/// Entry layer for generic system orchestrations, toolchain validations, and automated resource provisionings.
pub fn execute(action: &SetupAction) -> Result<()> {
    match action {
        SetupAction::Audit => {
            audit_host_environment().context("Host machine evaluation capability failed")?;
        }
        SetupAction::Repair | SetupAction::Bootstrap => {
            println!("[setup::bootstrap] Initiating zero-dependency automated remediation sequence.");
            provision_host_environment().context("Strict host provisioning failed")?;
            fetch_limine_binaries().context("Bootloader synchronization failed")?;
        }
        SetupAction::FetchBootloader => {
            fetch_limine_binaries().context("Bootloader binary synchronization workflow collapsed")?;
        }
        SetupAction::Toolchain => {
            provision_cross_compiler().context("Cross-compiler synchronization failed")?;
        }
    }
    
    Ok(())
}

/// Evaluates the existing host workstation for critical OSDev tooling (QEMU, xorriso, cross-compilers).
fn audit_host_environment() -> Result<()> {
    println!("[setup::audit] Scanning host workstation architecture and active PATHs...");
    
    let required_bins = [
        constants::tools::QEMU_X86_64,
        constants::tools::XORRISO,
        constants::tools::RUSTC,
        constants::tools::CARGO,
    ];
    let mut missing = 0;
    
    for bin in required_bins {
        if process::which(bin) {
            println!("[setup::audit] [ OK ] Verified binary executable inline: {}", bin);
        } else {
            println!("[setup::audit] [FAIL] Critical system dependency missing: {}", bin);
            missing += 1;
        }
    }
    
    if missing > 0 {
        bail!("Environment audit concluded with {} missing severe dependencies. Run 'cargo run -p xtask -- setup bootstrap' to inherently resolve these.", missing);
    } else {
        println!("[setup::audit] Success. Workstation meets all structural limits for OS engineering.");
    }
    
    Ok(())
}

/// Automatically acquires missing system packages via isolated host package managers (WinGet / Scoop / Brew / APT / Pacman).
fn provision_host_environment() -> Result<()> {
    println!("[setup::provision] Negotiating missing binary tools acquisition dynamically across host architectures...");
    
    let is_windows = cfg!(windows);
    let is_macos = cfg!(target_os = "macos");
    let is_linux = cfg!(target_os = "linux");
    
    // ------------------------------------------------------------------------------------------------ //
    // 1. QEMU EMULATOR PROVISIONING
    // ------------------------------------------------------------------------------------------------ //
    if !process::which_any(&[constants::tools::QEMU_X86_64, constants::tools::QEMU_X86_64_EXE]) {
        println!("[setup::provision] QEMU architecture missing. Attempting automated host-based installation.");
        let plan = ProvisionPlan {
            tool: "qemu",
            windows: Some(&["winget", "install", "--id", "SoftwareFreedomConservancy.QEMU", "-e", "--accept-package-agreements", "--accept-source-agreements"]),
            macos: Some(&["brew", "install", "qemu"]),
            linux_apt: Some(&["apt-get", "install", "-y", "qemu-system-x86"]),
            linux_pacman: Some(&["pacman", "-S", "--noconfirm", "qemu"]),
        };
        println!("[setup::provision] Applying provisioning plan for {}.", plan.tool);
        run_provision_plan(&plan, is_windows, is_macos, is_linux);
    }
    
    // ------------------------------------------------------------------------------------------------ //
    // 2. XORRISO IMAGE MANIPULATOR PROVISIONING
    // ------------------------------------------------------------------------------------------------ //
    if !process::which_any(&[constants::tools::XORRISO, constants::tools::XORRISO_EXE]) {
        println!("[setup::provision] Xorriso dependency missing. Attempting structural acquisition.");
        let plan = ProvisionPlan {
            tool: "xorriso",
            windows: Some(&["scoop", "install", "xorriso"]),
            macos: Some(&["brew", "install", "xorriso"]),
            linux_apt: Some(&["apt-get", "install", "-y", "xorriso"]),
            linux_pacman: Some(&["pacman", "-S", "--noconfirm", "libisoburn"]),
        };
        println!("[setup::provision] Applying provisioning plan for {}.", plan.tool);
        if is_windows && !process::which("scoop") {
            println!("[setup::provision] WARNING: Please install 'scoop' (scoop.sh) to automatically acquire xorriso on Windows without MSYS2.");
        } else {
            run_provision_plan(&plan, is_windows, is_macos, is_linux);
        }
    }
    
    println!("[setup::provision] Host evaluation layout locked. Native dependencies should be established.");
    paths::ensure_dir(&constants::paths::host_tools_bin())?;
    
    Ok(())
}

/// Handles explicit OS Target Toolchain management (x86_64-elf / aarch64-elf boundaries).
fn provision_cross_compiler() -> Result<()> {
    println!("[setup::toolchain] Initiating provisioning logic for GNU/LLVM Cross-Compilation toolchains.");
    println!("[setup::toolchain] Rust inherently manages primary system compiling via #![no_std].");
    println!("[setup::toolchain] Dedicated GCC extraction would be placed within 'artifacts/host_tools/cross/'.");
    Ok(())
}

/// Automates synchronization of Limine EFI/BIOS binaries from upstream sources.
/// Allows cross-platform construction of bootable ISOs without manual configuration.
fn fetch_limine_binaries() -> Result<()> {
    println!("[setup::fetch] Connecting to upstream vendor registries for Limine payload distribution...");
    
    // Explicit static binary locations managed by the upstream Limine project
    let repos = vec![
        ("limine-bios.sys", "https://raw.githubusercontent.com/limine-bootloader/limine/v7.0-branch-binary/limine-bios.sys"),
        ("limine-bios-cd.bin", "https://raw.githubusercontent.com/limine-bootloader/limine/v7.0-branch-binary/limine-bios-cd.bin"),
        ("limine-uefi-cd.bin", "https://raw.githubusercontent.com/limine-bootloader/limine/v7.0-branch-binary/limine-uefi-cd.bin"),
        ("BOOTX64.EFI", "https://raw.githubusercontent.com/limine-bootloader/limine/v7.0-branch-binary/BOOTX64.EFI"),
    ];
    
    let dest_dir = constants::paths::limine_bin_dir();
    paths::ensure_dir(&dest_dir).context("Failed establishing directory boundaries for limiting vendored bins")?;
    
    for (filename, url) in repos {
        let dest_file = dest_dir.join(filename);
        println!("[setup::fetch] -> Streaming object via cURL: {}", filename);
        
        if !process::run_best_effort("curl", &["-L", "-o", dest_file.to_str().unwrap_or(""), url]) {
            bail!("Remote host denied binary download or connection dropped forcefully.");
        }
    }
    
    println!("[setup::fetch] Synchronization sequence successful. OS wrapper mechanisms updated to latest stable protocol.");
    Ok(())
}
