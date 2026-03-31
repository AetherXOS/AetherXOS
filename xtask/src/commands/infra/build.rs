use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::{Bootloader, BuildAction, ImageFormat};
use crate::utils::{cargo, paths, process};

/// Entry point for the `xtask build` subsystem.
/// Dispatches to the appropriate build sequence based on the CLI action.
pub fn execute(action: &BuildAction) -> Result<()> {
    match action {
        BuildAction::Full { arch, bootloader, format, release } => {
            println!(
                "[build::full] Starting end-to-end pipeline: arch={}, bootloader={:?}, format={:?}, release={}",
                arch, bootloader, format, release
            );
            
            build_kernel(arch, *release).context("Failed to compile kernel component")?;
            build_initramfs().context("Failed to generate initramfs structure")?;
            bundle_image(bootloader, format).context("Failed to assemble bootable image hierarchy")?;
        }
        BuildAction::Image { bootloader, format } => {
            println!("[build::image] Assembling bootable image medium.");
            bundle_image(bootloader, format).context("Failed to assemble specific bootable image format")?;
        }
        BuildAction::Kernel { arch, release } => {
            build_kernel(arch, *release).context("Failed to natively compile kernel")?;
        }
        BuildAction::Initramfs => {
            build_initramfs().context("Failed to pack initramfs")?;
        }
        BuildAction::App { name, release } => {
            build_userspace_app(name, *release).context("Userspace application fabrication encountered a terminal error")?;
        }
    }
    
    println!("[build] Pipeline process execution completed successfully.");
    Ok(())
}

/// Compiles the kernel ELF payload for the explicitly defined target architecture.
fn build_kernel(arch: &str, is_release: bool) -> Result<()> {
    println!("[build::kernel] Processing standard kernel build for generic architecture: {}", arch);
    
    let target_triple = match arch {
        "x86_64" => "x86_64-unknown-none",
        "aarch64" => "aarch64-unknown-none",
        _ => bail!("Unsupported host/target architecture requested via CLI: {}", arch),
    };

    let mut args = vec!["build", "--target", target_triple];
    if is_release {
        args.push("--release");
    }

    cargo::cargo(&args).context("Platform cargo build invocation aborted")?;
    println!("[build::kernel] Architecture compilation finalized.");
    Ok(())
}

/// Archives the system's ephemeral early userspace into a boot-ready CPIO packet.
fn build_initramfs() -> Result<()> {
    println!("[build::initramfs] Generating CPIO compressed initramfs archive...");
    
    let initramfs_src = paths::resolve("boot/initramfs");
    let out_archive = paths::resolve("artifacts/boot_image/stage/boot/initramfs.cpio.gz");
    
    if let Some(parent) = out_archive.parent() {
        paths::ensure_dir(parent).context("Failed resolving parent directory for initramfs stage")?;
    }
    
    crate::commands::infra::initramfs::build(&initramfs_src, &out_archive)?;
    println!("[build::initramfs] Archive packet securely locked to: {}", out_archive.display());
    Ok(())
}

/// Automates generic isolation compilation of peripheral userspace binaries.
fn build_userspace_app(name: &str, is_release: bool) -> Result<()> {
    println!("[build::app] Orchestrating Cargo bounds for target userspace executable: {}", name);
    
    let app_dir = paths::resolve(&format!("src/userspace/{}", name));
    if !app_dir.exists() {
        bail!("Requested userspace application directory not found: {}", app_dir.display());
    }

    let mut compiler_args = vec!["build", "--manifest-path", "Cargo.toml", "--target", "x86_64-unknown-none"];
    if is_release {
        compiler_args.push("--release");
    }

    println!("[build::app] Enforcing strict #![no_std] limits and executing Rust compiler...");
    
    let status = std::process::Command::new("cargo")
        .args(&compiler_args)
        .current_dir(&app_dir)
        .status()
        .context("Userspace cargo sub-process execution unexpectedly collapsed")?;

    if !status.success() {
        bail!("Compilation context failed for userspace target: {}", name);
    }

    let target_profile = if is_release { "release" } else { "debug" };
    let compiled_elf = app_dir.join(format!("target/x86_64-unknown-none/{}/{}", target_profile, name));
    
    let init_bin_dir = paths::resolve("artifacts/boot_image/stage/boot/initramfs/usr/bin");
    paths::ensure_dir(&init_bin_dir)?;
    
    if compiled_elf.exists() {
        fs::copy(&compiled_elf, init_bin_dir.join(name)).context("Failed moving synthesized user program to VFS")?;
        println!("[build::app] Successfully integrated '{}' into initramfs isolation bounds.", name);
    } else {
        bail!("Critical workflow failure: Output ELF not presented where expected: {}", compiled_elf.display());
    }

    Ok(())
}

/// Binds requested OS components (Kernel, RAM_FS, Configs) using the specified bootloader.
/// Delegates the resulting staged directory into the ultimate format defined by ImageFormat.
fn bundle_image(bootloader: &Bootloader, format: &ImageFormat) -> Result<()> {
    let stage_dir = paths::resolve("artifacts/boot_image/stage/boot");
    paths::ensure_dir(&stage_dir)?;
    
    // Abstracted stage kernel artifact path (Rust emits without .elf on unknown-none)
    let kernel_src = paths::resolve("target/x86_64-unknown-none/debug/hypercore");
    let kernel_src_release = paths::resolve("target/x86_64-unknown-none/release/hypercore");
    
    let active_kernel = if kernel_src_release.exists() {
        &kernel_src_release
    } else {
        &kernel_src
    };

    if active_kernel.exists() {
        fs::copy(active_kernel, stage_dir.join("hypercore.elf"))
            .context("Failed staging binary kernel executable payload")?;
    } else {
        println!("[build::image] WARNING: Kernel executable not discovered at expected locations.");
    }

    // Embed bootloader environment parameters
    match bootloader {
        Bootloader::Limine => {
            println!("[build::image] Injecting Limine protocol definitions.");
            crate::commands::infra::limine::generate_configs(
                &stage_dir,
                "hypercore.elf",
                "initramfs.cpio.gz",
                "console=ttyS0 loglevel=7",
            ).context("Limine baseline integration process failed")?;
        }
        Bootloader::Multiboot2 | Bootloader::Grub => {
            println!("[build::image] Injecting Multiboot2/GRUB2 legacy bindings.");
            let grub_cfg = stage_dir.join("grub.cfg");
            let cfg_content = "set timeout=0\nset default=0\nmenuentry \"AetherXOS\" {\n  multiboot2 /boot/hypercore.elf\n  boot\n}\n";
            fs::write(grub_cfg, cfg_content).context("GRUB sequential binding failed")?;
        }
        Bootloader::Direct => {
            println!("[build::image] Notice: Direct execution bypass activated. Extraneous wrappers omitted.");
        }
    }

    // Target emission handling
    let outdir_env = std::env::var("XTASK_OUTDIR").unwrap_or_else(|_| "artifacts".to_string());
    let cli_outdir = PathBuf::from(outdir_env);
    
    match format {
        ImageFormat::Iso => {
            let iso_out = cli_outdir.join("hypercore.iso");
            crate::commands::infra::iso::assemble(&stage_dir, &iso_out)
                .context("Native ISO xorriso manipulation failed")?;
            println!("[build::image] ISO Image ready: {}", iso_out.display());
        }
        ImageFormat::Img => {
            // First require the base ISOHybrid via xorriso, then transform natively
            let base_iso = cli_outdir.join("hypercore-img-intermediate.iso");
            crate::commands::infra::iso::assemble(&stage_dir, &base_iso)?;
            
            let img_out = cli_outdir.join("hypercore.img");
            println!("[build::image] Converting target explicitly to block RAW format (.img)...");
            generate_raw_image(&base_iso, &img_out)?;
            
            // Cleanup intermediary
            let _ = fs::remove_file(base_iso);
        }
        ImageFormat::Vhd => {
            let base_iso = cli_outdir.join("hypercore-vhd-intermediate.iso");
            crate::commands::infra::iso::assemble(&stage_dir, &base_iso)?;
            
            let vhd_out = cli_outdir.join("hypercore.vhd");
            println!("[build::image] Converting target to Microsoft VirtualPC (VHD) architecture...");
            generate_vhd_image(&base_iso, &vhd_out)?;
            
            let _ = fs::remove_file(base_iso);
        }
    }

    Ok(())
}

/// Internal pipeline tool to translate a generic ISO layout into an absolute RAW block format (dd-capable)
fn generate_raw_image(iso_src: &Path, img_dest: &Path) -> Result<()> {
    if !iso_src.exists() {
        bail!("Source ISO object unavailable for requested RAW conversion operation.");
    }
    
    // Prefer QEMU-IMG binary translations if available on host. Fallback to 1-to-1 ISOHybrid block copy natively.
    if process::which("qemu-img") || process::which("qemu-img.exe") {
        println!("[build::img] Relying on qemu-img translation sub-system.");
        process::run_checked("qemu-img", &["convert", "-O", "raw", &iso_src.to_string_lossy(), &img_dest.to_string_lossy()])
            .context("QEMU-IMG structural synthesis failed.")?;
    } else {
        println!("[build::img] Standard Host fallback: Copying native ISOHybrid byte-stream segment.");
        fs::copy(iso_src, img_dest).context("ISOHybrid clone translation failed.")?;
    }
    
    println!("[build::img] Target format completed: {}", img_dest.display());
    Ok(())
}

/// Internal pipeline tool to translate generic output into hypervisor compatible structures
fn generate_vhd_image(iso_src: &Path, vhd_dest: &Path) -> Result<()> {
    if !iso_src.exists() {
        bail!("Source ISO object unavailable for requested VHD conversion operation.");
    }
    
    // Explicit hard dependency requirement for hypervisor-level translations (VirtualPC formatting)
    if process::which("qemu-img") || process::which("qemu-img.exe") {
        println!("[build::vhd] Requesting qemu-img vpc header construction format.");
        process::run_checked("qemu-img", &["convert", "-O", "vpc", &iso_src.to_string_lossy(), &vhd_dest.to_string_lossy()])
            .context("QEMU-IMG VHD header translation constraint failed.")?;
    } else {
        bail!("A verified QEMU environment is strictly required on this host workstation to construct VHD layouts.");
    }
    
    println!("[build::vhd] Hypervisor Target format completed: {}", vhd_dest.display());
    Ok(())
}
