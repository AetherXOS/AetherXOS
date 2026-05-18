use crate::commands::infra::build::image_tasks::{
    BootConfigTask, ImageFinalizeTask, KernelStageTask,
};
use crate::commands::infra::build::tasks::{InitramfsTask, KernelCompileTask};
use crate::commands::validation::safety::KernelSafetyAuditTask;
use crate::engine::{ExecutionContext, Pipeline};
use anyhow::Result;

pub struct WorkflowRegistry;

impl WorkflowRegistry {
    pub fn full_iso_pipeline(ctx: &ExecutionContext) -> Result<Pipeline> {
        let mut p = Pipeline::new("Full ISO Pipeline");

        // 0. Environment Audit
        p = p.add_task(Box::new(crate::engine::audit::ToolchainAuditTask));
        p = p.add_task(Box::new(crate::engine::ResourceAuditTask));

        // 1. Compile Kernel
        p = p.add_task(Box::new(KernelCompileTask {
            arch: crate::constants::defaults::build::ARCH,
            release: ctx.is_release,
            features: crate::utils::features::kernel_features_from_default(&["vfs", "drivers"])?,
        }));

        // 2. Safety Audit
        p = p.add_task(Box::new(KernelSafetyAuditTask));

        // 3. Initramfs
        p = p.add_task(Box::new(InitramfsTask));

        // 4. Staging & Bundling
        p = p.add_task(Box::new(KernelStageTask));

        p = p.add_task(Box::new(BootConfigTask {
            bootloader: crate::types::Bootloader::Limine,
        }));

        p = p.add_task(Box::new(ImageFinalizeTask {
            format: crate::types::ImageFormat::Iso,
        }));

        // 5. Documentation
        p = p.add_task(Box::new(crate::engine::docs::DocsGenerateTask {
            source_dir: "kernel".to_string(),
            output_file: "kernel_api.md".to_string(),
        }));

        Ok(p)
    }
}
