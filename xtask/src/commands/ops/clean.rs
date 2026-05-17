use anyhow::Result;
use std::fs;
use crate::utils::logging;
use crate::engine::operations::Op;
use crate::utils::sys::process::Executor;
use crate::utils::fs::stats::{self, DirStats, ScanOrchestrator};
use crate::utils::fs::paths::LAYOUT;

/// Purge build artifacts, staging areas, and logs.
/// 
/// If `all` is true, also runs `cargo clean`.
/// If `distros` is true, also removes downloaded distro images.
pub fn execute(all: bool, distros: bool, logs: bool, no_stats: bool, dry_run: bool) -> Result<()> {
    logging::info("clean", "Initiating system-wide purge sequence...", &[]);

    let mut total_stats = DirStats::new();
    let orchestrator = ScanOrchestrator::new(3); // Total 3 second timeout for all scans

    if dry_run {
        logging::warn("clean", "DRY-RUN MODE: No files will be deleted", &[]);
    }

    if no_stats {
        logging::info("clean", "Stats calculation disabled for this run", &[]);
    }

    // Define targets based on centralized LAYOUT
    let targets = vec![
        ("Artifacts", &LAYOUT.artifacts, true),
        ("Staging", &LAYOUT.staging, true),
        ("Distros", &LAYOUT.distros, distros || all), // Distros included if 'all' or explicit
    ];

    for (name, path, enabled) in targets {
        if !enabled || !path.exists() { continue; }

        if !no_stats && !total_stats.interrupted {
            let s = orchestrator.scan(path);
            total_stats.merge(&s);
        }

        if dry_run {
            logging::info("clean", &format!("[DRY-RUN] Would remove {} directory", name), &[
                ("path", &path.to_string_lossy()),
            ]);
        } else {
            logging::info("clean", &format!("Purging {}...", name), &[]);
            
            if name == "Distros" {
                // We keep the directory but clear contents to avoid structure break
                if let Ok(entries) = fs::read_dir(path) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.is_dir() {
                            let _ = fs::remove_dir_all(p);
                        } else {
                            let _ = fs::remove_file(p);
                        }
                    }
                }
            } else {
                Op::clean_dir(path, "CLEAN")?;
            }
        }
    }

    // Handle Logs
    if (logs || all) && LAYOUT.logs.exists() {
        if !no_stats && !total_stats.interrupted {
            let s = stats::get_file_stats(&LAYOUT.logs);
            total_stats.merge(&s);
        }

        if dry_run {
            logging::info("clean", "[DRY-RUN] Would remove execution log", &[("path", &LAYOUT.logs.to_string_lossy())]);
        } else {
            logging::info("clean", "Removing execution logs...", &[]);
            let _ = fs::remove_file(&LAYOUT.logs);
        }
    }

    // Deep Clean: Cargo
    if all {
        if dry_run {
            logging::info("clean", "[DRY-RUN] Would run 'cargo clean'", &[]);
        } else {
            logging::info("clean", "Running deep cargo clean...", &[]);
            let _ = Executor::new("cargo").arg("clean").run();
        }
    }

    if !dry_run {
        let mut fields = vec![];
        if !no_stats && !total_stats.interrupted {
            fields.push(("files".to_string(), total_stats.file_count.to_string()));
            fields.push(("reclaimed".to_string(), total_stats.format_size()));
        } else if total_stats.interrupted {
            fields.push(("stats".to_string(), "INTERRUPTED".to_string()));
        }

        let fields_refs: Vec<(&str, &str)> = fields.iter().map(|(k, v)| (k.as_ref(), v.as_ref())).collect();
        logging::ready(
            "clean",
            "System purge complete. Staging areas neutralized.",
            "SUCCESS",
            &fields_refs,
        );
    }

    Ok(())
}
