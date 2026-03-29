use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::BuildAction;
use crate::utils::cargo;
use crate::utils::paths;

/// Entry point for `cargo xtask build <action>`.
pub fn execute(action: &BuildAction) -> Result<()> {
    match action {
        BuildAction::Full => full_pipeline(),
        BuildAction::Iso => iso_only(),
        BuildAction::Kernel => kernel_only(),
        BuildAction::Initramfs => initramfs_only(),
    }
}

// ---------------------------------------------------------------------------
// Full pipeline: kernel + initramfs + limine config + ISO + smoke
// ---------------------------------------------------------------------------

fn full_pipeline() -> Result<()> {
    println!("[build::full] Starting full OS build pipeline");

    let target = "x86_64-unknown-none";
    let profile = "release";
    let append = "console=ttyS0 loglevel=7";

    // Step 1: Compile kernel
    println!("[build::full] Step 1/5: Compiling kernel (target={}, profile={})", target, profile);
    cargo::cargo(&["build", "--target", target, "--release"])?;

    // Step 2: Locate ELF artifact
    println!("[build::full] Step 2/5: Locating kernel ELF artifact");
    let target_dir = Path::new("target").join(target).join(profile);
    let elf_path = find_elf_artifact(&target_dir)?;
    println!("[build::full]   Found: {}", elf_path.display());

    // Step 3: Stage boot artifacts
    println!("[build::full] Step 3/5: Staging boot artifacts");
    let stage_dir = paths::resolve("artifacts/boot_image/stage/boot");
    paths::ensure_dir(&stage_dir)?;

    let stage_kernel = stage_dir.join("hypercore.elf");
    fs::copy(&elf_path, &stage_kernel).context("Failed to stage kernel ELF")?;

    // Step 4: Generate limine configs
    println!("[build::full] Step 4/5: Generating bootloader configurations");
    crate::commands::infra::limine::generate_configs(
        &stage_dir,
        "hypercore.elf",
        "initramfs.cpio.gz",
        append,
    )?;

    // Step 5: Generate initramfs
    println!("[build::full] Step 5/5: Building initramfs archive");
    let initramfs_dir = paths::resolve("boot/initramfs");
    let initramfs_out = stage_dir.join("initramfs.cpio.gz");
    crate::commands::infra::initramfs::build(&initramfs_dir, &initramfs_out)?;

    println!("[build::full] Pipeline completed successfully.");
    Ok(())
}

fn iso_only() -> Result<()> {
    println!("[build::iso] Building bootable ISO image");
    // Build kernel + stage first, then assemble ISO
    full_pipeline()?;
    let stage_dir = paths::resolve("artifacts/boot_image/stage/boot");
    let iso_out = paths::resolve("artifacts/boot_image/hypercore.iso");
    crate::commands::infra::iso::assemble(&stage_dir, &iso_out)?;
    println!("[build::iso] ISO written: {}", iso_out.display());
    Ok(())
}

fn kernel_only() -> Result<()> {
    println!("[build::kernel] Compiling kernel only");
    cargo::cargo(&["build", "--target", "x86_64-unknown-none", "--release"])?;
    let target_dir = Path::new("target").join("x86_64-unknown-none").join("release");
    let elf = find_elf_artifact(&target_dir)?;
    println!("[build::kernel] Kernel ELF: {}", elf.display());
    Ok(())
}

fn initramfs_only() -> Result<()> {
    println!("[build::initramfs] Generating initramfs archive");
    let initramfs_dir = paths::resolve("boot/initramfs");
    let out = paths::resolve("artifacts/boot_image/stage/boot/initramfs.cpio.gz");
    paths::ensure_dir(out.parent().unwrap())?;
    crate::commands::infra::initramfs::build(&initramfs_dir, &out)?;
    println!("[build::initramfs] Archive written: {}", out.display());
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Scan a directory for the first file with a valid ELF magic header (>1KB).
fn find_elf_artifact(dir: &Path) -> Result<PathBuf> {
    if !dir.exists() {
        bail!("Target directory not found: {}", dir.display());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let meta = entry.metadata()?;
            if meta.len() > 1024 {
                let mut buf = [0u8; 4];
                let file = fs::File::open(&path)?;
                use std::io::Read;
                let mut reader = std::io::BufReader::new(file);
                if reader.read_exact(&mut buf).is_ok() && buf == *b"\x7fELF" {
                    return Ok(path);
                }
            }
        }
    }
    bail!("No ELF artifact found in {}", dir.display())
}
