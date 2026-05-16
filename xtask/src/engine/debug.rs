use anyhow::Result;
use crate::engine::{Task, ExecutionContext, task::TaskStatus};
use crate::utils::logging;

pub struct DebugBridgeTask {
    pub image_path: std::path::PathBuf,
}

impl Task for DebugBridgeTask {
    fn name(&self) -> &str { "Interactive Debug Bridge" }
    fn description(&self) -> &str { "Launches QEMU with GDB server and prepares debugging symbols" }
    
    fn run(&self, _ctx: &ExecutionContext) -> Result<TaskStatus> {
        logging::status("DEBUG", "Synchronizing symbols and launching debug bridge...");
        
        // Launch QEMU in debug mode (S=frozen, s=gdb port 1234)
        crate::commands::ops::qemu::run(
            &self.image_path,
            1024, // Memory
            4,    // SMP
            false, // GUI
            Some(&["-S".to_string(), "-s".to_string()])
        )?;
        
        Ok(TaskStatus::Success)
    }
}
