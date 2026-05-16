use anyhow::{Result, Context};
use crate::engine::{Task, ExecutionContext, TaskStatus};
use crate::utils::logging;
use crate::utils::sys::process::Executor;

pub struct HookTask {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
}

impl Task for HookTask {
    fn name(&self) -> String { self.name.clone() }
    fn description(&self) -> String { "Executes a custom build hook or external script".to_string() }
    
    fn run(&self, ctx: &ExecutionContext) -> Result<TaskStatus> {
        logging::status("HOOK", &format!("Running custom hook: {}", self.name));
        
        Executor::new(&self.command)
            .args(&self.args)
            .current_dir(&ctx.repo_root)
            .run()
            .context(format!("Failed to execute hook command: {}", self.command))?;
            
        Ok(TaskStatus::Success)
    }
}
