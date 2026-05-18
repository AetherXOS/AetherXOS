use crate::engine::{ExecutionContext, Task, TaskStatus};
use crate::utils::logging;
use anyhow::{Context, Result};

pub struct QemuRunTask {
    pub image: std::path::PathBuf,
    pub gui: bool,
}

impl Task for QemuRunTask {
    fn name(&self) -> String {
        "QEMU Guest Execution".to_string()
    }
    fn description(&self) -> String {
        "Launches the AetherX OS image in a virtualized QEMU environment".to_string()
    }

    fn run(&self, _ctx: &ExecutionContext) -> Result<TaskStatus> {
        let qemu = crate::utils::sys::process::Discovery::qemu_system_x86_64()
            .context("qemu-system-x86_64 not found in PATH")?;

        let drive_arg = format!("file={},format=raw", self.image.display());
        let mut args = vec!["-m", "1024", "-drive", &drive_arg];
        if !self.gui {
            args.push("-nographic");
        }

        logging::status("QEMU", &format!("Launching {}...", self.image.display()));

        crate::utils::sys::process::Executor::new(qemu)
            .args(&args)
            .without_progress()
            .run()?;

        Ok(TaskStatus::Success)
    }
}
