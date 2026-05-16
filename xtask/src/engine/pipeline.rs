use anyhow::{Result, anyhow};
use std::time::Instant;
use super::task::{Task, TaskStatus};
use super::context::ExecutionContext;
use crate::utils::logging;

/// An orchestrated sequence of Tasks.
pub struct Pipeline {
    pub name: String,
    pub tasks: Vec<Box<dyn Task>>,
}

impl Pipeline {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            tasks: Vec::new(),
        }
    }

    pub fn add_task(mut self, task: Box<dyn Task>) -> Self {
        self.tasks.push(task);
        self
    }

    /// Execute all tasks in the pipeline sequentially.
    pub fn run(&self, ctx: &ExecutionContext) -> Result<()> {
        logging::status("PIPELINE", &format!("Executing Workflow: {}", self.name));
        let pipeline_start = Instant::now();
        
        let mut executed_tasks = Vec::new();
        let mut final_result = Ok(());

        for (idx, task) in self.tasks.iter().enumerate() {
            let task_name = task.name();
            let progress = format!("[{}/{}]", idx + 1, self.tasks.len());
            
            // 1. Skip check
            if !task.should_run(ctx) {
                logging::info(&task_name, &format!("{} Skipped (condition not met)", progress), &[]);
                continue;
            }

            // 2. Incremental check
            if let Some(fp) = task.fingerprint(ctx)? {
                let state = ctx.state.read().unwrap();
                if state.get_hash(&task_name) == Some(&fp) {
                    logging::info(&task_name, &format!("{} Up-to-date (cached)", progress), &[]);
                    continue;
                }
            }

            // 3. Execution
            logging::info(&task_name, &format!("{} Running...", progress), &[]);
            let task_start = Instant::now();
            
            match task.run(ctx) {
                Ok(TaskStatus::Success) => {
                    let elapsed = task_start.elapsed();
                    logging::success(&task_name, &format!("{} Done in {:?}", progress, elapsed), &[]);
                    executed_tasks.push(task);
                    
                    // Update Cache
                    if let Some(fp) = task.fingerprint(ctx)? {
                        let mut state = ctx.state.write().unwrap();
                        state.set_hash(task_name, fp);
                        let _ = state.save();
                    }
                },
                Ok(TaskStatus::Skipped(reason)) => {
                    logging::info(&task_name, &format!("{} Skipped: {}", progress, reason), &[]);
                },
                Ok(TaskStatus::Failed(reason)) => {
                    logging::error(&task_name, &format!("{} Failed: {}", progress, reason), &[]);
                    final_result = Err(anyhow!("Task '{}' failed: {}", task_name, reason));
                    break;
                },
                Err(e) => {
                    logging::error(&task_name, &format!("{} Crashed: {}", progress, e), &[]);
                    final_result = Err(e);
                    break;
                }
            }
        }

        // Cleanup on failure
        if final_result.is_err() {
            self.initiate_rollback(ctx, &executed_tasks);
        } else {
            logging::success("PIPELINE", &format!("Workflow '{}' completed in {:?}", self.name, pipeline_start.elapsed()), &[]);
        }

        final_result
    }

    fn initiate_rollback(&self, ctx: &ExecutionContext, executed: &[&Box<dyn Task>]) {
        logging::warn("PIPELINE", "Initiating rollback/cleanup sequence...", &[]);
        for task in executed.iter().rev() {
            let task_name = task.name();
            if let Err(e) = task.cleanup(ctx) {
                logging::error(&task_name, &format!("Cleanup failed: {}", e), &[]);
            }
        }
    }
}
