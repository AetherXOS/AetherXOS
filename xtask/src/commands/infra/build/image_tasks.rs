use crate::engine::{ExecutionContext, Task, TaskStatus};
use crate::types::{Bootloader, ImageFormat};
use crate::utils::logging;
use anyhow::{Context, Result};

pub struct KernelStageTask;

impl Task for KernelStageTask {
    fn name(&self) -> String {
        "Kernel Staging".to_string()
    }
    fn description(&self) -> String {
        "Locates the compiled kernel binary and copies it to the staging area".to_string()
    }

    fn run(&self, ctx: &ExecutionContext) -> Result<TaskStatus> {
        let staging = ctx
            .staging
            .as_ref()
            .context("Staging area not initialized")?;
        let bin_path = ctx.resolve_target_binary("aether-x-os", "aethercore");

        if !bin_path.exists() {
            return Ok(TaskStatus::Failed(format!(
                "Kernel binary not found at {}",
                bin_path.display()
            )));
        }

        staging.copy_file(&bin_path, "boot/aethercore.elf")?;
        Ok(TaskStatus::Success)
    }
}

pub struct BootConfigTask {
    pub bootloader: Bootloader,
}

impl Task for BootConfigTask {
    fn name(&self) -> String {
        "Bootloader Configuration".to_string()
    }
    fn description(&self) -> String {
        "Generates bootloader-specific configuration files".to_string()
    }

    fn run(&self, ctx: &ExecutionContext) -> Result<TaskStatus> {
        let staging = ctx
            .staging
            .as_ref()
            .context("Staging area not initialized")?;
        let boot_dir = staging.root.join("boot");

        match self.bootloader {
            Bootloader::Limine => {
                crate::commands::infra::limine::generate_configs(
                    &boot_dir,
                    "aethercore.elf",
                    Some("initramfs.cpio.gz"),
                    crate::constants::defaults::run::KERNEL_APPEND,
                )?;
            }
            _ => {
                return Ok(TaskStatus::Skipped(
                    "Bootloader not yet supported in task engine".into(),
                ));
            }
        }

        Ok(TaskStatus::Success)
    }
}

pub struct ImageFinalizeTask {
    pub format: ImageFormat,
}

impl Task for ImageFinalizeTask {
    fn name(&self) -> String {
        "Image Finalization".to_string()
    }
    fn description(&self) -> String {
        "Converts the staging area into the final bootable image format".to_string()
    }

    fn run(&self, ctx: &ExecutionContext) -> Result<TaskStatus> {
        let staging = ctx
            .staging
            .as_ref()
            .context("Staging area not initialized")?;
        let output_name = format!("aethercore.{}", self.format.as_str());
        let output_path = ctx.artifact_path(&output_name);

        match self.format {
            ImageFormat::Iso => {
                crate::commands::infra::iso::assemble(&staging.root, &output_path)?;
            }
            ImageFormat::Img => {
                let temp_iso = ctx.artifact_path("intermediate.iso");
                crate::commands::infra::iso::assemble(&staging.root, &temp_iso)?;
                self.convert_image(&temp_iso, &output_path, "raw")?;
                let _ = std::fs::remove_file(temp_iso);
            }
            ImageFormat::Vhd => {
                let temp_iso = ctx.artifact_path("intermediate.iso");
                crate::commands::infra::iso::assemble(&staging.root, &temp_iso)?;
                self.convert_image(&temp_iso, &output_path, "vpc")?;
                let _ = std::fs::remove_file(temp_iso);
            }
        }

        logging::ready(
            "IMAGE",
            "Image finalized and ready for deployment",
            &output_path.to_string_lossy(),
            &[],
        );
        Ok(TaskStatus::Success)
    }

    fn cleanup(&self, ctx: &ExecutionContext) -> Result<()> {
        let temp_iso = ctx.artifact_path("intermediate.iso");
        if temp_iso.exists() {
            logging::info(
                "CLEANUP",
                "Removing intermediate artifacts",
                &[("file", &temp_iso.to_string_lossy())],
            );
            let _ = std::fs::remove_file(temp_iso);
        }
        Ok(())
    }
}

impl ImageFinalizeTask {
    fn convert_image(
        &self,
        src: &std::path::Path,
        dest: &std::path::Path,
        format: &str,
    ) -> Result<()> {
        if let Some(qemu_img) = crate::utils::sys::process::Discovery::qemu_img() {
            crate::utils::sys::process::Executor::new(qemu_img)
                .args(&[
                    "convert",
                    "-O",
                    format,
                    &src.to_string_lossy(),
                    &dest.to_string_lossy(),
                ])
                .run()?;
        } else {
            if format == "raw" {
                std::fs::copy(src, dest)?;
            } else {
                anyhow::bail!("qemu-img not found, cannot convert to {}", format);
            }
        }
        Ok(())
    }
}
