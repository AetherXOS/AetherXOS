use crate::constants::{self, cargo as cargo_consts};
use crate::utils::{cargo, logging, paths};
use anyhow::{Context, Result, bail};
use std::fs;

/// Automates generic isolation compilation of peripheral userspace binaries.
pub fn build_userspace_app(name: &str, is_release: bool) -> Result<()> {
    logging::info(
        "app",
        "orchestrating cargo build for target app",
        &[("name", name)],
    );

    let app_dir = paths::userspace_src(name);
    if !app_dir.exists() {
        bail!(
            "Requested userspace application directory not found: {}",
            app_dir.display()
        );
    }

    let mut compiler_args = vec![
        cargo_consts::CMD_BUILD,
        cargo_consts::ARG_MANIFEST_PATH,
        cargo_consts::MANIFEST_FILE,
        cargo_consts::ARG_TARGET,
        constants::defaults::build::USERSPACE_TARGET,
    ];
    if is_release {
        compiler_args.push(cargo_consts::ARG_RELEASE);
    }

    cargo::cargo_in_dir(&compiler_args, &app_dir)
        .context("Userspace cargo sub-process execution unexpectedly collapsed")?;

    let target_profile = if is_release { "release" } else { "debug" };
    let compiled_elf = app_dir.join(format!(
        "target/{}/{}/{}",
        constants::defaults::build::USERSPACE_TARGET,
        target_profile,
        name
    ));

    let init_bin_dir = constants::paths::boot_image_stage_boot().join("initramfs/usr/bin");
    paths::ensure_dir(&init_bin_dir)?;

    if compiled_elf.exists() {
        logging::info(
            "app",
            "verifying application binary integrity",
            &[("name", name)],
        );
        crate::utils::elf::validate_elf(&compiled_elf)?;

        fs::copy(&compiled_elf, init_bin_dir.join(name))
            .context("Failed moving synthesized user program to VFS")?;
        logging::info(
            "app",
            "integrated app into initramfs isolation bounds",
            &[("name", name)],
        );
    } else {
        bail!(
            "Critical workflow failure: Output ELF not presented where expected: {}",
            compiled_elf.display()
        );
    }

    Ok(())
}
