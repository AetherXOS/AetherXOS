use anyhow::{Result, anyhow};
use std::collections::{HashMap, HashSet};
use crate::engine::{Task, ExecutionContext, task::TaskStatus};
use crate::utils::logging;

pub struct TaskNode {
    pub task: Box<dyn Task>,
    pub dependencies: Vec<String>,
}

pub struct DagPipeline {
    pub name: String,
    pub nodes: HashMap<String, TaskNode>,
}

impl DagPipeline {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            nodes: HashMap::new(),
        }
    }

    pub fn add_task(&mut self, task: Box<dyn Task>, deps: Vec<&str>) {
        let name = task.name().to_string();
        self.nodes.insert(name, TaskNode {
            task,
            dependencies: deps.iter().map(|&s| s.to_string()).collect(),
        });
    }

    pub fn run(&self, ctx: &ExecutionContext) -> Result<()> {
        logging::status("DAG", &format!("Executing DAG Pipeline: {}", self.name));
        
        let order = self.resolve_order()?;
        
        for task_name in order {
            let node = self.nodes.get(&task_name).unwrap();
            
            if ctx.dry_run {
                logging::info("DRY-RUN", &format!("Would execute task: {}", task_name), &[]);
                continue;
            }

            loop {
                let start = std::time::Instant::now();
                logging::set_current_task(Some(task_name.clone()));
                logging::status("TASK", &format!("Running: {}", task_name));
                let status = node.task.run(ctx)?;
                let duration = start.elapsed();
                
                // Record metrics for the timeline
                crate::utils::ui::telemetry::Telemetry::record(&task_name, duration);
                
                match status {
                    TaskStatus::Success => {
                        logging::success("TASK", &format!("Completed: {}", task_name), &[]);
                        break;
                    }
                    TaskStatus::Skipped(_) => {
                        logging::info("TASK", &format!("Skipped: {}", task_name), &[]);
                        break;
                    }
                    TaskStatus::Failed(e) => {
                        if ctx.non_interactive {
                            return Err(anyhow!("Task '{}' failed: {}", task_name, e));
                        }
                        
                        logging::error("DEBUGGER", &format!("Task '{}' failed: {}", task_name, e), &[]);
                        let options = vec!["Retry", "Spwan Debug Shell", "Abort"];
                        let selection = inquire::Select::new("Action on Failure:", options).prompt()?;
                        
                        match selection {
                            "Retry" => continue,
                            "Spwan Debug Shell" => {
                                logging::status("SHELL", "Spawning diagnostic shell. Type 'exit' to return.");
                                let _ = std::process::Command::new("powershell").spawn()?.wait();
                                continue;
                            }
                            _ => return Err(anyhow!("Task '{}' aborted by user", task_name)),
                        }
                    }
                }
            }
        }
        
        logging::set_current_task(None);
        Ok(())
    }

    fn resolve_order(&self) -> Result<Vec<String>> {
        let mut order = Vec::new();
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();

        for name in self.nodes.keys() {
            self.visit(name, &mut visited, &mut visiting, &mut order)?;
        }

        Ok(order)
    }

    fn visit(&self, name: &str, visited: &mut HashSet<String>, visiting: &mut HashSet<String>, order: &mut Vec<String>) -> Result<()> {
        if visiting.contains(name) {
            return Err(anyhow!("Circular dependency detected at task: {}", name));
        }
        if visited.contains(name) {
            return Ok(());
        }

        visiting.insert(name.to_string());

        if let Some(node) = self.nodes.get(name) {
            for dep in &node.dependencies {
                self.visit(dep, visited, visiting, order)?;
            }
        }

        visiting.remove(name);
        visited.insert(name.to_string());
        order.push(name.to_string());

        Ok(())
    }

    pub fn to_mermaid(&self) -> String {
        let mut graph = vec!["graph TD".to_string()];
        for (name, node) in &self.nodes {
            let sanitized_name = name.replace(' ', "_");
            for dep in &node.dependencies {
                let sanitized_dep = dep.replace(' ', "_");
                graph.push(format!("    {} --> {}", sanitized_dep, sanitized_name));
            }
            if node.dependencies.is_empty() {
                graph.push(format!("    {}", sanitized_name));
            }
        }
        
        // Highlight current task
        if let Ok(task_lock) = crate::utils::ui::logging::CURRENT_TASK.lock() {
            if let Some(ref current) = *task_lock {
                let sanitized_current = current.replace(' ', "_");
                graph.push(format!("    style {} fill:#00ffcc,stroke:#333,stroke-width:4px", sanitized_current));
            }
        }
        
        graph.join("\n")
    }

    pub fn to_dot(&self) -> String {
        let mut dot = vec!["digraph G {".to_string(), "    node [shape=box];".to_string()];
        for (name, node) in &self.nodes {
            for dep in &node.dependencies {
                dot.push(format!("    \"{}\" -> \"{}\";", dep, name));
            }
        }
        dot.push("}".to_string());
        dot.join("\n")
    }
}
