use anyhow::{Result, Context};
use std::fs;
use crate::utils::logging;
use crate::engine::operations::Op;
use crate::utils::sys::process::Executor;
use crate::utils::fs::stats::{self, DirStats};

pub fn execute(all: bool, distros: bool, logs: bool, dry_run: bool) -> Result<()> {
    logging::info("clean", "Initiating system-wide purge sequence...", &[]);

    let mut total_stats = DirStats::new();

    if dry_run {
        logging::warn("clean", "DRY-RUN MODE: No files will be deleted", &[]);
    }

    // 1. Core Artifacts
    let artifacts = crate::utils::paths::repo_root().join("artifacts");
    if artifacts.exists() {
        let s = stats::get_dir_stats(&artifacts);
        total_stats.add(&s);
        
        if dry_run {
            logging::info("clean", "[DRY-RUN] Would remove artifacts directory", &[
                ("path", &artifacts.to_string_lossy()),
                ("files", &s.file_count.to_string()),
                ("size", &s.format_size())
            ]);
        } else {
            logging::info("clean", "Purging artifacts...", &[("size", &s.format_size())]);
            Op::clean_dir(&artifacts, "CLEAN")?;
        }
    }

    // 2. Staging Areas
    let staging = crate::utils::paths::repo_root().join("staging");
    if staging.exists() {
        let s = stats::get_dir_stats(&staging);
        total_stats.add(&s);

        if dry_run {
            logging::info("clean", "[DRY-RUN] Would remove staging directory", &[
                ("path", &staging.to_string_lossy()),
                ("files", &s.file_count.to_string()),
                ("size", &s.format_size())
            ]);
        } else {
            logging::info("clean", "Purging staging area...", &[("size", &s.format_size())]);
            Op::clean_dir(&staging, "CLEAN")?;
        }
    }

    // 3. Optional: Distros
    if distros {
        let distros = crate::utils::paths::repo_root().join("distros");
        if distros.exists() {
            let s = stats::get_dir_stats(&distros);
            total_stats.add(&s);

            if dry_run {
                logging::info("clean", "[DRY-RUN] Would remove distros directory", &[
                    ("path", &distros.to_string_lossy()),
                    ("files", &s.file_count.to_string()),
                    ("size", &s.format_size())
                ]);
            } else {
                logging::info("clean", "Purging distro rootfs and ISO images...", &[("size", &s.format_size())]);
                fs::remove_dir_all(&distros).context("Failed to remove distros directory")?;
                fs::create_dir_all(&distros).context("Failed to re-create empty distros directory")?;
            }
        }
    }

    // 4. Optional: Logs
    if logs {
        let log_file = crate::utils::paths::repo_root().join("xtask.log");
        if log_file.exists() {
            let s = stats::get_file_stats(&log_file);
            total_stats.add(&s);

            if dry_run {
                logging::info("clean", "[DRY-RUN] Would remove xtask.log", &[
                    ("path", &log_file.to_string_lossy()),
                    ("size", &s.format_size())
                ]);
            } else {
                logging::info("clean", "Removing execution logs...", &[("size", &s.format_size())]);
                fs::remove_file(&log_file).context("Failed to remove xtask.log")?;
            }
        }
    }

    // 5. Deep Clean: Cargo
    if all {
        if dry_run {
            logging::info("clean", "[DRY-RUN] Would run 'cargo clean' for all targets", &[]);
        } else {
            logging::info("clean", "Running deep cargo clean...", &[]);
            Executor::new("cargo").arg("clean").run()?;
        }
    }

    if !dry_run {
        let fields = [
            ("total_files", total_stats.file_count.to_string()),
            ("space_reclaimed", total_stats.format_size())
        ];
        let fields_refs: Vec<(&str, &str)> = fields.iter().map(|(k,v)| (*k, v.as_str())).collect();

        logging::ready(
            "clean",
            "System purge complete. All designated staging areas are neutralized.",
            "SUCCESS",
            &fields_refs,
        );
    } else {
        logging::info("clean", "Dry-run analysis complete. No changes made.", &[]);
    }

    Ok(())
}
