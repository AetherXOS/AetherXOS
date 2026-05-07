use anyhow::{Result, bail};
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::utils::{logging, paths as utils_paths, process, fs as fs_utils};

pub mod iso_paths;
pub mod tools;
pub mod utils;

/// Assemble a bootable ISO image from staged boot artifacts and Limine binaries.
pub fn assemble(stage_boot_dir: &Path, out_iso: &Path) -> Result<()> {
    logging::info("iso", &format!("Assembling bootable ISO: {}", out_iso.display()), &[]);

    tools::ensure_iso_tools()?;
    let xorriso = tools::find_iso_tool()?;
    logging::info("iso", &format!("Using ISO tool: {}", xorriso), &[]);

    let limine_bin_dir = utils_paths::resolve("artifacts/limine/bin");
    let required = ["limine-bios-cd.bin", "limine-bios.sys", "limine-uefi-cd.bin", "BOOTX64.EFI"];
    for name in &required {
        let p = limine_bin_dir.join(name);
        if !p.exists() {
            bail!("Missing Limine binary: {}. Run limine fetch first.", p.display());
        }
    }

    let iso_root = utils::unique_iso_root(out_iso)?;
    fs::create_dir_all(iso_root.join("boot"))?;
    fs::create_dir_all(iso_root.join("EFI/BOOT"))?;

    // Copy staged artifacts
    for entry in fs::read_dir(stage_boot_dir)? {
        let entry = entry?;
        let src = entry.path();
        let dest = iso_root.join("boot").join(entry.file_name());
        if src.is_dir() {
            fs_utils::copy_dir_all(&src, &dest)?;
        } else {
            fs::copy(&src, &dest)?;
        }
        if entry.file_name().to_string_lossy().to_lowercase() == "limine.conf" {
            fs::copy(&src, iso_root.join("limine.conf"))?;
        }
    }

    // Copy Limine binaries
    let boot_files = [
        ("limine-bios-cd.bin", "boot/limine-bios-cd.bin"),
        ("limine-bios.sys", "boot/limine-bios.sys"),
        ("limine-uefi-cd.bin", "boot/limine-uefi-cd.bin"),
        ("BOOTX64.EFI", "EFI/BOOT/BOOTX64.EFI"),
    ];
    for (src_name, dest_rel) in &boot_files {
        fs::copy(limine_bin_dir.join(src_name), iso_root.join(dest_rel))?;
    }

    utils_paths::ensure_dir(out_iso.parent().unwrap())?;
    if out_iso.exists() {
        fs_utils::try_remove_file_with_retries(out_iso, 3)?;
    }

    if xorriso.contains("oscdimg") {
        let boot_sector = iso_root.join("boot/limine-bios-cd.bin");
        let efi_boot = iso_root.join("boot/limine-uefi-cd.bin");
        let boot_data = format!("2#p0,e,b\"{}\"#pEF,e,b\"{}\"", 
            boot_sector.to_string_lossy().replace('\\', "/"),
            efi_boot.to_string_lossy().replace('\\', "/"));
        
        process::run_checked(&xorriso, &[
            "-m", "-o", "-u2", &format!("-bootdata:{}", boot_data),
            iso_root.to_string_lossy().as_ref(), out_iso.to_string_lossy().as_ref(),
        ])?;
    } else {
        let iso_root_arg = iso_paths::maybe_msys_path(&iso_root, &xorriso);
        let out_iso_arg = iso_paths::maybe_msys_path(out_iso, &xorriso);
        
        let mut args = if xorriso.contains("xorriso") { vec!["-as", "mkisofs"] } else { vec![] };
        args.extend(&[
            "-R", "-J", "-b", "boot/limine-bios-cd.bin", "-c", "boot/bootcat",
            "-no-emul-boot", "-boot-load-size", "4", "-boot-info-table",
            "--efi-boot", "boot/limine-uefi-cd.bin", "-efi-boot-part", "--efi-boot-image",
            "--protective-msdos-label", "-o", &out_iso_arg, &iso_root_arg,
        ]);

        let output = Command::new(&xorriso).args(&args).output()?;
        if !output.status.success() {
            bail!("ISO tool failed: {}", String::from_utf8_lossy(&output.stderr));
        }
    }

    logging::ready("iso", "ISO assembled successfully", out_iso.to_string_lossy());
    
    // Post-Assembly Verification
    verify_iso_integrity(out_iso)?;
    
    let _ = fs::remove_dir_all(&iso_root);
    Ok(())
}

fn verify_iso_integrity(iso_path: &Path) -> Result<()> {
    logging::info("iso", "starting post-assembly integrity audit", &[("path", &iso_path.to_string_lossy())]);
    
    if !iso_path.exists() {
        bail!("Verification Fault: ISO image was not presented at expected location.");
    }

    let meta = fs::metadata(iso_path)?;
    if meta.len() < 1024 * 1024 {
        logging::warn("iso", "ISO size is unusually small. Verify content manually.", &[("size", &format!("{} bytes", meta.len()))]);
    }

    // Try to list contents if 7z is available to ensure bootloader files are present
    if process::which("7z") {
        let output = Command::new("7z").args(["l", &iso_path.to_string_lossy()]).output()?;
        let list = String::from_utf8_lossy(&output.stdout);
        
        let critical_files = ["aethercore.elf", "limine.conf", "limine-bios.sys"];
        for file in &critical_files {
            if !list.contains(file) {
                bail!("Integrity Fault: Critical boot asset '{}' is missing from the finalized ISO.", file);
            }
        }
        logging::info("iso", "deep content inspection successful", &[]);
    }

    Ok(())
}

/// Finalize a pre-populated ISO root directory into a bootable ISO image.
pub fn finalize_iso_from_root(iso_root: &Path, out_iso: &Path) -> Result<()> {
    tools::ensure_iso_tools()?;
    let xorriso = tools::find_iso_tool()?;
    
    utils_paths::ensure_dir(out_iso.parent().unwrap())?;
    if out_iso.exists() {
        fs_utils::try_remove_file_with_retries(out_iso, 3)?;
    }

    if xorriso.contains("oscdimg") {
        let boot_sector = iso_root.join("boot/limine-bios-cd.bin");
        let efi_boot = iso_root.join("boot/limine-uefi-cd.bin");
        let boot_data = format!("2#p0,e,b\"{}\"#pEF,e,b\"{}\"", 
            boot_sector.to_string_lossy().replace('\\', "/"),
            efi_boot.to_string_lossy().replace('\\', "/"));
        
        process::run_checked(&xorriso, &[
            "-m", "-o", "-u2", &format!("-bootdata:{}", boot_data),
            iso_root.to_string_lossy().as_ref(), out_iso.to_string_lossy().as_ref(),
        ])?;
    } else {
        let iso_root_arg = iso_paths::maybe_msys_path(iso_root, &xorriso);
        let out_iso_arg = iso_paths::maybe_msys_path(out_iso, &xorriso);
        
        let mut args = if xorriso.contains("xorriso") { vec!["-as", "mkisofs"] } else { vec![] };
        args.extend(&[
            "-R", "-J", "-b", "boot/limine-bios-cd.bin", "-c", "boot/bootcat",
            "-no-emul-boot", "-boot-load-size", "4", "-boot-info-table",
            "--efi-boot", "boot/limine-uefi-cd.bin", "-efi-boot-part", "--efi-boot-image",
            "--protective-msdos-label", "-o", &out_iso_arg, &iso_root_arg,
        ]);

        let output = Command::new(&xorriso).args(&args).output()?;
        if !output.status.success() {
            bail!("ISO tool failed: {}", String::from_utf8_lossy(&output.stderr));
        }
    }

    logging::ready("iso", "ISO finalized successfully", out_iso.to_string_lossy());
    Ok(())
}
