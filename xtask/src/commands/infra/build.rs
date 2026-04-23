use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;

use crate::cli::{Bootloader, BuildAction, ImageFormat};
use crate::constants::{self, cargo as cargo_consts};
use crate::utils::{cargo, context, logging, paths};
use aethercore_common::TargetArch;

/// Entry point for the `xtask build` subsystem.
/// Dispatches to the appropriate build sequence based on the CLI action.
pub fn execute(action: &BuildAction) -> Result<()> {
    match action {
        BuildAction::Full {
            arch,
            bootloader,
            format,
            release,
        } => {
            logging::info(
                "build",
                "starting end-to-end pipeline",
                &[
                    ("arch", arch.as_str()),
                    ("bootloader", bootloader.as_str()),
                    ("format", format.as_str()),
                    ("release", &release.to_string()),
                ],
            );

            build_kernel(*arch, *release).context("Failed to compile kernel component")?;
            build_initramfs().context("Failed to generate initramfs structure")?;
            bundle_image(*arch, bootloader, format)
                .context("Failed to assemble bootable image hierarchy")?;
        }
        BuildAction::Image { bootloader, format } => {
            logging::info(
                "build",
                "assembling bootable image medium",
                &[
                    ("bootloader", bootloader.as_str()),
                    ("format", format.as_str()),
                ],
            );
            bundle_image(constants::defaults::build::ARCH, bootloader, format)
                .context("Failed to assemble specific bootable image format")?;
        }
        BuildAction::Kernel { arch, release } => {
            build_kernel(*arch, *release).context("Failed to natively compile kernel")?;
        }
        BuildAction::Initramfs => {
            build_initramfs().context("Failed to pack initramfs")?;
        }
        BuildAction::App { name, release } => {
            build_userspace_app(name, *release)
                .context("Userspace application fabrication encountered a terminal error")?;
        }
        BuildAction::TierStatus => {
            logging::info("build", "generating tier status reports", &[]);
        }
    }

    logging::ready(
        "xtask",
        "pipeline process execution completed successfully",
        constants::paths::ARTIFACTS_DIR,
    );
    Ok(())
}

/// Compiles the kernel ELF payload for the explicitly defined target architecture.
fn build_kernel(arch: TargetArch, is_release: bool) -> Result<()> {
    logging::info(
        "kernel",
        "processing standard kernel build",
        &[("arch", arch.as_str())],
    );
    let target_triple = arch.to_bare_metal_triple();

    let mut args = vec![
        cargo_consts::CMD_BUILD,
        cargo_consts::ARG_TARGET,
        target_triple,
    ];
    if is_release {
        args.push(cargo_consts::ARG_RELEASE);
    }

    cargo::cargo(&args).context("Platform cargo build invocation aborted")?;
    logging::info("kernel", "architecture compilation finalized", &[]);
    Ok(())
}

/// Archives the system's ephemeral early userspace into a boot-ready CPIO packet.
fn build_initramfs() -> Result<()> {
    logging::info("ramfs", "generating CPIO compressed initramfs archive", &[]);

    let initramfs_src = constants::paths::boot_initramfs_src();
    let out_archive = constants::paths::boot_image_stage_initramfs();

    if let Some(parent) = out_archive.parent() {
        paths::ensure_dir(parent)
            .context("Failed resolving parent directory for initramfs stage")?;
    }

    crate::commands::infra::initramfs::build(&initramfs_src, &out_archive)?;
    logging::info(
        "ramfs",
        "archive packet securely locked",
        &[("path", &out_archive.to_string_lossy())],
    );
    Ok(())
}

/// Automates generic isolation compilation of peripheral userspace binaries.
fn build_userspace_app(name: &str, is_release: bool) -> Result<()> {
    logging::info(
        "app",
        "orchestrating cargo build for target app",
        &[("name", name)],
    );

    let app_dir = paths::userspace_src(name);
    if !app_dir.exists() {
        bail!(
            "Requested userspace application directory not found: {}",
            app_dir.display()
        );
    }

    let mut compiler_args = vec![
        cargo_consts::CMD_BUILD,
        cargo_consts::ARG_MANIFEST_PATH,
        cargo_consts::MANIFEST_FILE,
        cargo_consts::ARG_TARGET,
        constants::defaults::build::USERSPACE_TARGET,
    ];
    if is_release {
        compiler_args.push(cargo_consts::ARG_RELEASE);
    }

    crate::utils::cargo::cargo_in_dir(&compiler_args, &app_dir)
        .context("Userspace cargo sub-process execution unexpectedly collapsed")?;

    let target_profile = if is_release { "release" } else { "debug" };
    let compiled_elf = app_dir.join(format!(
        "target/{}/{}/{}",
        constants::defaults::build::USERSPACE_TARGET,
        target_profile,
        name
    ));

    let init_bin_dir = constants::paths::boot_image_stage_boot().join("initramfs/usr/bin");
    paths::ensure_dir(&init_bin_dir)?;

    if compiled_elf.exists() {
        fs::copy(&compiled_elf, init_bin_dir.join(name))
            .context("Failed moving synthesized user program to VFS")?;
        logging::info(
            "app",
            "integrated app into initramfs isolation bounds",
            &[("name", name)],
        );
    } else {
        bail!(
            "Critical workflow failure: Output ELF not presented where expected: {}",
            compiled_elf.display()
        );
    }

    Ok(())
}

/// Binds requested OS components (Kernel, RAM_FS, Configs) using the specified bootloader.
/// Delegates the resulting staged directory into the ultimate format defined by ImageFormat.
fn bundle_image(arch: TargetArch, bootloader: &Bootloader, format: &ImageFormat) -> Result<()> {
    let stage_dir = constants::paths::boot_image_stage_boot();
    paths::ensure_dir(&stage_dir)?;
    let target_triple = arch.to_bare_metal_triple();

    // Abstracted stage kernel artifact path (Rust emits without .elf on unknown-none)
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

    // Embed bootloader environment parameters
    match bootloader {
        Bootloader::Limine => {
            logging::info("image", "injecting limine protocol definitions", &[]);
            crate::commands::infra::limine::generate_configs(
                &stage_dir,
                "aethercore.elf",
                "initramfs.cpio.gz",
                constants::defaults::run::KERNEL_APPEND,
            )
            .context("Limine baseline integration process failed")?;
        }
        Bootloader::Multiboot2 | Bootloader::Grub => {
            logging::info("image", "injecting multiboot2/grub2 legacy bindings", &[]);
            let grub_cfg = stage_dir.join("grub.cfg");
            let cfg_content = "set timeout=0\nset default=0\nmenuentry \"Aether X OS\" {\n  multiboot2 /boot/aethercore.elf\n  boot\n}\n";
            fs::write(grub_cfg, cfg_content).context("GRUB sequential binding failed")?;
        }
        Bootloader::Direct => {
            logging::info("image", "direct execution bypass activated", &[]);
        }
    }

    // Target emission handling
    let cli_outdir = context::out_dir();

    match format {
        ImageFormat::Iso => {
            let iso_out = cli_outdir.join("aethercore.iso");
            crate::commands::infra::iso::assemble(&stage_dir, &iso_out)
                .context("Native ISO xorriso manipulation failed")?;
            logging::ready("image", "ISO image ready", iso_out.to_string_lossy());
        }
        ImageFormat::Img => {
            let base_iso = cli_outdir.join("aethercore-img-intermediate.iso");
            crate::commands::infra::iso::assemble(&stage_dir, &base_iso)?;

            let img_out = cli_outdir.join("aethercore.img");
            logging::info("image", "converting target to block RAW format", &[]);
            generate_raw_image(&base_iso, &img_out)?;
            let _ = fs::remove_file(base_iso);
            logging::ready("image", "disk image ready", img_out.to_string_lossy());
        }
        ImageFormat::Vhd => {
            let base_iso = cli_outdir.join("aethercore-vhd-intermediate.iso");
            crate::commands::infra::iso::assemble(&stage_dir, &base_iso)?;

            let vhd_out = cli_outdir.join("aethercore.vhd");
            logging::info("image", "converting target to VHD architecture", &[]);
            generate_vhd_image(&base_iso, &vhd_out)?;
            let _ = fs::remove_file(base_iso);
            logging::ready("image", "VHD image ready", vhd_out.to_string_lossy());
        }
    }

    Ok(())
}

fn qemu_img_binary() -> Option<&'static str> {
    crate::utils::process::find_qemu_img()
}

/// Internal pipeline tool to translate a generic ISO layout into an absolute RAW block format (dd-capable)
fn generate_raw_image(iso_src: &Path, img_dest: &Path) -> Result<()> {
    if !iso_src.exists() {
        bail!("Source ISO object unavailable for requested RAW conversion operation.");
    }

    if img_dest.exists() {
        let src_meta = fs::metadata(iso_src).ok();
        let dest_meta = fs::metadata(img_dest).ok();
        if let (Some(s), Some(d)) = (src_meta, dest_meta) {
            if let (Ok(s_time), Ok(d_time)) = (s.modified(), d.modified()) {
                if s_time <= d_time {
                    logging::info("image", "RAW conversion skipped (already up to date)", &[]);
                    return Ok(());
                }
            }
        }
    }

    // Prefer QEMU-IMG binary translations if available on host. Fallback to 1-to-1 ISOHybrid block copy natively.
    if let Some(qemu_img) = qemu_img_binary() {
        logging::info("image", "relying on qemu-img translation sub-system", &[]);
        crate::utils::process::run_checked(
            qemu_img,
            &[
                "convert",
                "-O",
                "raw",
                &iso_src.to_string_lossy(),
                &img_dest.to_string_lossy(),
            ],
        )
        .context("QEMU-IMG structural synthesis failed.")?;
    } else {
        logging::info(
            "image",
            "standard host fallback: copying native ISOHybrid",
            &[],
        );
        fs::copy(iso_src, img_dest).context("ISOHybrid clone translation failed.")?;
    }

    Ok(())
}

/// Internal pipeline tool to translate generic output into hypervisor compatible structures
fn generate_vhd_image(iso_src: &Path, vhd_dest: &Path) -> Result<()> {
    if !iso_src.exists() {
        bail!("Source ISO object unavailable for requested VHD conversion operation.");
    }

    if vhd_dest.exists() {
        let src_meta = fs::metadata(iso_src).ok();
        let dest_meta = fs::metadata(vhd_dest).ok();
        if let (Some(s), Some(d)) = (src_meta, dest_meta) {
            if let (Ok(s_time), Ok(d_time)) = (s.modified(), d.modified()) {
                if s_time <= d_time {
                    logging::info("image", "VHD conversion skipped (already up to date)", &[]);
                    return Ok(());
                }
            }
        }
    }

    // Explicit hard dependency requirement for hypervisor-level translations (VirtualPC formatting)
    if let Some(qemu_img) = qemu_img_binary() {
        logging::info("image", "requesting qemu-img vpc header construction", &[]);
        crate::utils::process::run_checked(
            qemu_img,
            &[
                "convert",
                "-O",
                "vpc",
                &iso_src.to_string_lossy(),
                &vhd_dest.to_string_lossy(),
            ],
        )
        .context("QEMU-IMG VHD header translation constraint failed.")?;
    } else {
        bail!(
            "A verified QEMU environment is strictly required on this host workstation to construct VHD layouts."
        );
    }

    Ok(())
}
