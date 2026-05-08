use crate::cli::BuildAction;
use crate::constants;
use crate::utils::logging;
use anyhow::{Context, Result};

pub mod app;
pub mod distro;
pub mod image;
pub mod kernel;
pub mod raw_disk;
pub mod rootfs;

/// Entry point for the `xtask build` subsystem.
pub fn execute(action: &BuildAction) -> Result<()> {
    match action {
        BuildAction::Full {
            arch,
            bootloader,
            format,
            features,
            release,
            rootfs,
        } => {
            logging::info(
                "build",
                "starting end-to-end pipeline",
                &[
                    ("arch", arch.as_str()),
                    ("bootloader", bootloader.as_str()),
                    ("format", format.as_str()),
                    ("features", &features.to_string()),
                    ("release", &release.to_string()),
                ],
            );

            kernel::build_kernel(*arch, *release, *features)
                .context("Failed to compile kernel component")?;
            build_initramfs().context("Failed to generate initramfs structure")?;
            image::bundle_image(
                *arch,
                bootloader,
                format,
                rootfs.as_deref().map(|s| std::path::Path::new(s)),
            )
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
            image::bundle_image(constants::defaults::build::ARCH, bootloader, format, None)
                .context("Failed to assemble specific bootable image format")?;
        }
        BuildAction::Kernel {
            arch,
            features,
            release,
        } => {
            kernel::build_kernel(*arch, *release, *features)
                .context("Failed to natively compile kernel")?;
        }
        BuildAction::Initramfs => {
            build_initramfs().context("Failed to pack initramfs")?;
        }
        BuildAction::App { name, release } => {
            app::build_userspace_app(name, *release)
                .context("Userspace application fabrication encountered a terminal error")?;
        }
        BuildAction::DistroIso {
            distro,
            version,
            variant,
            arch,
        } => {
            distro::build_distro_iso(distro.clone(), version.clone(), variant.clone(), *arch)
                .context("Failed to build distro-based ISO")?;
        }
        BuildAction::UpdateIsoKernel {
            iso,
            kernel,
            out,
            workdir,
        } => {
            update_iso_kernel(iso, kernel.as_deref(), out.as_deref(), workdir.as_deref())
                .context("Failed to update kernel inside ISO")?;
        }
        BuildAction::TierStatus => {
            logging::info("build", "generating tier status reports", &[]);
        }
        BuildAction::VerifyElf { arch, release, elf } => {
            verify_elf_action(*arch, *release, elf.as_deref())
                .context("ELF verification pipeline failed")?;
            // Skip the generic "pipeline completed" ready-log — verify_elf prints its own
            return Ok(());
        }
    }

    logging::ready(
        "xtask",
        "pipeline process execution completed successfully",
        constants::paths::ARTIFACTS_DIR,
    );
    Ok(())
}

/// Replace the kernel file inside an existing ISO.
fn update_iso_kernel(
    iso_path: &str,
    kernel_path: Option<&str>,
    out_iso: Option<&str>,
    workdir: Option<&str>,
) -> Result<()> {
    use std::path::PathBuf;
    use std::process::Command;

    logging::info(
        "update-iso",
        "starting kernel-in-ISO update",
        &[("iso", iso_path)],
    );

    let iso = PathBuf::from(iso_path);
    if !iso.exists() {
        anyhow::bail!("Specified ISO does not exist: {}", iso.display());
    }

    // Determine kernel to inject: either supplied or rebuild default debug kernel
    let kernel_elf = if let Some(k) = kernel_path {
        let kp = PathBuf::from(k);
        if !kp.exists() {
            anyhow::bail!("Supplied kernel ELF does not exist: {}", kp.display());
        }
        kp
    } else {
        logging::info(
            "update-iso",
            "no kernel provided — rebuilding kernel (debug)",
            &[],
        );
        // Rebuild using existing kernel builder (default arch)
        crate::commands::infra::build::kernel::build_kernel(
            constants::defaults::build::ARCH,
            false,
            aethercore_common::KernelFeatures::VFS
                | aethercore_common::KernelFeatures::DRIVERS
                | aethercore_common::KernelFeatures::LOGGING,
        )
        .context("Rebuilding kernel for injection failed")?;

        // Resolve built kernel path (debug)
        let triple = constants::defaults::build::ARCH.to_bare_metal_triple();
        crate::utils::paths::resolve(&format!("target/{}/debug/aethercore", triple))
    };

    logging::info(
        "update-iso",
        "kernel resolved for injection",
        &[("kernel", &kernel_elf.to_string_lossy())],
    );

    if workdir.is_some() {
        logging::warn("update-iso", "`--workdir` is ignored in in-place mode", &[]);
    }
    if let Some(out) = out_iso {
        if PathBuf::from(out) != iso {
            anyhow::bail!(
                "In-place mode does not create a new ISO. Remove `--out` or pass the same path as `--iso`."
            );
        }
    }

    let xorriso = crate::commands::infra::iso::tools::find_iso_tool()?;
    if !xorriso.contains("xorriso") {
        anyhow::bail!(
            "In-place ISO kernel update requires xorriso. Current tool '{}' does not support update mode.",
            xorriso
        );
    }

    let iso_arg = crate::commands::infra::iso::iso_paths::maybe_msys_path(&iso, &xorriso);
    let kernel_arg = crate::commands::infra::iso::iso_paths::maybe_msys_path(&kernel_elf, &xorriso);

    logging::info(
        "update-iso",
        "updating kernel entry in-place via xorriso",
        &[("iso", &iso_arg)],
    );
    let output = Command::new(&xorriso)
        .args([
            "-abort_on",
            "FAILURE",
            "-dev",
            &iso_arg,
            "-boot_image",
            "any",
            "keep",
            "-update",
            &kernel_arg,
            "/boot/aethercore.elf",
            "-commit",
        ])
        .output()
        .context("Failed to execute xorriso in-place update")?;

    if !output.status.success() {
        anyhow::bail!(
            "xorriso in-place update failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    logging::ready(
        "update-iso",
        "ISO kernel updated in-place",
        &iso.to_string_lossy(),
    );

    Ok(())
}

/// Archives the system's ephemeral early userspace into a boot-ready CPIO packet.
fn build_initramfs() -> Result<()> {
    logging::info("ramfs", "generating CPIO compressed initramfs archive", &[]);

    let initramfs_src = constants::paths::boot_initramfs_src();
    let out_archive = constants::paths::boot_image_stage_initramfs();

    if let Some(parent) = out_archive.parent() {
        crate::utils::paths::ensure_dir(parent)
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

/// Standalone ELF integrity verification action.
///
/// Flow:
///  1. If `elf_path` is given, skip the rebuild and verify that binary directly.
///  2. Otherwise rebuild the kernel for `arch`, then validate the output ELF.
///
/// Useful for rapid iteration: `cargo xtask build verify-elf` is much faster
/// than a full `cargo xtask build distro-iso`.
fn verify_elf_action(
    arch: aethercore_common::TargetArch,
    release: bool,
    elf_path: Option<&str>,
) -> Result<()> {
    use std::time::Instant;

    let t0 = Instant::now();

    let elf = if let Some(path) = elf_path {
        // Use supplied binary — skip rebuild entirely
        let p = std::path::PathBuf::from(path);
        if !p.exists() {
            anyhow::bail!("Supplied ELF path does not exist: {}", p.display());
        }
        logging::info(
            "verify-elf",
            "using pre-built binary (skipping rebuild)",
            &[("path", path)],
        );
        p
    } else {
        // Rebuild the kernel first
        logging::info(
            "verify-elf",
            "rebuilding kernel before verification",
            &[
                ("arch", arch.as_str()),
                ("profile", if release { "release" } else { "debug" }),
            ],
        );
        kernel::build_kernel(
            arch,
            release,
            aethercore_common::KernelFeatures::VFS | aethercore_common::KernelFeatures::DRIVERS,
        )
        .context("Kernel rebuild failed")?;

        // Resolve the output ELF path
        let triple = arch.to_bare_metal_triple();
        let profile = if release { "release" } else { "debug" };
        crate::utils::paths::resolve(&format!("target/{}/{}/aethercore", triple, profile))
    };

    logging::info(
        "verify-elf",
        "running ELF security audit",
        &[("file", &elf.to_string_lossy())],
    );

    match crate::utils::elf::validate_elf(&elf) {
        Ok(()) => {
            let elapsed = t0.elapsed();
            logging::ready(
                "verify-elf",
                "ELF integrity audit PASSED",
                &format!("{:.2}s", elapsed.as_secs_f32()),
            );
        }
        Err(e) => {
            logging::warn(
                "verify-elf",
                "ELF integrity audit FAILED",
                &[("reason", &e.to_string())],
            );
            return Err(e);
        }
    }

    Ok(())
}
