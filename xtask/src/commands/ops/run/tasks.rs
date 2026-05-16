use anyhow::Result;
use crate::engine::{Task, ExecutionContext, task::TaskStatus};
use crate::utils::logging;

pub struct SmokeTestTask {
    pub bootloader: crate::types::Bootloader,
}

impl Task for SmokeTestTask {
    fn name(&self) -> &str { "QEMU Smoke Test" }
    fn description(&self) -> &str { "Runs an automated smoke test in QEMU to verify bootability" }
    
    fn run(&self, _ctx: &ExecutionContext) -> Result<TaskStatus> {
        logging::info("SMOKE", "Initializing automated smoke test sequence", &[("bootloader", self.bootloader.as_str())]);
        crate::commands::ops::qemu::smoke_test()?;
        Ok(TaskStatus::Success)
    }
}
