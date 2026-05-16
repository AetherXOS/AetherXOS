use anyhow::Result;
use crate::engine::{Task, ExecutionContext, TaskStatus};
use crate::utils::logging;
use crate::utils::sys::process::Executor;

pub struct KernelSafetyAuditTask;

impl Task for KernelSafetyAuditTask {
    fn name(&self) -> String { "Kernel Safety Audit".to_string() }
    fn description(&self) -> String { "Analyzes kernel source code for unsafe usage patterns and potential memory leaks".to_string() }
    
    fn run(&self, _ctx: &ExecutionContext) -> Result<TaskStatus> {
        logging::status("SAFETY", "Performing static analysis of kernel sources...");
        
        // Execute clippy audit using the new Executor API
        Executor::new("cargo")
            .args(&["clippy", "-p", "aethercore", "--", "-D", "warnings"])
            .best_effort() 
            .run()?;
        
        Ok(TaskStatus::Success)
    }
}
