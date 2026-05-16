use anyhow::Result;
use crate::engine::{Task, ExecutionContext, task::TaskStatus};
use crate::utils::logging;
use std::path::PathBuf;

pub struct QemuRunTask {
    pub image_path: PathBuf,
    pub memory_mb: u32,
    pub smp: u32,
}

impl Task for QemuRunTask {
    fn name(&self) -> &str { "QEMU Emulation" }
    fn description(&self) -> &str { "Runs the OS image in the QEMU emulator" }
    
    fn run(&self, _ctx: &ExecutionContext) -> Result<TaskStatus> {
        logging::status("QEMU", &format!("Launching emulation for: {}", self.image_path.display()));
        
        crate::commands::ops::qemu::run(
            &self.image_path,
            self.memory_mb,
            self.smp,
            false, // gui
            None,  // extra args
        )?;
        Ok(TaskStatus::Success)
    }
}
