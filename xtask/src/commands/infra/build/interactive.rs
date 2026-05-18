use aethercore_common::TargetArch;
use anyhow::Result;
use strum::IntoEnumIterator;

use crate::cli::{Bootloader, ImageFormat};
use crate::commands::infra::build::image_tasks::{
    BootConfigTask, ImageFinalizeTask, KernelStageTask,
};
use crate::commands::infra::build::tasks::{InitramfsTask, KernelCompileTask};
use crate::commands::validation::safety::KernelSafetyAuditTask;
use crate::engine::{ExecutionContext, Pipeline, StagingArea};
use crate::utils::{features, logging, ui};

#[derive(Clone, Copy)]
enum BuildMode {
    KernelOnly,
    FullPipeline,
}

impl core::fmt::Display for BuildMode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::KernelOnly => write!(f, "Kernel only"),
            Self::FullPipeline => write!(f, "Full pipeline (kernel + initramfs + image)"),
        }
    }
}

pub fn run() -> Result<()> {
    logging::info(
        "build::interactive",
        "Initializing Interactive Masterpiece Wizard",
        &[],
    );

    let modes = [BuildMode::KernelOnly, BuildMode::FullPipeline];
    let mode = *ui::select("Select build mode", &modes)?;

    let arches: Vec<TargetArch> = TargetArch::supported().collect();
    let arch = *ui::select("Select target architecture", &arches)?;

    let profile_choices = ["debug", "release"];
    let profile = *ui::select("Select build profile", &profile_choices)?;
    let release = profile == "release";

    let resolved_features = features::prompt_kernel_feature_selection("Build", &[])?;

    let mut pipeline = Pipeline::new("Interactive Build Workflow");

    // Create execution context
    let mut ctx = ExecutionContext {
        repo_root: crate::utils::core::context::repo_root(),
        out_dir: crate::utils::core::context::out_dir(),
        is_release: release,
        arch: arch.to_string(),
        features: resolved_features
            .to_cargo_features()
            .iter()
            .map(|&s| s.to_string())
            .collect(),
        staging: None,
        state: std::sync::Arc::new(std::sync::RwLock::new(crate::engine::EngineState::load())),
        non_interactive: false,
        dry_run: false,
        parameters: std::collections::HashMap::new(),
    };

    // Add core compilation task
    pipeline = pipeline.add_task(Box::new(KernelCompileTask {
        arch,
        release,
        features: resolved_features,
    }));

    // Add safety audit for production-ready code
    pipeline = pipeline.add_task(Box::new(KernelSafetyAuditTask));

    if matches!(mode, BuildMode::FullPipeline) {
        let bootloader = *ui::select("Select bootloader", &Bootloader::iter().collect::<Vec<_>>())?;
        let format = *ui::select(
            "Select image format",
            &ImageFormat::iter().collect::<Vec<_>>(),
        )?;

        let distros = vec![
            "almalinux",
            "alpine",
            "archlinux",
            "debian",
            "fedora",
            "opensuse",
            "rockylinux",
            "none",
        ];
        let distro = *ui::select("Select Target Distro Integration", &distros)?;
        if distro != "none" {
            ctx.parameters
                .insert("distro".to_string(), distro.to_string());
        }

        // Initialize staging for full pipeline
        ctx.staging = Some(StagingArea::new(ctx.out_dir.join("stage"))?);

        pipeline = pipeline
            .add_task(Box::new(InitramfsTask))
            .add_task(Box::new(KernelStageTask))
            .add_task(Box::new(BootConfigTask { bootloader }))
            .add_task(Box::new(ImageFinalizeTask { format }));
    }

    pipeline.run(&ctx)?;

    logging::ready(
        "build::interactive",
        "Masterpiece pipeline completed successfully",
        "ok",
        &[],
    );
    Ok(())
}
