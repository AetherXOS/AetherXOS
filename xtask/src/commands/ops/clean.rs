use anyhow::{Result, Context};
use std::fs;
use crate::utils::logging;
use crate::engine::operations::Op;
use crate::utils::sys::process::Executor;
use crate::utils::fs::stats::{self, DirStats};

pub fn execute(all: bool, distros: bool, logs: bool, no_stats: bool, dry_run: bool) -> Result<()> {
    logging::info("clean", "Initiating system-wide purge sequence...", &[]);

    let mut total_stats = DirStats::new();
    let mut stats_aborted = no_stats;

    if dry_run {
        logging::warn("clean", "DRY-RUN MODE: No files will be deleted", &[]);
    }

    if no_stats {
        logging::info("clean", "Stats calculation skipped by user request", &[]);
    }

    // 1. Core Artifacts
    let artifacts = crate::utils::paths::repo_root().join("artifacts");
    if artifacts.exists() {
        let mut s = DirStats::new();
        if !stats_aborted {
            s = stats::get_dir_stats(&artifacts);
            if s.interrupted { stats_aborted = true; }
            total_stats.merge(&s);
        }
        
        if dry_run {
            let fields = if !stats_aborted {
                vec![
                    ("path".to_string(), artifacts.to_string_lossy().to_string()),
                    ("files".to_string(), s.file_count.to_string()),
                    ("size".to_string(), s.format_size()),
                ]
            } else {
                vec![("path".to_string(), artifacts.to_string_lossy().to_string())]
            };
            let field_refs: Vec<(&str, &str)> = fields.iter().map(|(k, v)| (k.as_ref(), v.as_ref())).collect();
            logging::info("clean", "[DRY-RUN] Would remove artifacts directory", &field_refs);
        } else {
            if !stats_aborted {
                logging::info("clean", "Purging artifacts...", &[("size", &s.format_size())]);
            } else {
                logging::info("clean", "Purging artifacts...", &[]);
            }
            Op::clean_dir(&artifacts, "CLEAN")?;
        }
    }

    // 2. Staging Areas
    let staging = crate::utils::paths::repo_root().join("staging");
    if staging.exists() {
        let mut s = DirStats::new();
        if !stats_aborted {
            s = stats::get_dir_stats(&staging);
            if s.interrupted { stats_aborted = true; }
            total_stats.merge(&s);
        }

        if dry_run {
            let fields = if !stats_aborted {
                vec![
                    ("path".to_string(), staging.to_string_lossy().to_string()),
                    ("files".to_string(), s.file_count.to_string()),
                    ("size".to_string(), s.format_size()),
                ]
            } else {
                vec![("path".to_string(), staging.to_string_lossy().to_string())]
            };
            let field_refs: Vec<(&str, &str)> = fields.iter().map(|(k, v)| (k.as_ref(), v.as_ref())).collect();
            logging::info("clean", "[DRY-RUN] Would remove staging directory", &field_refs);
        } else {
            if !stats_aborted {
                logging::info("clean", "Purging staging area...", &[("size", &s.format_size())]);
            } else {
                logging::info("clean", "Purging staging area...", &[]);
            }
            Op::clean_dir(&staging, "CLEAN")?;
        }
    }

    // 3. Optional: Distros
    if distros {
        let distros = crate::utils::paths::repo_root().join("distros");
        if distros.exists() {
            let mut s = DirStats::new();
            if !stats_aborted {
                s = stats::get_dir_stats(&distros);
                if s.interrupted { stats_aborted = true; }
                total_stats.merge(&s);
            }

            if dry_run {
                let fields = if !stats_aborted {
                    vec![
                        ("path".to_string(), distros.to_string_lossy().to_string()),
                        ("files".to_string(), s.file_count.to_string()),
                        ("size".to_string(), s.format_size()),
                    ]
                } else {
                    vec![("path".to_string(), distros.to_string_lossy().to_string())]
                };
                let field_refs: Vec<(&str, &str)> = fields.iter().map(|(k, v)| (k.as_ref(), v.as_ref())).collect();
                logging::info("clean", "[DRY-RUN] Would remove distros directory", &field_refs);
            } else {
                if !stats_aborted {
                    logging::info("clean", "Purging distro rootfs and ISO images...", &[("size", &s.format_size())]);
                } else {
                    logging::info("clean", "Purging distro rootfs and ISO images...", &[]);
                }
                fs::remove_dir_all(&distros).context("Failed to remove distros directory")?;
                fs::create_dir_all(&distros).context("Failed to re-create empty distros directory")?;
            }
        }
    }

    // 4. Optional: Logs
    if logs {
        let log_file = crate::utils::paths::repo_root().join("xtask.log");
        if log_file.exists() {
            let mut s = DirStats::new();
            if !stats_aborted {
                s = stats::get_file_stats(&log_file);
                total_stats.merge(&s);
            }

            if dry_run {
                let fields = if !stats_aborted {
                    vec![
                        ("path".to_string(), log_file.to_string_lossy().to_string()),
                        ("size".to_string(), s.format_size()),
                    ]
                } else {
                    vec![("path".to_string(), log_file.to_string_lossy().to_string())]
                };
                let field_refs: Vec<(&str, &str)> = fields.iter().map(|(k, v)| (k.as_ref(), v.as_ref())).collect();
                logging::info("clean", "[DRY-RUN] Would remove xtask.log", &field_refs);
            } else {
                if !stats_aborted {
                    logging::info("clean", "Removing execution logs...", &[("size", &s.format_size())]);
                } else {
                    logging::info("clean", "Removing execution logs...", &[]);
                }
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
        let mut fields = vec![];
        if !stats_aborted {
            fields.push(("total_files".to_string(), total_stats.file_count.to_string()));
            fields.push(("space_reclaimed".to_string(), total_stats.format_size()));
        } else {
            fields.push(("stats".to_string(), "SKIPPED/INTERRUPTED".to_string()));
        }
        let fields_refs: Vec<(&str, &str)> = fields.iter().map(|(k, v)| (k.as_ref(), v.as_ref())).collect();

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
