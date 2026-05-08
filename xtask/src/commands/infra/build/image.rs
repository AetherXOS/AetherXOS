use crate::cli::{Bootloader, ImageFormat};
use crate::constants;
use crate::utils::{context, fs as fs_utils, logging, paths, process};
use aethercore_common::TargetArch;
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;

use super::raw_disk;
use super::rootfs;

/// Binds requested OS components (Kernel, RAM_FS, Configs) using the specified bootloader.
/// Delegates the resulting staged directory into the ultimate format defined by ImageFormat.
pub fn bundle_image(
    arch: TargetArch,
    bootloader: &Bootloader,
    format: &ImageFormat,
    external_rootfs: Option<&Path>,
) -> Result<()> {
    let stage_dir = constants::paths::boot_image_stage_boot();
    paths::ensure_dir(&stage_dir)?;
    let target_triple = arch.to_bare_metal_triple();

    // Abstracted stage kernel artifact path
    let kernel_src = paths::resolve(&format!("target/{}/debug/aethercore", target_triple));
    let kernel_src_release =
        paths::resolve(&format!("target/{}/release/aethercore", target_triple));

    let active_kernel = if kernel_src_release.exists() {
        &kernel_src_release
    } else {
        &kernel_src
    };

    if active_kernel.exists() {
        fs::copy(active_kernel, stage_dir.join("aethercore.elf"))
            .context("Failed staging binary kernel executable payload")?;
    } else {
        logging::info("image", "WARNING: kernel executable not discovered", &[]);
    }

    // Handle external rootfs
    if let Some(rootfs_path) = external_rootfs {
        let target_root = stage_dir.join("var/lib/hypercore/rootfs");
        if let Some(parent) = target_root.parent() {
            paths::ensure_dir(parent)?;
        }
        paths::ensure_dir(&target_root)?;

        if rootfs_path.exists() {
            if rootfs_path.is_dir() {
                logging::info(
                    "image",
                    "Copying external rootfs directory into stage",
                    &[("src", &rootfs_path.to_string_lossy())],
                );
                fs_utils::copy_dir_all(rootfs_path, &target_root)
                    .context("Failed copying external rootfs directory into image stage")?;
            } else if rootfs_path.is_file() {
                logging::info(
                    "image",
                    "Extracting external rootfs archive into stage",
                    &[("src", &rootfs_path.to_string_lossy())],
                );
                rootfs::extract_rootfs_archive(rootfs_path, &target_root)?;
            }
        }
    }

    // Optional: create partitioned raw disk image
    if let Some(rootfs_path) = external_rootfs {
        if rootfs_path.is_dir() {
            let src_dir = stage_dir.join("var/lib/hypercore/rootfs");
            let output_img = context::out_dir().join("aethercore-rootfs.img");
            match raw_disk::create_partitioned_raw_image_from_dir(&src_dir, &output_img) {
                Ok(_) => logging::ready(
                    "image",
                    "partitioned rootfs disk image created",
                    &output_img.to_string_lossy(),
                ),
                Err(e) => logging::warn(
                    "image",
                    "failed to produce partitioned rootfs image; skipping",
                    &[("error", &e.to_string())],
                ),
            }
        }
    }

    // Bootloader configs
    match bootloader {
        Bootloader::Limine => {
            logging::info("image", "injecting limine protocol definitions", &[]);
            crate::commands::infra::limine::generate_configs(
                &stage_dir,
                "aethercore.elf",
                Some("initramfs.cpio.gz"),
                constants::defaults::run::KERNEL_APPEND,
            )?;
        }
        Bootloader::Multiboot2 | Bootloader::Grub => {
            logging::info("image", "injecting multiboot2/grub2 legacy bindings", &[]);
            let grub_cfg = stage_dir.join("grub.cfg");
            fs::write(grub_cfg, constants::boot::GRUB_CFG_TEMPLATE)?;
        }
        Bootloader::Direct => {
            logging::info("image", "direct execution bypass activated", &[]);
        }
    }

    // Final image assembly
    let cli_outdir = context::out_dir();
    match format {
        ImageFormat::Iso => {
            let iso_out = cli_outdir.join("aethercore.iso");
            crate::commands::infra::iso::assemble(&stage_dir, &iso_out)?;
            logging::ready("image", "ISO image ready", iso_out.to_string_lossy());
        }
        ImageFormat::Img => {
            let base_iso = cli_outdir.join("aethercore-img-intermediate.iso");
            crate::commands::infra::iso::assemble(&stage_dir, &base_iso)?;
            let img_out = cli_outdir.join("aethercore.img");
            generate_raw_image(&base_iso, &img_out)?;
            let _ = fs::remove_file(base_iso);
            logging::ready("image", "disk image ready", img_out.to_string_lossy());
        }
        ImageFormat::Vhd => {
            let base_iso = cli_outdir.join("aethercore-vhd-intermediate.iso");
            crate::commands::infra::iso::assemble(&stage_dir, &base_iso)?;
            let vhd_out = cli_outdir.join("aethercore.vhd");
            generate_vhd_image(&base_iso, &vhd_out)?;
            let _ = fs::remove_file(base_iso);
            logging::ready("image", "VHD image ready", vhd_out.to_string_lossy());
        }
    }

    Ok(())
}

fn generate_raw_image(iso_src: &Path, img_dest: &Path) -> Result<()> {
    if !iso_src.exists() {
        bail!("Source ISO object unavailable for requested RAW conversion operation.");
    }

    if let Some(qemu_img) = process::find_qemu_img() {
        process::run_checked(
            qemu_img,
            &[
                "convert",
                "-O",
                "raw",
                &iso_src.to_string_lossy(),
                &img_dest.to_string_lossy(),
            ],
        )?;
    } else {
        fs::copy(iso_src, img_dest)?;
    }
    Ok(())
}

fn generate_vhd_image(iso_src: &Path, vhd_dest: &Path) -> Result<()> {
    if !iso_src.exists() {
        bail!("Source ISO object unavailable for requested VHD conversion operation.");
    }

    if let Some(qemu_img) = process::find_qemu_img() {
        process::run_checked(
            qemu_img,
            &[
                "convert",
                "-O",
                "vpc",
                &iso_src.to_string_lossy(),
                &vhd_dest.to_string_lossy(),
            ],
        )?;
    } else {
        bail!("A verified QEMU environment is strictly required to construct VHD layouts.");
    }
    Ok(())
}
