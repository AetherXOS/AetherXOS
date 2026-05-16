use anyhow::Result;
use aethercore_common::TargetArch;
use strum::IntoEnumIterator;

use crate::cli::{Bootloader, ImageFormat};
use crate::utils::{features, logging, ui};

use super::{image, kernel};

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
    logging::info("build::interactive", "starting interactive build wizard", &[]);

    let modes = [BuildMode::KernelOnly, BuildMode::FullPipeline];
    let mode = *ui::select("Select build mode", &modes)?;

    let arches: Vec<TargetArch> = TargetArch::supported().collect();
    let arch = *ui::select("Select target architecture", &arches)?;

    let profile_choices = ["debug", "release"];
    let profile = *ui::select("Select build profile", &profile_choices)?;
    let release = profile == "release";

    let features = features::prompt_kernel_feature_selection("Build", &[])?;

    logging::info(
        "build::interactive",
        "selected kernel build settings",
        &[
            ("mode", &mode.to_string()),
            ("arch", arch.as_str()),
            ("release", &release.to_string()),
            ("features", &features.to_string()),
        ],
    );

    kernel::build_kernel(arch, release, features)?;

    if matches!(mode, BuildMode::FullPipeline) {
        let bootloaders: Vec<Bootloader> = Bootloader::iter().collect();
        let bootloader = *ui::select("Select bootloader", &bootloaders)?;

        let formats: Vec<ImageFormat> = ImageFormat::iter().collect();
        let format = *ui::select("Select image format", &formats)?;

        let rootfs = if ui::confirm("Use external rootfs source?", false)? {
            let value = ui::input("External rootfs path (directory or archive)", None)?;
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        } else {
            None
        };

        super::build_initramfs()?;
        image::bundle_image(
            arch,
            &bootloader,
            &format,
            rootfs
                .as_deref()
                .map(std::path::Path::new),
        )?;
    }

    logging::ready("build::interactive", "interactive build flow completed", "ok");
    Ok(())
}
