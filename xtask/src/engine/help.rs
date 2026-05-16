use anyhow::Result;
use crate::engine::{Task, ExecutionContext, task::TaskStatus, Pipeline};

pub struct PipelineHelpTask {
    pub pipeline_name: String,
    pub task_descriptions: Vec<(String, String)>,
}

impl Task for PipelineHelpTask {
    fn name(&self) -> &str { "Workflow Intelligence" }
    fn description(&self) -> &str { "Provides detailed insights and documentation for the current workflow" }
    
    fn run(&self, _ctx: &ExecutionContext) -> Result<TaskStatus> {
        println!("\n📖 Workflow Intelligence: {}", self.pipeline_name);
        println!("========================================");
        for (name, desc) in &self.task_descriptions {
            println!("🔹 {}: {}", name, desc);
        }
        println!("========================================\n");
        Ok(TaskStatus::Success)
    }
}

pub fn generate_help(pipeline: &Pipeline) -> PipelineHelpTask {
    let task_descriptions = pipeline.tasks.iter()
        .map(|t| (t.name().to_string(), t.description().to_string()))
        .collect();
    PipelineHelpTask {
        pipeline_name: pipeline.name.clone(),
        task_descriptions,
    }
}
