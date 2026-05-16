use anyhow::{Context, Result};
use crate::cli::BuildAction;
use crate::constants;
use crate::utils::logging;

pub mod app;
pub mod distro;
pub mod image;
pub mod image_tasks;
pub mod interactive;
pub mod kernel;
pub mod raw_disk;
pub mod rootfs;
pub mod tasks;
pub mod update_iso_tasks;

/// Entry point for the `xtask build` subsystem.
pub fn execute(action: &BuildAction) -> Result<()> {
    match action {
        BuildAction::Full {
            common,
            bootloader,
            format,
            rootfs,
        } => {
            let resolved_features = resolve_kernel_features("Build full pipeline", &common.features)?;
            
            let ctx = crate::engine::ExecutionContext {
                repo_root: crate::utils::core::context::repo_root(),
                out_dir: crate::utils::core::context::out_dir(),
                is_release: common.release,
                arch: common.arch.to_string(),
                features: resolved_features.to_cargo_features().iter().map(|&s| s.to_string()).collect(),
                staging: None,
                state: crate::engine::EngineState::load(),
            };

            crate::engine::Pipeline::new("Full Build Pipeline")
                .add_task(Box::new(tasks::KernelCompileTask { 
                    arch: common.arch, 
                    release: common.release, 
                    features: resolved_features 
                }))
                .add_task(Box::new(tasks::InitramfsTask))
                .run(&ctx)?;

            image::bundle_image(common.arch, bootloader, format, rootfs.as_deref().map(|s| std::path::Path::new(s)))
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
        BuildAction::Kernel { common } => {
            let resolved_features = resolve_kernel_features("Build kernel", &common.features)?;
            kernel::build_kernel(common.arch, common.release, resolved_features).context("Failed to natively compile kernel")?;
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
        BuildAction::UpdateIsoKernel { iso, kernel, out: _, workdir: _ } => {
            let kernel_path = if let Some(k) = kernel {
                std::path::PathBuf::from(k)
            } else {
                // Rebuild logic remains similar but wrapped in a pipeline if we want
                let features = crate::utils::features::kernel_features_from_default(&["vfs", "drivers"])?;
                let arch = crate::constants::defaults::build::ARCH;
                kernel::build_kernel(arch, false, features)?;
                crate::utils::paths::resolve(&format!("target/{}/debug/aethercore", arch.to_bare_metal_triple()))
            };

            let ctx = crate::engine::ExecutionContext::from_defaults();
            crate::engine::Pipeline::new("ISO Hot-Update")
                .add_task(Box::new(update_iso_tasks::IsoKernelUpdateTask {
                    iso_path: std::path::PathBuf::from(iso),
                    kernel_path,
                }))
                .run(&ctx)?;
        }
        BuildAction::TierStatus => {
            logging::info("build", "generating tier status reports", &[]);
        }
        BuildAction::VerifyElf { common, elf } => {
            verify_elf_action(common.arch, common.release, elf.as_deref())
                .context("ELF verification pipeline failed")?;
            // Skip the generic "pipeline completed" ready-log — verify_elf prints its own
            return Ok(());
        }
        BuildAction::Interactive => {
            interactive::run().context("Interactive build wizard failed")?;
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
fn verify_elf_action(arch: aethercore_common::TargetArch, release: bool, elf_path: Option<&str>) -> Result<()> {
    use std::time::Instant;

    let t0 = Instant::now();

    let elf = if let Some(path) = elf_path {
        // Use supplied binary — skip rebuild entirely
        let p = std::path::PathBuf::from(path);
        if !p.exists() {
            anyhow::bail!("Supplied ELF path does not exist: {}", p.display());
        }
        logging::info("verify-elf", "using pre-built binary (skipping rebuild)", &[
            ("path", path),
        ]);
        p
    } else {
        // Rebuild the kernel first
        logging::info("verify-elf", "rebuilding kernel before verification", &[
            ("arch",    arch.as_str()),
            ("profile", if release { "release" } else { "debug" }),
        ]);
        let features = crate::utils::features::kernel_features_from_default(&["vfs", "drivers"])
            .context("Failed resolving default kernel features for verify-elf")?;
        kernel::build_kernel(arch, release, features).context("Kernel rebuild failed")?;

        // Resolve the output ELF path
        let triple = arch.to_bare_metal_triple();
        let profile = if release { "release" } else { "debug" };
        crate::utils::paths::resolve(&format!("target/{}/{}/aethercore", triple, profile))
    };

    logging::info("verify-elf", "running ELF security audit", &[
        ("file", &elf.to_string_lossy()),
    ]);

    match crate::utils::elf::validate_elf(&elf) {
        Ok(()) => {
            let elapsed = t0.elapsed();
            logging::ready(
                "verify-elf",
                "ELF integrity audit PASSED",
                &format!("{:.2}s", elapsed.as_secs_f32()),
            );
        }
        Err(_e) => {
            logging::warn("verify-elf", "ELF integrity audit FAILED", &[
                ("reason", &_e.to_string()),
            ]);
            return Err(_e);
        }
    }

    Ok(())
}

fn resolve_kernel_features(purpose: &str, features: &Option<aethercore_common::KernelFeatures>) -> Result<aethercore_common::KernelFeatures> {
    match features {
        Some(value) => Ok(*value),
        None => {
            if crate::utils::config::is_non_interactive() {
                crate::utils::features::kernel_features_from_default(&[])
                    .context("Failed resolving default kernel features from Cargo.toml")
            } else {
                crate::utils::features::prompt_kernel_feature_selection(purpose, &[])
                    .context("Interactive kernel feature selection failed")
            }
        }
    }
}
