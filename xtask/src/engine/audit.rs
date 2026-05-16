use anyhow::{Result, Context};
use crate::engine::{Task, ExecutionContext, task::TaskStatus};
use crate::utils::logging;

pub struct ToolchainAuditTask;

impl Task for ToolchainAuditTask {
    fn name(&self) -> &str { "Toolchain Audit" }
    fn description(&self) -> &str { "Ensures all required Rust targets and components are installed" }
    
    fn run(&self, _ctx: &ExecutionContext) -> Result<TaskStatus> {
        logging::status("AUDIT", "Checking Rust toolchain components...");
        
        // Check for x86_64-unknown-none target
        let output = std::process::Command::new("rustup")
            .args(["target", "list", "--installed"])
            .output()
            .context("Failed to run rustup")?;
            
        let installed = String::from_utf8_lossy(&output.stdout);
        if !installed.contains("x86_64-unknown-none") {
            logging::warn("AUDIT", "Target 'x86_64-unknown-none' is missing. Attempting to install...", &[]);
            crate::utils::sys::process::run_checked("rustup", &["target", "add", "x86_64-unknown-none"])?;
        }
        
        Ok(TaskStatus::Success)
    }
}
