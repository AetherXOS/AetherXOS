use crate::engine::{ExecutionContext, Task, TaskStatus};
use crate::utils::logging;
use anyhow::{Context, Result};

pub struct DownloadTask {
    pub url: String,
    pub dest: std::path::PathBuf,
}

impl Task for DownloadTask {
    fn name(&self) -> String {
        "Network Resource Acquisition".to_string()
    }
    fn description(&self) -> String {
        "Downloads external dependencies or artifacts from a remote URL".to_string()
    }

    fn run(&self, _ctx: &ExecutionContext) -> Result<TaskStatus> {
        logging::status("NET", &format!("Downloading {}...", self.url));
        let tool = if cfg!(windows) { "curl.exe" } else { "curl" };

        crate::utils::sys::process::Executor::new(tool)
            .args(&[
                "-fsSL",
                "--progress-bar",
                "-o",
                &self.dest.to_string_lossy(),
                &self.url,
            ])
            .run()
            .with_context(|| format!("Failed to download {}", self.url))?;

        Ok(TaskStatus::Success)
    }
}
