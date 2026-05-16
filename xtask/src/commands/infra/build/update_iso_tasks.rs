use anyhow::{Result, Context};
use std::path::PathBuf;
use crate::engine::{Task, ExecutionContext, TaskStatus};
use crate::utils::logging;
use crate::utils::sys::process::Executor;

pub struct IsoKernelUpdateTask {
    pub iso_path: PathBuf,
    pub kernel_path: PathBuf,
}

impl Task for IsoKernelUpdateTask {
    fn name(&self) -> String { "ISO Kernel Injection".to_string() }
    fn description(&self) -> String { "Hot-swaps the kernel binary inside an existing ISO image without a full rebuild".to_string() }
    
    fn run(&self, _ctx: &ExecutionContext) -> Result<TaskStatus> {
        let xorriso = crate::commands::infra::iso::tools::find_iso_tool()?;
        if !xorriso.contains("xorriso") {
            return Ok(TaskStatus::Failed("In-place ISO update requires xorriso".into()));
        }

        let iso_arg = crate::commands::infra::iso::iso_paths::maybe_msys_path(&self.iso_path, &xorriso);
        let kernel_arg = crate::commands::infra::iso::iso_paths::maybe_msys_path(&self.kernel_path, &xorriso);

        logging::info("UPDATE", "Performing in-place kernel swap", &[("iso", &iso_arg)]);

        Executor::new(&xorriso)
            .args(&[
                "-abort_on", "FAILURE",
                "-dev", &iso_arg,
                "-boot_image", "any", "keep",
                "-update", &kernel_arg, "/boot/aethercore.elf",
                "-commit",
            ])
            .run()
            .context("Xorriso execution failed during in-place update")?;

        Ok(TaskStatus::Success)
    }
}
