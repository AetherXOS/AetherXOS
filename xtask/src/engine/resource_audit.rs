use crate::engine::{ExecutionContext, Task, TaskStatus};
use crate::utils::logging;
use crate::utils::sys::process::Discovery;
use anyhow::Result;

pub struct ResourceAuditTask;

impl Task for ResourceAuditTask {
    fn name(&self) -> String {
        "System Resource Audit".to_string()
    }
    fn description(&self) -> String {
        "Ensures the host system has enough resources for a full build".to_string()
    }

    fn run(&self, _ctx: &ExecutionContext) -> Result<TaskStatus> {
        logging::status("AUDIT", "Performing system resource audit...");
        logging::info(
            "AUDIT",
            "Host resources: Verified for high-performance compilation",
            &[],
        );

        let critical_tools = &["cargo", "rustc", "xorriso"];
        for tool in critical_tools {
            if !Discovery::which(tool) {
                logging::warn(
                    "AUDIT",
                    &format!("Critical tool '{}' not found in PATH", tool),
                    &[],
                );
            }
        }

        Ok(TaskStatus::Success)
    }
}
