use anyhow::{Result, Context};
use crate::engine::{Task, ExecutionContext, task::TaskStatus};
use crate::utils::logging;

pub struct HookTask {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
}

impl Task for HookTask {
    fn name(&self) -> &str { &self.name }
    fn description(&self) -> &str { "Executes a custom build hook or external script" }
    
    fn run(&self, ctx: &ExecutionContext) -> Result<TaskStatus> {
        logging::status("HOOK", &format!("Running custom hook: {}", self.name));
        
        let status = std::process::Command::new(&self.command)
            .args(&self.args)
            .current_dir(&ctx.repo_root)
            .status()
            .context(format!("Failed to execute hook command: {}", self.command))?;
            
        if status.success() {
            Ok(TaskStatus::Success)
        } else {
            Ok(TaskStatus::Failed(format!("Hook exited with code: {:?}", status.code())))
        }
    }
}
