use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use crate::engine::{Task, ExecutionContext, TaskStatus};
use crate::utils::logging;

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildManifest {
    pub arch: String,
    pub is_release: bool,
    pub features: Vec<String>,
    pub timestamp: String,
}

pub struct ManifestExportTask {
    pub output_path: PathBuf,
}

impl Task for ManifestExportTask {
    fn name(&self) -> String { "Build Manifest Export".to_string() }
    fn description(&self) -> String { "Saves the current build configuration to a JSON file for traceability and reproducibility".to_string() }
    
    fn run(&self, ctx: &ExecutionContext) -> Result<TaskStatus> {
        let manifest = BuildManifest {
            arch: ctx.arch.clone(),
            is_release: ctx.is_release,
            features: ctx.features.clone(),
            timestamp: chrono::Local::now().to_rfc3339(),
        };

        let json = serde_json::to_string_pretty(&manifest)
            .context("Failed to serialize build manifest")?;
            
        std::fs::write(&self.output_path, json)
            .context("Failed to write build manifest to disk")?;
            
        logging::info("MANIFEST", "Build configuration exported", &[("path", &self.output_path.to_string_lossy())]);
        Ok(TaskStatus::Success)
    }
}
