use anyhow::Result;
use std::path::PathBuf;
use crate::engine::{Task, ExecutionContext, task::TaskStatus};
use crate::utils::{logging, net, paths, HashAlgo};

pub struct DownloadTask {
    pub url: String,
    pub dest: PathBuf,
    pub expected_hash: Option<(HashAlgo, String)>,
}

impl Task for DownloadTask {
    fn name(&self) -> &str { "File Download" }
    fn description(&self) -> &str { "Downloads a remote resource with integrity checks" }
    
    fn run(&self, _ctx: &ExecutionContext) -> Result<TaskStatus> {
        if self.dest.exists() {
            if self.verify_integrity()? {
                return Ok(TaskStatus::Skipped("Local file is valid".to_string()));
            }
            std::fs::remove_file(&self.dest)?;
        }

        paths::ensure_dir(self.dest.parent().unwrap())?;
        logging::info("DOWNLOAD", "Starting download", &[("url", &self.url)]);
        net::download_with_configured_retries(&self.url, &self.dest)?;
        
        if !self.verify_integrity()? {
            return Ok(TaskStatus::Failed("Integrity check failed after download".to_string()));
        }
        
        Ok(TaskStatus::Success)
    }
}

impl DownloadTask {
    fn verify_integrity(&self) -> Result<bool> {
        if let Some((algo, expected)) = &self.expected_hash {
            let actual = crate::utils::hash_file(&self.dest, *algo)?;
            if actual != *expected {
                logging::warn("DOWNLOAD", "Hash mismatch", &[("expected", expected), ("actual", &actual)]);
                return Ok(false);
            }
        }
        Ok(true)
    }
}
