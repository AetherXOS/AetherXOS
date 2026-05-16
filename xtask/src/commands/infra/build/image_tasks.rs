use anyhow::{Context, Result};
use crate::engine::{Task, ExecutionContext, task::TaskStatus};
use crate::types::{Bootloader, ImageFormat};
use crate::utils::logging;

pub struct KernelStageTask;

impl Task for KernelStageTask {
    fn name(&self) -> &str { "Kernel Staging" }
    fn description(&self) -> &str { "Locates the compiled kernel binary and copies it to the staging area" }
    
    fn run(&self, ctx: &ExecutionContext) -> Result<TaskStatus> {
        let staging = ctx.staging.as_ref().context("Staging area not initialized")?;
        let bin_path = ctx.resolve_target_binary("aether-x-os", "aethercore");
        
        if !bin_path.exists() {
            return Ok(TaskStatus::Failed(format!("Kernel binary not found at {}", bin_path.display())));
        }

        staging.copy_file(&bin_path, "boot/aethercore.elf")?;
        Ok(TaskStatus::Success)
    }
}

pub struct BootConfigTask {
    pub bootloader: Bootloader,
}

impl Task for BootConfigTask {
    fn name(&self) -> &str { "Bootloader Configuration" }
    fn description(&self) -> &str { "Generates bootloader-specific configuration files" }
    
    fn run(&self, ctx: &ExecutionContext) -> Result<TaskStatus> {
        let staging = ctx.staging.as_ref().context("Staging area not initialized")?;
        let boot_dir = staging.root.join("boot");
        
        match self.bootloader {
            Bootloader::Limine => {
                crate::commands::infra::limine::generate_configs(
                    &boot_dir,
                    "aethercore.elf",
                    Some("initramfs.cpio.gz"),
                    crate::constants::defaults::run::KERNEL_APPEND
                )?;
            }
            _ => return Ok(TaskStatus::Skipped("Bootloader not yet supported in task engine".into())),
        }
        
        Ok(TaskStatus::Success)
    }
}

pub struct ImageFinalizeTask {
    pub format: ImageFormat,
}

impl Task for ImageFinalizeTask {
    fn name(&self) -> &str { "Image Finalization" }
    fn description(&self) -> &str { "Converts the staging area into the final bootable image format" }
    
    fn run(&self, ctx: &ExecutionContext) -> Result<TaskStatus> {
        let staging = ctx.staging.as_ref().context("Staging area not initialized")?;
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
        
        logging::ready("IMAGE", "Image finalized and ready for deployment", &output_path.to_string_lossy());
        Ok(TaskStatus::Success)
    }

    fn cleanup(&self, ctx: &ExecutionContext) -> Result<()> {
        let temp_iso = ctx.artifact_path("intermediate.iso");
        if temp_iso.exists() {
            logging::info("CLEANUP", "Removing intermediate artifacts", &[("file", &temp_iso.to_string_lossy())]);
            let _ = std::fs::remove_file(temp_iso);
        }
        Ok(())
    }
}

impl ImageFinalizeTask {
    fn convert_image(&self, src: &std::path::Path, dest: &std::path::Path, format: &str) -> Result<()> {
        if let Some(qemu_img) = crate::utils::sys::process::find_qemu_img() {
            crate::utils::sys::process::run_checked(
                qemu_img,
                &["convert", "-O", format, &src.to_string_lossy(), &dest.to_string_lossy()]
            )?;
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
