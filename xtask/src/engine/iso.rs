use anyhow::{Result, Context};
use std::path::PathBuf;
use crate::engine::{Task, ExecutionContext, task::TaskStatus};
use crate::utils::{logging, paths};

pub struct IsoFinalizeTask {
    pub staging_dir: PathBuf,
    pub output_iso: PathBuf,
}

impl Task for IsoFinalizeTask {
    fn name(&self) -> &str { "ISO Finalization" }
    fn description(&self) -> &str { "Packages the staging directory into a bootable ISO" }
    
    fn run(&self, _ctx: &ExecutionContext) -> Result<TaskStatus> {
        logging::status("ISO", &format!("Finalizing ISO: {}", self.output_iso.display()));
        
        if let Some(parent) = self.output_iso.parent() {
            paths::ensure_dir(parent)?;
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
    fn name(&self) -> &str { "Limine Bootloader Setup" }
    fn description(&self) -> &str { "Configures Limine and copies necessary binaries to the staging area" }
    
    fn run(&self, _ctx: &ExecutionContext) -> Result<TaskStatus> {
        let boot_dir = self.staging_dir.join("boot");
        paths::ensure_dir(&boot_dir)?;
        
        std::fs::copy(&self.kernel_path, boot_dir.join("aethercore.elf"))?;
        
        let mut initrd_name = None;
        if let Some(initrd) = &self.initrd_path {
            let name = "initrd.cpio.gz";
            std::fs::copy(initrd, boot_dir.join(name))?;
            initrd_name = Some(name);
        }

        crate::commands::infra::limine::generate_configs(
            &boot_dir,
            "aethercore.elf",
            initrd_name,
            crate::constants::defaults::run::KERNEL_APPEND
        )?;
        
        Ok(TaskStatus::Success)
    }
}
