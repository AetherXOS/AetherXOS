use crate::engine::ExecutionContext;

/// Represents the possible outcomes of a task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    Success,
    Failed(String),
    Skipped(String),
}

/// The core abstraction for all build units in AetherX.
pub trait Task: Send + Sync {
    /// Unique identifier for the task instance.
    fn name(&self) -> String;
    
    /// Human-readable explanation.
    fn description(&self) -> String;
    
    /// Main execution logic.
    fn run(&self, ctx: &ExecutionContext) -> anyhow::Result<TaskStatus>;
    
    /// Predicate to check if the task is needed in the current context.
    fn should_run(&self, _ctx: &ExecutionContext) -> bool { true }
    
    /// Optional hash for incremental builds.
    fn fingerprint(&self, _ctx: &ExecutionContext) -> anyhow::Result<Option<String>> { Ok(None) }
    
    /// Cleanup logic on failure.
    fn cleanup(&self, _ctx: &ExecutionContext) -> anyhow::Result<()> { Ok(()) }
}
