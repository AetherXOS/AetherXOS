use anyhow::{Context, Result, bail};
use std::fs;

use crate::config;
use crate::utils::{paths, report};

use crate::commands::release::preflight::models::{WarningAuditHit, WarningAuditReport};

pub fn execute(strict: bool, from_file: Option<&str>) -> Result<()> {
    println!("[release::warning-audit] Auditing warning lines for critical kernel paths");
    let root = paths::repo_root();

    let mut logs = Vec::new();
    if let Some(path) = from_file {
        logs.push(root.join(path));
    } else {
        logs.push(root.join("build_log.txt"));
        logs.push(root.join("build_output.txt"));
        logs.push(root.join("cargo_build.txt"));
    }

    let critical_paths = [
        "kernel/src/hal/",
        "kernel/src/kernel_runtime/",
        "kernel/src/kernel/syscalls/",
    ];
    let mut hits = Vec::new();
    let mut scanned_logs = 0usize;

    for log in logs {
        if !log.exists() {
            continue;
        }
        scanned_logs += 1;
        let text = fs::read_to_string(&log)
            .with_context(|| format!("failed reading warning audit log: {}", log.display()))?;
        for line in text.lines() {
            let lower = line.to_ascii_lowercase();
            if !lower.contains("warning") {
                continue;
            }
            if critical_paths.iter().any(|path| lower.contains(path)) {
                hits.push(WarningAuditHit {
                    source_file: log
                        .strip_prefix(&root)
                        .unwrap_or(log.as_path())
                        .to_string_lossy()
                        .replace('\\', "/"),
                    line: line.trim().to_string(),
                });
            }
        }
    }

    let report_obj = WarningAuditReport {
        generated_utc: report::utc_now_iso(),
        strict,
        overall_ok: hits.is_empty(),
        scanned_logs,
        hit_count: hits.len(),
        hits,
    };

    let out_json = root.join(config::repo_paths::WARNING_AUDIT_JSON);
    let out_md = root.join(config::repo_paths::WARNING_AUDIT_MD);
    report::write_json_report(&out_json, &report_obj)?;
    report::write_text_report(&out_md, &render_warning_audit_md(&report_obj))?;

    if strict && !report_obj.overall_ok {
        bail!(
            "strict warning audit failed: hit_count={}. See {}",
            report_obj.hit_count,
            out_json.display()
        );
    }

    println!("[release::warning-audit] PASS");
    Ok(())
}

fn render_warning_audit_md(report_obj: &WarningAuditReport) -> String {
    let mut md = String::new();
    md.push_str("# Warning Audit\n\n");
    md.push_str(&format!("- generated_utc: {}\n", report_obj.generated_utc));
    md.push_str(&format!("- strict: {}\n", report_obj.strict));
    md.push_str(&format!("- overall_ok: {}\n", report_obj.overall_ok));
    md.push_str(&format!("- scanned_logs: {}\n", report_obj.scanned_logs));
    md.push_str(&format!("- hit_count: {}\n\n", report_obj.hit_count));

    if report_obj.hits.is_empty() {
        md.push_str("No critical-path warning lines found in scanned logs.\n");
        return md;
    }

    md.push_str("## Hits\n\n");
    for hit in &report_obj.hits {
        md.push_str(&format!("- {} :: {}\n", hit.source_file, hit.line));
    }
    md
}
