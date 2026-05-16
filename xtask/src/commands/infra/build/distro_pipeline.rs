use anyhow::Result;
use crate::engine::{Pipeline, ExecutionContext, DownloadTask};
use crate::commands::infra::build::tasks::{KernelCompileTask, InitramfsTask};
use aethercore_common::{TargetArch, KernelFeatures};
use std::path::PathBuf;

pub fn create_distro_pipeline(
    distro: String,
    url: String,
    arch: TargetArch,
    features: KernelFeatures,
    out_iso: PathBuf,
) -> Pipeline {
    Pipeline::new(format!("Distro ISO: {}", distro))
        .add_task(Box::new(KernelCompileTask {
            arch,
            release: false,
            features,
        }))
        .add_task(Box::new(DownloadTask {
            url,
            dest: crate::utils::core::context::out_dir().join("guest_cache").join("rootfs.tar.xz"),
            expected_hash: None, // Can be improved with registry lookup
        }))
        .add_task(Box::new(InitramfsTask))
        // Add more tasks like ExtractRootfsTask, LimineSetupTask, FinalizeIsoTask
}
