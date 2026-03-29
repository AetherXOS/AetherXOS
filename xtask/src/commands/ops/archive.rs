use anyhow::{Context, Result};
use serde::Serialize;
use std::fs;

use crate::utils::paths;
use crate::utils::report;

/// Default source paths to archive from nightly runs.
const SOURCE_PATHS: &[&str] = &[
    "reports/p0_p1_nightly",
    "reports/p1_nightly",
    "reports/p1_release_acceptance",
    "reports/p1_ops_gate",
    "reports/ab_slot_flip",
    "reports/ab_boot_recovery_gate",
    "reports/reboot_recovery_gate",
    "artifacts/qemu_soak",
    "artifacts/boot_ab",
];

#[derive(Serialize)]
struct ArchiveManifest {
    run_id: String,
    created_utc: String,
    copied: Vec<String>,
    missing: Vec<String>,
}

/// Archive nightly run artifacts into a timestamped directory.
///
/// Replaces: scripts/archive_nightly_artifacts.ps1
pub fn execute(run_id: &Option<String>) -> Result<()> {
    let id = run_id.clone().unwrap_or_else(|| {
        chrono::Local::now().format("%Y%m%d_%H%M%S").to_string()
    });

    let archive_root = paths::resolve("artifacts/nightly_runs");
    let dest = archive_root.join(&id);
    paths::ensure_dir(&dest)?;

    println!("[archive] Archiving nightly artifacts to: {}", dest.display());

    let mut copied = Vec::new();
    let mut missing = Vec::new();

    for source in SOURCE_PATHS {
        let src = paths::resolve(source);
        if src.exists() {
            let leaf = src.file_name().unwrap().to_string_lossy().to_string();
            let dst = dest.join(&leaf);
            copy_dir_recursive(&src, &dst)?;
            copied.push(source.to_string());
            println!("[archive]   Copied: {}", source);
        } else {
            missing.push(source.to_string());
        }
    }

    let manifest = ArchiveManifest {
        run_id: id.clone(),
        created_utc: report::utc_now_iso(),
        copied,
        missing,
    };
    report::write_json_report(&dest.join("manifest.json"), &manifest)?;

    println!("[archive] Archive completed: {}", dest.display());
    Ok(())
}

/// Recursively copy a directory tree.
fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)
                .with_context(|| format!("Failed to copy {}", entry.path().display()))?;
        }
    }
    Ok(())
}
