use anyhow::{Result, Context};
use crate::constants::{cargo as cargo_consts};
use crate::utils::{logging};
use aethercore_common::TargetArch;

/// Compiles the kernel ELF payload for the explicitly defined target architecture.
pub fn build_kernel(arch: TargetArch, is_release: bool, features: aethercore_common::KernelFeatures) -> Result<()> {
    logging::info(
        "kernel",
        "processing standard kernel build",
        &[("arch", arch.as_str()), ("features", &features.to_string())],
    );
    let target_triple = arch.to_bare_metal_triple();

    let mut args = vec![
        cargo_consts::CMD_BUILD,
        "-p",
        "aether-x-os",
        "--lib",
        "--bin",
        "aethercore",
        cargo_consts::ARG_TARGET,
        target_triple,
    ];
    if is_release {
        args.push(cargo_consts::ARG_RELEASE);
    }

    let cargo_features = features.to_cargo_features();
    let features_str = cargo_features.join(",");
    if !cargo_features.is_empty() {
        args.push(cargo_consts::ARG_FEATURES);
        args.push(&features_str);
    }

    // Run cargo from the `kernel` directory so kernel-local `.cargo/config.toml`
    // and other per-package config are applied when building the kernel.
    let kernel_dir = std::path::Path::new("kernel");
    crate::utils::cargo::cargo_in_dir(&args, kernel_dir)
        .context("Platform cargo build invocation aborted")?;
    logging::info("kernel", "architecture compilation finalized", &[]);
    Ok(())
}
