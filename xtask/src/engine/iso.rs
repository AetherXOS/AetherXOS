use crate::engine::{ExecutionContext, Task, TaskStatus};
use crate::utils::logging;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

pub struct IsoFinalizeTask {
    pub staging_dir: PathBuf,
    pub output_iso: PathBuf,
}

impl Task for IsoFinalizeTask {
    fn name(&self) -> String {
        "ISO Finalization".to_string()
    }
    fn description(&self) -> String {
        "Packages the staging directory into a bootable ISO image".to_string()
    }

    fn run(&self, _ctx: &ExecutionContext) -> Result<TaskStatus> {
        logging::status(
            "ISO",
            &format!("Finalizing ISO: {}", self.output_iso.display()),
        );

        if let Some(parent) = self.output_iso.parent() {
            fs::create_dir_all(parent)?;
        }

        crate::commands::infra::iso::finalize_iso_from_root(&self.staging_dir, &self.output_iso)
            .context("Failed to finalize ISO image")?;

        Ok(TaskStatus::Success)
    }
}

pub struct LimineSetupTask {
    pub staging_dir: PathBuf,
    pub kernel_path: PathBuf,
    pub initrd_path: Option<PathBuf>,
}

impl Task for LimineSetupTask {
    fn name(&self) -> String {
        "Limine Bootloader Setup".to_string()
    }
    fn description(&self) -> String {
        "Configures Limine and copies necessary binaries to the staging area".to_string()
    }

    fn run(&self, _ctx: &ExecutionContext) -> Result<TaskStatus> {
        let boot_dir = self.staging_dir.join("boot");
        fs::create_dir_all(&boot_dir)?;

        fs::copy(&self.kernel_path, boot_dir.join("aethercore.elf"))?;

        let mut initrd_name = None;
        if let Some(initrd) = &self.initrd_path {
            let name = "initrd.cpio.gz";
            fs::copy(initrd, boot_dir.join(name))?;
            initrd_name = Some(name);
        }

        crate::commands::infra::limine::generate_configs(
            &boot_dir,
            "aethercore.elf",
            initrd_name,
            crate::constants::defaults::run::KERNEL_APPEND,
        )?;

        Ok(TaskStatus::Success)
    }
}
