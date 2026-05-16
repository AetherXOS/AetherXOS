use anyhow::{Result, Context};
use std::path::PathBuf;
use crate::engine::{Task, ExecutionContext, task::TaskStatus, Op};
use crate::utils::{logging, HashAlgo};
use crate::utils::fs::registry::DistroImage;

pub struct DistroBuildTask {
    pub name: String,
    pub image: DistroImage,
    pub staging_area: PathBuf,
}

impl Task for DistroBuildTask {
    fn name(&self) -> &str { "Distro Base Preparation" }
    fn description(&self) -> &str { "Acquires and extracts the base distribution rootfs" }
    
    fn run(&self, ctx: &ExecutionContext) -> Result<TaskStatus> {
        let cache_dir = ctx.out_dir.join("guest_cache");
        let url = self.image.url();
        let filename = url.split('/').last().unwrap_or("rootfs.tar.xz");
        let archive_path = cache_dir.join(filename);
        
        // 1. Download base image
        let hashes = self.image.hashes();
        let sha256 = hashes.get(&HashAlgo::Sha256).cloned();
        
        let download = crate::engine::DownloadTask {
            url: url.to_string(),
            dest: archive_path.clone(),
            expected_hash: sha256.map(|s| (HashAlgo::Sha256, s)),
        };
        download.run(ctx)?;

        // 2. Extract to staging
        logging::status("DISTRO", &format!("Extracting {} base to staging...", self.name));
        Op::clean_dir(&self.staging_area, "DISTRO")?;
        
        crate::commands::infra::build::rootfs::extract_rootfs_archive(&archive_path, &self.staging_area)
            .context("Failed to extract distro rootfs")?;
            
        Ok(TaskStatus::Success)
    }
}
