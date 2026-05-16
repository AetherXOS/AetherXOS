use anyhow::Result;
use crate::engine::{Task, ExecutionContext, TaskStatus};
use crate::utils::logging;

pub struct SmokeTestTask {
    pub bootloader: crate::types::Bootloader,
}

impl Task for SmokeTestTask {
    fn name(&self) -> String { "QEMU Smoke Test".to_string() }
    fn description(&self) -> String { "Runs an automated smoke test in QEMU to verify bootability".to_string() }
    
    fn run(&self, _ctx: &ExecutionContext) -> Result<TaskStatus> {
        logging::info("SMOKE", "Initializing automated smoke test sequence", &[("bootloader", self.bootloader.as_str())]);
        crate::commands::ops::qemu::smoke_test()?;
        Ok(TaskStatus::Success)
    }
}
