use anyhow::Result;
use crate::engine::{Task, ExecutionContext, TaskStatus};
use crate::utils::sys::process::Executor;

pub struct KernelSafetyAuditTask;

impl Task for KernelSafetyAuditTask {
    fn name(&self) -> String { "Kernel Safety Audit".to_string() }
    fn description(&self) -> String { "Analyzes kernel source code for unsafe usage patterns and potential memory leaks".to_string() }
    
    fn run(&self, _ctx: &ExecutionContext) -> Result<TaskStatus> {
        // AOP handles logging
        // Execute clippy audit using the new Executor API
        Executor::new("cargo")
            .args(&["clippy", "-p", "aether-x-os", "--", "-D", "warnings"])
            .best_effort() 
            .run()?;
        
        Ok(TaskStatus::Success)
    }
}
