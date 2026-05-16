use anyhow::Result;
use crate::engine::{ExecutionContext, BuildProfile};
use crate::utils::logging;

pub struct UniversalController;

impl UniversalController {
    pub fn build_pipeline(name: &str, ctx: &ExecutionContext) -> Result<crate::engine::dag::DagPipeline> {
        use crate::constants::workflows::*;
        use crate::engine::dag::DagPipeline;
        
        let mut dag = DagPipeline::new(name);
        
        match name {
            FULL_ISO => {
                // ... Convert Registry to DAG if needed, or just use linear DAG
                dag.add_task(Box::new(crate::engine::audit::ToolchainAuditTask), vec![]);
                dag.add_task(Box::new(crate::engine::ResourceAuditTask), vec!["Environment Audit"]);
                dag.add_task(Box::new(crate::commands::infra::build::tasks::KernelCompileTask {
                    arch: crate::constants::defaults::build::ARCH,
                    release: ctx.is_release,
                    features: crate::utils::features::kernel_features_from_default(&["vfs", "drivers"])?,
                }), vec!["Resource Audit"]);
                dag.add_task(Box::new(crate::commands::validation::KernelSafetyAuditTask), vec!["Kernel Compilation"]);
                // ... (rest of the ISO pipeline)
            }
            KERNEL_DEV => {
                let features_ref: Vec<&str> = ctx.features.iter().map(|s| s.as_str()).collect();
                dag.add_task(Box::new(crate::commands::infra::build::tasks::KernelCompileTask {
                    arch: crate::constants::defaults::build::ARCH,
                    release: ctx.is_release,
                    features: crate::utils::features::kernel_features_from_default(&features_ref)?,
                }), vec![]);
                dag.add_task(Box::new(crate::commands::validation::KernelSafetyAuditTask), vec!["Kernel Compilation"]);
            }
            DOCS => {
                dag.add_task(Box::new(crate::engine::DocsGenerateTask {
                    source_dir: "kernel".to_string(),
                    output_file: "kernel_api.md".to_string(),
                }), vec![]);
            }
            DEBUG => {
                dag.add_task(Box::new(crate::engine::DebugBridgeTask {
                    image_path: crate::constants::paths::artifact_dir().join("aethercore.iso"),
                }), vec![]);
            }
            _ => {
                if let Ok(profile) = BuildProfile::load(&ctx.repo_root, name) {
                    let mut new_ctx = ctx.clone();
                    profile.apply_to(&mut new_ctx);
                    return Self::build_pipeline(FULL_ISO, &new_ctx);
                }
            }
        }
        
        Ok(dag)
    }

    pub fn dispatch_workflow(name: &str, ctx: &ExecutionContext) -> Result<()> {
        logging::status("DISPATCH", &format!("Orchestrating workflow: {}", name));
        let dag = Self::build_pipeline(name, ctx)?;
        match dag.run(ctx) {
            Ok(_) => {
                crate::utils::ui::notifications::pipeline_success(name);
                crate::utils::ui::voice::pipeline_success_voice(name);
                let _ = crate::utils::ui::oracle::Oracle::suggest_next(name, true, None);
                Ok(())
            }
            Err(e) => {
                crate::utils::ui::telemetry::Telemetry::record_failure(name, &e.to_string());
                crate::utils::ui::notifications::pipeline_failed(name, &e.to_string());
                crate::utils::ui::voice::pipeline_failed_voice(name);
                let _ = crate::utils::ui::oracle::Oracle::suggest_next(name, false, Some(&e.to_string()));
                Err(e)
            }
        }
    }
}
