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

            logging::status("TASK", &format!("Running: {}", task_name));
            let status = node.task.run(ctx)?;
            
            match status {
                TaskStatus::Success => logging::success("TASK", &format!("Completed: {}", task_name), &[]),
                TaskStatus::Skipped(_) => logging::info("TASK", &format!("Skipped: {}", task_name), &[]),
                TaskStatus::Failed(e) => return Err(anyhow!("Task '{}' failed: {}", task_name, e)),
            }
        }

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
}
