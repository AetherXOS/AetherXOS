use anyhow::{Result, bail};
use std::fs;
use std::path::Path;

use crate::utils::logging;
use crate::utils::paths;
use crate::utils::process;

/// Assemble a bootable ISO image from staged boot artifacts and Limine binaries.
///
/// Replaces: scripts/build_boot_image_platform.py::build_iso()
pub fn assemble(stage_boot_dir: &Path, out_iso: &Path) -> Result<()> {
    logging::info(
        "iso",
        &format!("Assembling bootable ISO: {}", out_iso.display()),
        &[],
    );

    // Locate xorriso
    let xorriso = find_xorriso()?;
    logging::info("iso", &format!("Using xorriso: {}", xorriso), &[]);

    // Locate limine binaries
    let limine_bin_dir = paths::resolve("artifacts/limine/bin");
    let required = [
        "limine-bios-cd.bin",
        "limine-bios.sys",
        "limine-uefi-cd.bin",
        "BOOTX64.EFI",
    ];
    for name in &required {
        let p = limine_bin_dir.join(name);
        if !p.exists() {
            bail!(
                "Missing Limine binary: {}. Run limine fetch first.",
                p.display()
            );
        }
    }

    // Create ISO root layout
    let iso_root = out_iso.parent().unwrap().join("iso_root");
    if iso_root.exists() {
        fs::remove_dir_all(&iso_root)?;
    }
    fs::create_dir_all(iso_root.join("boot"))?;
    fs::create_dir_all(iso_root.join("EFI/BOOT"))?;

    // Copy staged boot artifacts into ISO root
    for entry in fs::read_dir(stage_boot_dir)? {
        let entry = entry?;
        let dest = iso_root.join("boot").join(entry.file_name());
        fs::copy(entry.path(), &dest)?;
        // Also copy limine.conf to ISO root for fallback
        if entry.file_name().to_string_lossy().to_lowercase() == "limine.conf" {
            fs::copy(entry.path(), iso_root.join("limine.conf"))?;
        }
    }

    // Copy Limine binaries
    fs::copy(
        limine_bin_dir.join("limine-bios-cd.bin"),
        iso_root.join("boot/limine-bios-cd.bin"),
    )?;
    fs::copy(
        limine_bin_dir.join("limine-bios.sys"),
        iso_root.join("boot/limine-bios.sys"),
    )?;
    fs::copy(
        limine_bin_dir.join("limine-bios.sys"),
        iso_root.join("limine-bios.sys"),
    )?;
    fs::copy(
        limine_bin_dir.join("limine-uefi-cd.bin"),
        iso_root.join("boot/limine-uefi-cd.bin"),
    )?;
    fs::copy(
        limine_bin_dir.join("BOOTX64.EFI"),
        iso_root.join("EFI/BOOT/BOOTX64.EFI"),
    )?;

    // Build ISO via xorriso
    paths::ensure_dir(out_iso.parent().unwrap())?;

    let iso_root_arg = maybe_msys_path(&iso_root, &xorriso);
    let out_iso_arg = maybe_msys_path(out_iso, &xorriso);

    process::run_checked(
        &xorriso,
        &[
            "-as",
            "mkisofs",
            "-b",
            "boot/limine-bios-cd.bin",
            "-no-emul-boot",
            "-boot-load-size",
            "4",
            "-boot-info-table",
            "--efi-boot",
            "boot/limine-uefi-cd.bin",
            "-efi-boot-part",
            "--efi-boot-image",
            "--protective-msdos-label",
            "-o",
            &out_iso_arg,
            &iso_root_arg,
        ],
    )?;

    logging::ready(
        "iso",
        "ISO assembled successfully",
        &out_iso.to_string_lossy(),
    );
    Ok(())
}

/// Find xorriso binary, including MSYS2 fallback on Windows.
fn find_xorriso() -> Result<String> {
    if process::which("xorriso") {
        return Ok("xorriso".to_string());
    }
    // Windows MSYS2 fallback
    // Check common MSYS2 and Git Bash xorriso locations
    for msys_path in &[
        r"C:\msys64\usr\bin\xorriso.exe",
        r"C:\msys32\usr\bin\xorriso.exe",
        r"C:\Program Files\Git\usr\bin\xorriso.exe",
    ] {
        if Path::new(msys_path).exists() {
            return Ok(msys_path.to_string());
        }
    }
    bail!("xorriso not found in PATH or MSYS2. Install it to build ISO images.")
}

/// Convert a Windows path to MSYS2-compatible format if needed.
fn maybe_msys_path(path: &Path, _xorriso_bin: &str) -> String {
    let raw = path.to_string_lossy().to_string();
    maybe_msys_path_for_platform(&raw, cfg!(windows))
}

fn maybe_msys_path_for_platform(raw: &str, is_windows: bool) -> String {
    // Only convert if using MSYS2 xorriso and path looks like a Windows drive path
    // Always convert Windows drive paths when xorriso is from MSYS, regardless of original case
    // On Windows, always convert drive paths to POSIX format for MSYS tools
    // Check if we're on Windows AND have a drive path (C:\, D:\, etc)
    let is_drive_path = raw.len() >= 2 && raw.as_bytes()[1] == b':';
    if is_windows && is_drive_path {
        let drive = raw.as_bytes()[0].to_ascii_lowercase() as char;
        // Convert C:\path\to\file -> /c/path/to/file
        let path_part = raw[2..].replace('\\', "/");
        format!("/{}{}", drive, path_part)
    } else {
        raw.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::maybe_msys_path_for_platform;

    #[test]
    fn maybe_msys_path_converts_drive_paths_on_windows_branch() {
        let converted = maybe_msys_path_for_platform(r"C:\work\artifacts\boot.iso", true);
        assert_eq!(converted, "/c/work/artifacts/boot.iso");
    }

    #[test]
    fn maybe_msys_path_keeps_drive_paths_on_non_windows_branch() {
        let raw = r"C:\work\artifacts\boot.iso";
        let converted = maybe_msys_path_for_platform(raw, false);
        assert_eq!(converted, raw);
    }

    #[test]
    fn maybe_msys_path_keeps_non_drive_paths_on_windows_branch() {
        let raw = r"\\server\share\boot.iso";
        let converted = maybe_msys_path_for_platform(raw, true);
        assert_eq!(converted, raw);
    }
}
