use crate::engine::{ExecutionContext, Op, Task, TaskStatus};
use crate::utils::fs::registry::DistroImage;
use crate::utils::logging;
use anyhow::{Context, Result};
use std::path::PathBuf;

pub struct DistroBuildTask {
    pub name: String,
    pub image: DistroImage,
    pub staging_area: PathBuf,
}

impl Task for DistroBuildTask {
    fn name(&self) -> String {
        format!("Distro Preparation: {}", self.name)
    }
    fn description(&self) -> String {
        format!("Acquires and extracts the {} base rootfs", self.name)
    }

    fn run(&self, ctx: &ExecutionContext) -> Result<TaskStatus> {
        let cache_dir = ctx.out_dir.join("guest_cache");
        let url = self.image.url();
        let filename = url.split('/').next_back().unwrap_or("rootfs.tar.xz");
        let archive_path = cache_dir.join(filename);

        let download = crate::engine::DownloadTask {
            url: url.to_string(),
            dest: archive_path.clone(),
        };
        download.run(ctx)?;

        logging::status(
            "DISTRO",
            &format!("Extracting {} base to staging...", self.name),
        );
        Op::clean_dir(&self.staging_area, "DISTRO")?;

        crate::commands::infra::build::rootfs::extract_rootfs_archive(
            &archive_path,
            &self.staging_area,
        )
        .context("Failed to extract distro rootfs")?;

        Ok(TaskStatus::Success)
    }
}
