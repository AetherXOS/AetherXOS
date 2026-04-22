use anyhow::Result;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;

use crate::constants;
use crate::utils::paths;
use crate::utils::report;

// ---------------------------------------------------------------------------
// Types matching Python syscall_coverage_report.py output
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct SyscallRow {
    linux_nr: String,
    handler: String,
    status: String,
    file: String,
    reason: String,
}

#[derive(Serialize)]
struct CoverageSummary {
    total: usize,
    implemented: usize,
    partial: usize,
    no: usize,
    external: usize,
    implemented_pct: f64,
}

/// Generate Linux syscall coverage report natively in Rust.
///
/// Replaces: scripts/syscall_coverage_report.py
pub fn execute(linux_compat: bool, format: &str, out: &Option<String>) -> Result<()> {
    println!(
        "[syscall-coverage] Generating report (linux_compat={}, format={})",
        linux_compat, format
    );

    let root = paths::repo_root();

    // Scan dispatch files for syscall mappings.
    let dispatch_dirs = vec![
        paths::kernel_src("modules/linux_compat/sys_dispatcher"),
        paths::kernel_src("kernel/syscalls"),
    ];

    let mut mappings: HashMap<String, String> = HashMap::new();
    let map_re = regex::Regex::new(r"linux_nr::([A-Z0-9_]+)\s*=>\s*Some\((.*?)\),").unwrap();
    let fn_re = regex::Regex::new(r"\b(sys_linux_[a-zA-Z0-9_]+)\b").unwrap();

    for dir in &dispatch_dirs {
        if !dir.exists() {
            continue;
        }
        for entry in walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|x| x == "rs").unwrap_or(false))
        {
            let text = fs::read_to_string(entry.path()).unwrap_or_default();
            for cap in map_re.captures_iter(&text) {
                let nr = cap[1].to_string();
                let expr = &cap[2];
                if let Some(fn_match) = fn_re.find(expr) {
                    mappings.insert(nr, fn_match.as_str().to_string());
                } else {
                    mappings.insert(
                        nr,
                        format!(
                            "<expr>{}",
                            expr.split_whitespace().collect::<Vec<_>>().join(" ")
                        ),
                    );
                }
            }
        }
    }

    // Scan handler files for function bodies.
    let handler_dirs = vec![
        paths::kernel_src("modules/linux_compat"),
        paths::kernel_src("kernel/syscalls"),
    ];

    let fn_def_re = regex::Regex::new(r"\bfn\s+(sys_linux_[a-zA-Z0-9_]+)\s*\(").unwrap();
    let mut handler_bodies: HashMap<String, (String, String)> = HashMap::new(); // name -> (file, body)

    for dir in &handler_dirs {
        if !dir.exists() {
            continue;
        }
        for entry in walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|x| x == "rs").unwrap_or(false))
        {
            let text = fs::read_to_string(entry.path()).unwrap_or_default();
            let rel = entry
                .path()
                .strip_prefix(&root)
                .unwrap_or(entry.path())
                .to_string_lossy()
                .replace('\\', "/");

            for m in fn_def_re.find_iter(&text) {
                let fn_name_match = fn_def_re.captures(&text[m.start()..]).unwrap();
                let fn_name = fn_name_match[1].to_string();
                
                // Need to find the brace offset starting from m.start()
                let body = if let Some(brace_offset) = text[m.start()..].find('{') {
                    crate::utils::parser::extract_body(&text, m.start() + brace_offset).unwrap_or_default()
                } else {
                    String::new()
                };
                handler_bodies.insert(fn_name, (rel.clone(), body));
            }
        }
    }

    // Classify each mapping
    let mut rows: Vec<SyscallRow> = Vec::new();
    for (nr, handler) in mappings.iter() {
        let (status, file, reason) = if let Some(expr_raw) = handler.strip_prefix("<expr>") {
            let expr = expr_raw.to_lowercase();
            if expr.contains("linux_nosys()") || expr.contains("enosys") {
                (
                    "no".into(),
                    "-".into(),
                    "direct expression returns ENOSYS".into(),
                )
            } else {
                ("implemented".into(), "-".into(), "direct expression".into())
            }
        } else if let Some((file, body)) = handler_bodies.get(handler) {
            let body_lc = body.to_lowercase();
            if body_lc.contains("linux_nosys()") {
                ("no".into(), file.clone(), "contains linux_nosys()".into())
            } else if body_lc.contains("eopnotsupp") || body_lc.contains("enosys") {
                (
                    "partial".into(),
                    file.clone(),
                    "returns EOPNOTSUPP/ENOSYS path".into(),
                )
            } else if body_lc.contains("todo") || body_lc.contains("stub") {
                (
                    "partial".into(),
                    file.clone(),
                    "contains TODO/stub markers".into(),
                )
            } else {
                (
                    "implemented".into(),
                    file.clone(),
                    "no unsupported markers".into(),
                )
            }
        } else {
            (
                "external".into(),
                "-".into(),
                "handler definition not found".into(),
            )
        };

        rows.push(SyscallRow {
            linux_nr: nr.clone(),
            handler: handler.clone(),
            status,
            file,
            reason,
        });
    }

    rows.sort_by(|a, b| a.linux_nr.cmp(&b.linux_nr));

    let total = rows.len();
    let implemented = rows.iter().filter(|r| r.status == "implemented").count();
    let partial = rows.iter().filter(|r| r.status == "partial").count();
    let no = rows.iter().filter(|r| r.status == "no").count();
    let external = rows.iter().filter(|r| r.status == "external").count();
    let implemented_pct = if total > 0 {
        100.0 * implemented as f64 / total as f64
    } else {
        0.0
    };

    let summary = CoverageSummary {
        total,
        implemented,
        partial,
        no,
        external,
        implemented_pct,
    };

    // Render output
    let rendered = if format == "json" {
        serde_json::to_string_pretty(&rows)?
    } else {
        let mut md = String::new();
        md.push_str("# Linux Syscall Coverage Report\n\n");
        md.push_str(&format!("Total mapped syscalls: **{}**\n\n", total));
        md.push_str("| Status | Count | Percent |\n|---|---:|---:|\n");
        for (label, count) in &[
            ("implemented", implemented),
            ("partial", partial),
            ("no", no),
            ("external", external),
        ] {
            md.push_str(&format!(
                "| {} | {} | {:.1}% |\n",
                label,
                count,
                if total > 0 {
                    100.0 * *count as f64 / total as f64
                } else {
                    0.0
                }
            ));
        }
        md.push_str("\n| Linux NR | Handler | Status | File | Reason |\n|---|---|---|---|---|\n");
        for r in &rows {
            md.push_str(&format!(
                "| {} | `{}` | {} | `{}` | {} |\n",
                r.linux_nr, r.handler, r.status, r.file, r.reason
            ));
        }
        md
    };

    // Write output
    if let Some(out_path) = out {
        let p = paths::resolve(out_path);
        paths::ensure_dir(p.parent().unwrap())?;
        report::write_text_report(&p, &rendered)?;
        println!("[syscall-coverage] Report written: {}", p.display());
    } else {
        println!("{}", rendered);
    }

    // Always write summary JSON
    let summary_path = constants::paths::syscall_coverage_summary();
    report::write_json_report(&summary_path, &summary)?;

    println!(
        "[syscall-coverage] Total: {} | Implemented: {} ({:.1}%) | Partial: {} | No: {} | External: {}",
        total, implemented, implemented_pct, partial, no, external
    );
    Ok(())
}
