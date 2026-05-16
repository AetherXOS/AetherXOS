use anyhow::{Result, Context};
use std::path::Path;
use crate::engine::{Task, ExecutionContext, task::TaskStatus};
use crate::utils::logging;
use crate::constants::cargo as cargo_consts;
use aethercore_common::{TargetArch, KernelFeatures};

pub struct KernelCompileTask {
    pub arch: TargetArch,
    pub release: bool,
    pub features: KernelFeatures,
}

impl Task for KernelCompileTask {
    fn name(&self) -> &str { "Kernel Compilation" }
    fn description(&self) -> &str { "Compiles the AetherX core kernel binary" }
    
    fn run(&self, _ctx: &ExecutionContext) -> Result<TaskStatus> {
        let target_triple = self.arch.to_bare_metal_triple();
        let mut args = vec![
            cargo_consts::CMD_BUILD,
            "-p", "aether-x-os",
            "--lib", "--bin", "aethercore",
            cargo_consts::ARG_TARGET, target_triple,
        ];
        if self.release { args.push(cargo_consts::ARG_RELEASE); }

        let cargo_features = self.features.to_cargo_features();
        let features_str = cargo_features.join(",");
        if !cargo_features.is_empty() {
            args.push(cargo_consts::ARG_FEATURES);
            args.push(&features_str);
        }

        logging::status("BUILD", &format!("Compiling kernel for {} (Profile: {})", target_triple, if self.release { "release" } else { "debug" }));
        
        crate::utils::sys::process::run_checked_in_dir(
            "cargo",
            &args,
            Path::new("kernel")
        ).context("Failed to compile kernel")?;
        
        Ok(TaskStatus::Success)
    }
}

pub struct InitramfsTask;

impl Task for InitramfsTask {
    fn name(&self) -> &str { "Initramfs Generation" }
    fn description(&self) -> &str { "Packs the early userspace into a CPIO archive" }
    
    fn run(&self, _ctx: &ExecutionContext) -> Result<TaskStatus> {
        use crate::constants::paths;
        let src = paths::boot_initramfs_src();
        let out = paths::boot_image_stage_initramfs();
        crate::commands::infra::initramfs::build(&src, &out)?;
        Ok(TaskStatus::Success)
    }
}
