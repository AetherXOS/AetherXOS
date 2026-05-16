use anyhow::Result;
use super::task::{Task, TaskStatus};
use super::context::ExecutionContext;
use crate::utils::logging;

/// A sequence of tasks that form a high-level workflow.
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

    pub fn run(&self, ctx: &ExecutionContext) -> Result<()> {
        logging::status("PIPELINE", &format!("Starting workflow: {}", self.name));
        
        let mut executed_tasks = Vec::new();
        let mut result = Ok(());
        let pipeline_start = std::time::Instant::now();

        for (i, task) in self.tasks.iter().enumerate() {
            logging::step("PIPELINE", &format!("[{}/{}] {}", i + 1, self.tasks.len(), task.name()));
            
            if !task.should_run(ctx) {
                logging::info("PIPELINE", "Task is up-to-date, skipping", &[("task", task.name())]);
                continue;
            }

            let task_start = std::time::Instant::now();

            match task.run(ctx) {
                Ok(TaskStatus::Success) => {
                    executed_tasks.push(task);
                    let elapsed = task_start.elapsed();
                    logging::info("PIPELINE", "Task completed", &[
                        ("task", task.name()), 
                        ("duration", &format!("{:.2}s", elapsed.as_secs_f32()))
                    ]);
                    // Save state after each task for resilience
                    let _ = ctx.state.save();
                }
                Ok(TaskStatus::Skipped(reason)) => {
                    logging::info("PIPELINE", "Task skipped by logic", &[("task", task.name()), ("reason", &reason)]);
                }
                Ok(TaskStatus::Failed(reason)) => {
                    logging::error("PIPELINE", "Task reported failure", &[("task", task.name()), ("reason", &reason)]);
                    result = Err(anyhow::anyhow!("Task {} failed: {}", task.name(), reason));
                    break;
                }
                Err(e) => {
                    logging::error("PIPELINE", "Task crashed with error", &[("task", task.name()), ("error", &e.to_string())]);
                    result = Err(e);
                    break;
                }
            }
        }

        if result.is_err() {
            logging::warn("PIPELINE", "Workflow failed, initiating cleanup...", &[]);
            for task in executed_tasks.iter().rev() {
                if let Err(cleanup_err) = task.cleanup(ctx) {
                    logging::error("PIPELINE", "Cleanup failed for task", &[("task", task.name()), ("error", &cleanup_err.to_string())]);
                }
            }
        } else {
            let total_elapsed = pipeline_start.elapsed();
            logging::success("PIPELINE", &format!("Workflow {} completed in {:.2}s", self.name, total_elapsed.as_secs_f32()), &[]);
        }

        result
    }
}
