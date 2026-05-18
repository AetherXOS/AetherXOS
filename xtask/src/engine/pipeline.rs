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
        let is_tui = crate::utils::core::config::get_settings().tui_hud_enabled;
        if is_tui {
            return self.run_tui_hud(ctx);
        }

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
            // 3. Execution
            let result = crate::utils::ui::logging::aop_wrap(&task_name, &format!("{} {}", progress, task.description()), || {
                task.run(ctx)
            });
            
            match result {
                Ok(TaskStatus::Success) => {
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

    fn run_tui_hud(&self, ctx: &ExecutionContext) -> Result<()> {
        use crate::utils::ui::pipeline_hud::{HudEvent, run_hud, HudResult};
        use crate::utils::sys::execution::TUI_HUD_LOG_SENDER;
        use std::time::Duration;

        loop {
            let (event_tx, event_rx) = crossbeam_channel::unbounded();
            let (log_tx, log_rx) = crossbeam_channel::unbounded();

            // 1. Set global log redirector
            *TUI_HUD_LOG_SENDER.lock().unwrap() = Some(log_tx);

            let task_names: Vec<(String, String)> = self.tasks.iter()
                .map(|t| (t.name().to_string(), t.description().to_string()))
                .collect();

            let mut final_result = Ok(());
            let mut retry_flag = false;

            std::thread::scope(|s| {
                let event_tx_clone = event_tx.clone();
                let event_tx_clone2 = event_tx.clone();

                // Background pipeline executor thread
                let runner = s.spawn(move || {
                    let mut executed_tasks = Vec::new();
                    let mut pipeline_res = Ok(());

                    for (idx, task) in self.tasks.iter().enumerate() {
                        let task_name = task.name();
                        
                        if !task.should_run(ctx) {
                            continue;
                        }

                        if let Some(fp) = task.fingerprint(ctx).unwrap_or(None) {
                            let state = ctx.state.read().unwrap();
                            if state.get_hash(&task_name) == Some(&fp) {
                                continue;
                            }
                        }

                        let _ = event_tx_clone.send(HudEvent::TaskStarted { index: idx });

                        let result = task.run(ctx);

                        match result {
                            Ok(TaskStatus::Success) => {
                                executed_tasks.push(task);
                                if let Some(fp) = task.fingerprint(ctx).unwrap_or(None) {
                                    let mut state = ctx.state.write().unwrap();
                                    state.set_hash(task_name, fp);
                                    let _ = state.save();
                                }
                                let _ = event_tx_clone.send(HudEvent::TaskFinished { index: idx, success: true, reason: None });
                            }
                            Ok(TaskStatus::Skipped(reason)) => {
                                let _ = event_tx_clone.send(HudEvent::TaskFinished { index: idx, success: true, reason: Some(reason) });
                            }
                            Ok(TaskStatus::Failed(reason)) => {
                                let _ = event_tx_clone.send(HudEvent::TaskFinished { index: idx, success: false, reason: Some(reason.clone()) });
                                pipeline_res = Err(anyhow!("Task '{}' failed: {}", task_name, reason));
                                break;
                            }
                            Err(e) => {
                                let _ = event_tx_clone.send(HudEvent::TaskFinished { index: idx, success: false, reason: Some(e.to_string()) });
                                pipeline_res = Err(e);
                                break;
                            }
                        }
                    }

                    if pipeline_res.is_err() {
                        self.initiate_rollback(ctx, &executed_tasks);
                    }
                    
                    let success = pipeline_res.is_ok();
                    let _ = event_tx_clone.send(HudEvent::Finished { success });
                    pipeline_res
                });

                // Bridge thread to forward executor logs to HUD
                s.spawn(move || {
                    while let Ok(line) = log_rx.recv() {
                        let _ = event_tx_clone2.send(HudEvent::TaskLog { line });
                    }
                });

                // Run TUI HUD in the main thread (blocks until HUD loop exits)
                match run_hud(event_rx, task_names) {
                    HudResult::Retry => {
                        retry_flag = true;
                    }
                    HudResult::Exit(res) => {
                        final_result = res;
                    }
                }

                // Wait for runner thread and capture final result if not retrying
                let thread_res = runner.join().unwrap_or_else(|_| Err(anyhow!("Pipeline thread crashed")));
                if !retry_flag && final_result.is_ok() {
                    final_result = thread_res;
                }
            });

            // Clean up log redirector
            *TUI_HUD_LOG_SENDER.lock().unwrap() = None;

            if retry_flag {
                println!("\n🔄  Retrying build workflow now...\n");
                std::thread::sleep(Duration::from_millis(500));
                continue;
            }

            return final_result;
        }
    }
}
