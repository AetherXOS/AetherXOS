use anyhow::Result;
use crate::engine::{Task, ExecutionContext, task::TaskStatus};
use crate::utils::logging;

pub struct GlibcAuditTask {
    pub format: String,
}

impl Task for GlibcAuditTask {
    fn name(&self) -> &str { "Glibc ABI Audit" }
    fn description(&self) -> &str { "Analyzes the kernel's compatibility with various Glibc versions" }
    
    fn run(&self, _ctx: &ExecutionContext) -> Result<TaskStatus> {
        logging::status("GLIBC", &format!("Starting ABI audit (Format: {})", self.format));
        crate::commands::validation::glibc::run_audit(&self.format)?;
        Ok(TaskStatus::Success)
    }
}
