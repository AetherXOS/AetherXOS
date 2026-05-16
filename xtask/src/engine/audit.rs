use anyhow::{Result, Context};
use crate::engine::{Task, ExecutionContext, TaskStatus};
use crate::utils::logging;
use crate::utils::sys::process::Executor;

pub struct ToolchainAuditTask;

impl Task for ToolchainAuditTask {
    fn name(&self) -> String { "Toolchain Audit".to_string() }
    fn description(&self) -> String { "Ensures all required Rust targets and components are installed".to_string() }
    
    fn run(&self, _ctx: &ExecutionContext) -> Result<TaskStatus> {
        logging::status("AUDIT", "Checking Rust toolchain components...");
        
        let installed = Executor::new("rustup")
            .args(&["target", "list", "--installed"])
            .run_capture()
            .context("Failed to run rustup")?;
            
        if !installed.contains("x86_64-unknown-none") {
            logging::warn("AUDIT", "Target 'x86_64-unknown-none' is missing. Attempting auto-installation...", &[]);
            Executor::new("rustup")
                .args(&["target", "add", "x86_64-unknown-none"])
                .run()?;
        }
        
        Ok(TaskStatus::Success)
    }
}
