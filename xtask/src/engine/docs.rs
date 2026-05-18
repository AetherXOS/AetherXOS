use crate::engine::{ExecutionContext, Task, TaskStatus};
use crate::utils::logging;
use anyhow::{Context, Result};
use std::fs;

pub struct DocsGenerateTask {
    pub source_dir: String,
    pub output_file: String,
}

impl Task for DocsGenerateTask {
    fn name(&self) -> String {
        "API Documentation Generation".to_string()
    }
    fn description(&self) -> String {
        "Scans source code for doc comments and generates a markdown reference".to_string()
    }

    fn run(&self, ctx: &ExecutionContext) -> Result<TaskStatus> {
        logging::status(
            "DOCS",
            &format!("Generating documentation for {}...", self.source_dir),
        );
        let mut markdown = format!(
            "# AetherX OS API Reference\n\nGenerated on: {}\n\n",
            chrono::Utc::now()
        );

        let src_path = ctx.repo_root.join(&self.source_dir);
        if !src_path.exists() {
            return Ok(TaskStatus::Failed(format!(
                "Source directory not found: {}",
                src_path.display()
            )));
        }

        for entry in walkdir::WalkDir::new(src_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
        {
            let content = fs::read_to_string(entry.path())?;
            let rel_path = entry
                .path()
                .strip_prefix(&ctx.repo_root)
                .unwrap_or(entry.path());

            let mut file_has_docs = false;
            let mut file_docs = format!("## File: `{}`\n\n", rel_path.display());

            for line in content.lines() {
                if line.trim().starts_with("///") {
                    file_has_docs = true;
                    file_docs.push_str(&format!(
                        "{}\n",
                        line.trim().trim_start_matches("///").trim()
                    ));
                }
            }

            if file_has_docs {
                markdown.push_str(&file_docs);
                markdown.push_str("\n---\n\n");
            }
        }

        let out_path = ctx.artifact_path(&self.output_file);
        fs::write(&out_path, markdown).context("Failed to write documentation")?;

        logging::ready(
            "DOCS",
            "Documentation generated successfully",
            &out_path.to_string_lossy(),
            &[],
        );
        Ok(TaskStatus::Success)
    }
}
