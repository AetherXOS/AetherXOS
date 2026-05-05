use crate::utils::{context, logging};
use anyhow::{Context, Result};
use std::fs;

pub fn execute(target_too: bool) -> Result<()> {
    let out_dir = context::out_dir();

    if out_dir.exists() {
        logging::info(
            "clean",
            "removing artifacts directory",
            &[("path", &out_dir.to_string_lossy())],
        );
        fs::remove_dir_all(&out_dir).with_context(|| {
            format!(
                "Failed to remove artifacts directory: {}",
                out_dir.display()
            )
        })?;
    } else {
        logging::info("clean", "artifacts directory does not exist, skipping", &[]);
    }

    if target_too {
        let target_dir = crate::utils::paths::resolve("target");
        if target_dir.exists() {
            logging::info(
                "clean",
                "removing target directory",
                &[("path", &target_dir.to_string_lossy())],
            );
            fs::remove_dir_all(&target_dir).with_context(|| {
                format!(
                    "Failed to remove target directory: {}",
                    target_dir.display()
                )
            })?;
        } else {
            logging::info("clean", "target directory does not exist, skipping", &[]);
        }
    }

    logging::success("clean", "workspace clean completed", &[]);
    Ok(())
}
