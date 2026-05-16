use anyhow::Result;
use crate::engine::{Task, ExecutionContext, TaskStatus, Pipeline};

pub struct PipelineHelpTask {
    pub pipeline_name: String,
    pub task_descriptions: Vec<(String, String)>,
}

impl Task for PipelineHelpTask {
    fn name(&self) -> String { "Workflow Intelligence".to_string() }
    fn description(&self) -> String { "Provides detailed insights and documentation for the current workflow".to_string() }
    
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
        .map(|t| (t.name(), t.description()))
        .collect();
    PipelineHelpTask {
        pipeline_name: pipeline.name.clone(),
        task_descriptions,
    }
}
