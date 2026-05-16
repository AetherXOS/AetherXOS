use anyhow::Result;
use super::context::ExecutionContext;

/// A high-level result from a task execution.
pub enum TaskStatus {
    Success,
    Skipped(String),
    Failed(String),
}

/// A single, atomic operation in the xtask pipeline.
pub trait Task {
    /// Human-readable name of the task.
    fn name(&self) -> &str;
    
    /// Description of what this task does.
    fn description(&self) -> &str;
    
    /// Execute the task logic.
    fn run(&self, ctx: &ExecutionContext) -> Result<TaskStatus>;
    
    /// Check if the task needs to run (e.g. file timestamps).
    fn should_run(&self, _ctx: &ExecutionContext) -> bool {
        true
    }
    
    /// Cleanup logic if the task or pipeline fails.
    fn cleanup(&self, _ctx: &ExecutionContext) -> Result<()> {
        Ok(())
    }
}
