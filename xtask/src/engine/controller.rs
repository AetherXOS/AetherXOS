use anyhow::Result;
use crate::engine::{Pipeline, ExecutionContext, WorkflowRegistry, BuildProfile};
use crate::utils::logging;

pub struct UniversalController;

impl UniversalController {
    pub fn dispatch_workflow(name: &str, ctx: &ExecutionContext) -> Result<()> {
        use crate::constants::workflows::*;
        
        logging::status("DISPATCH", &format!("Orchestrating workflow: {}", name));
        
        let pipeline = match name {
            FULL_ISO => WorkflowRegistry::full_iso_pipeline(ctx)?,
            KERNEL_DEV => {
                let mut p = Pipeline::new(KERNEL_DEV);
                let features_ref: Vec<&str> = ctx.features.iter().map(|s| s.as_str()).collect();
                p = p.add_task(Box::new(crate::commands::infra::build::tasks::KernelCompileTask {
                    arch: crate::constants::defaults::build::ARCH,
                    release: ctx.is_release,
                    features: crate::utils::features::kernel_features_from_default(&features_ref)?,
                }));
                p = p.add_task(Box::new(crate::commands::validation::KernelSafetyAuditTask));
                p
            }
            DOCS => {
                let mut p = Pipeline::new(DOCS);
                p = p.add_task(Box::new(crate::engine::DocsGenerateTask {
                    source_dir: "kernel".to_string(),
                    output_file: "kernel_api.md".to_string(),
                }));
                p
            }
            DEBUG => {
                let mut p = Pipeline::new(DEBUG);
                p = p.add_task(Box::new(crate::engine::DebugBridgeTask {
                    image_path: crate::constants::paths::artifact_dir().join("aethercore.iso"),
                }));
                p
            }
            _ => {
                if let Ok(profile) = BuildProfile::load(&ctx.repo_root, name) {
                    logging::info("CONTROLLER", "Detected profile name, loading...", &[]);
                    let mut new_ctx = ctx.clone();
                    profile.apply_to(&mut new_ctx);
                    return Self::dispatch_workflow(FULL_ISO, &new_ctx);
                }
                Pipeline::new("Custom / Unknown")
            }
        };

        pipeline.run(ctx)?;
        Ok(())
    }
}
