use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::thread;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use crate::engine::{Task, ExecutionContext, task::TaskStatus};
use crate::utils::logging;

pub struct ParallelPipeline {
    pub name: String,
    pub tasks: Vec<Arc<dyn Task>>,
}

impl ParallelPipeline {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            tasks: Vec::new(),
        }
    }

    pub fn add_task(mut self, task: Arc<dyn Task>) -> Self {
        self.tasks.push(task);
        self
    }

    pub fn run(&self, ctx: Arc<ExecutionContext>) -> Result<()> {
        logging::status("PARALLEL", &format!("Launching parallel workflow: {}", self.name));
        
        let mp = MultiProgress::new();
        let style = ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {msg} [{elapsed_precise}]")
            .unwrap();

        let mut handles = Vec::new();
        let errors = Arc::new(Mutex::new(Vec::new()));

        for task in &self.tasks {
            let task = Arc::clone(task);
            let ctx = Arc::clone(&ctx);
            
            let pb = mp.add(ProgressBar::new_spinner());
            pb.set_style(style.clone());
            pb.set_prefix(task.name().to_string());
            pb.enable_steady_tick(std::time::Duration::from_millis(100));

            let err_clone = Arc::clone(&errors);

            let handle = thread::spawn(move || {
                pb.set_message("Executing...");
                match task.run(&ctx) {
                    Ok(TaskStatus::Success) => pb.finish_with_message("Done"),
                    Ok(TaskStatus::Skipped(reason)) => pb.finish_with_message(format!("Skipped: {}", reason)),
                    Ok(TaskStatus::Failed(reason)) => {
                        pb.abandon_with_message(format!("Failed: {}", reason));
                        err_clone.lock().unwrap().push(anyhow::anyhow!("Task '{}' failed: {}", task.name(), reason));
                    }
                    Err(e) => {
                        pb.abandon_with_message(format!("Error: {}", e));
                        err_clone.lock().unwrap().push(e);
                    }
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.join();
        }

        let errs = errors.lock().unwrap();
        if !errs.is_empty() {
            anyhow::bail!("Parallel workflow failed with {} errors", errs.len());
        }

        logging::success("PARALLEL", &format!("Workflow {} completed", self.name), &[]);
        Ok(())
    }
}
