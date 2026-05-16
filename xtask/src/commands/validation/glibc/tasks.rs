use anyhow::Result;
use crate::engine::{Task, ExecutionContext, TaskStatus};
use crate::utils::logging;

pub struct GlibcAuditTask {
    pub format: String,
}

impl Task for GlibcAuditTask {
    fn name(&self) -> String { "Glibc ABI Audit".to_string() }
    fn description(&self) -> String { "Analyzes the kernel's compatibility with various Glibc versions".to_string() }
    
    fn run(&self, _ctx: &ExecutionContext) -> Result<TaskStatus> {
        logging::status("GLIBC", &format!("Starting ABI audit (Format: {})", self.format));
        crate::commands::validation::glibc::run_audit(&self.format)?;
        Ok(TaskStatus::Success)
    }
}
