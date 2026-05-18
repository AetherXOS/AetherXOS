use crate::engine::{ExecutionContext, Task, TaskStatus};
use crate::utils::logging;
use anyhow::Result;

pub struct DebugBridgeTask {
    pub image_path: std::path::PathBuf,
}

impl Task for DebugBridgeTask {
    fn name(&self) -> String {
        "Interactive Debug Bridge".to_string()
    }
    fn description(&self) -> String {
        "Launches QEMU with GDB server and prepares debugging symbols".to_string()
    }

    fn run(&self, _ctx: &ExecutionContext) -> Result<TaskStatus> {
        logging::status(
            "DEBUG",
            "Synchronizing symbols and launching debug bridge...",
        );

        crate::commands::ops::qemu::run(
            &self.image_path,
            1024,
            4,
            false,
            Some(&["-S".to_string(), "-s".to_string()]),
        )?;

        Ok(TaskStatus::Success)
    }
}
