use anyhow::{Result, Context};
use std::path::PathBuf;
use std::process::Command;
use crate::engine::{Task, ExecutionContext, task::TaskStatus};
use crate::utils::logging;

pub struct IsoKernelUpdateTask {
    pub iso_path: PathBuf,
    pub kernel_path: PathBuf,
}

impl Task for IsoKernelUpdateTask {
    fn name(&self) -> &str { "ISO Kernel Injection" }
    fn description(&self) -> &str { "Hot-swaps the kernel binary inside an existing ISO image without a full rebuild" }
    
    fn run(&self, _ctx: &ExecutionContext) -> Result<TaskStatus> {
        let xorriso = crate::commands::infra::iso::tools::find_iso_tool()?;
        if !xorriso.contains("xorriso") {
            return Ok(TaskStatus::Failed("In-place ISO update requires xorriso".into()));
        }

        let iso_arg = crate::commands::infra::iso::iso_paths::maybe_msys_path(&self.iso_path, &xorriso);
        let kernel_arg = crate::commands::infra::iso::iso_paths::maybe_msys_path(&self.kernel_path, &xorriso);

        logging::info("UPDATE", "Performing in-place kernel swap", &[("iso", &iso_arg)]);

        let output = Command::new(&xorriso)
            .args([
                "-abort_on", "FAILURE",
                "-dev", &iso_arg,
                "-boot_image", "any", "keep",
                "-update", &kernel_arg, "/boot/aethercore.elf",
                "-commit",
            ])
            .output()
            .context("Xorriso execution failed during in-place update")?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Ok(TaskStatus::Failed(format!("xorriso error: {}", err)));
        }

        Ok(TaskStatus::Success)
    }
}
