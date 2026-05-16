use anyhow::Result;
use crate::engine::{Pipeline, ExecutionContext};
use crate::commands::infra::build::tasks::{KernelCompileTask, InitramfsTask};
use crate::commands::infra::build::image_tasks::{KernelStageTask, BootConfigTask, ImageFinalizeTask};
use crate::commands::validation::safety::KernelSafetyAuditTask;

pub struct WorkflowRegistry;

impl WorkflowRegistry {
    pub fn full_iso_pipeline(ctx: &ExecutionContext) -> Result<Pipeline> {
        let mut p = Pipeline::new("Full ISO Pipeline");
        
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

        Ok(p)
    }
}
