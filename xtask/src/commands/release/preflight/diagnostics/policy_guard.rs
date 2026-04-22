use anyhow::{Result, bail};
use regex::Regex;
use std::fs;
use walkdir::WalkDir;

use crate::config;
use crate::utils::{paths, report};

use crate::commands::release::preflight::models::{
    PolicyGuardReport, PolicyViolation,
};

pub fn execute(strict: bool) -> Result<()> {
    println!("[release::policy-guard] Scanning critical kernel modules for forbidden patterns");
    let root = paths::repo_root();

    let targets = [
        "kernel/src/hal",
        "kernel/src/kernel_runtime",
        "kernel/src/kernel/syscalls",
    ];
    let rules = [
        (Regex::new(r"\bunimplemented!\b")?, "critical", "unimplemented!"),
        (Regex::new(r"\btodo!\b")?, "high", "todo!"),
        (Regex::new(r"\bdbg!\s*\(")?, "high", "dbg!(...)"),
        (Regex::new(r"FIXME")?, "medium", "FIXME marker"),
    ];

    let mut violations = Vec::new();
    let mut scanned_files = 0usize;

    for rel in targets {
        let base = root.join(rel);
        if !base.exists() {
            continue;
        }
        for entry in WalkDir::new(&base)
            .into_iter()
            .filter_map(|result| result.ok())
            .filter(|entry| entry.path().is_file())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .map(|ext| ext == "rs")
                    .unwrap_or(false)
            })
        {
            scanned_files += 1;
            let abs = entry.path();
            let rel_path = abs
                .strip_prefix(&root)
                .unwrap_or(abs)
                .to_string_lossy()
                .replace('\\', "/");
            let text = fs::read_to_string(abs).unwrap_or_default();
            for (idx, line) in text.lines().enumerate() {
                for (rule, severity, label) in &rules {
                    if rule.is_match(line) {
                        violations.push(PolicyViolation {
                            path: rel_path.clone(),
                            line: idx + 1,
                            pattern: (*label).to_string(),
                            severity: (*severity).to_string(),
                            snippet: line.trim().to_string(),
                        });
                    }
                }
            }
        }
    }

    let overall_ok = violations.is_empty();
    let report_obj = PolicyGuardReport {
        generated_utc: report::utc_now_iso(),
        strict,
        overall_ok,
        violation_count: violations.len(),
        scanned_files,
        violations,
    };

    let out_json = root.join(config::repo_paths::CRITICAL_POLICY_GUARD_JSON);
    let out_md = root.join(config::repo_paths::CRITICAL_POLICY_GUARD_MD);
    report::write_json_report(&out_json, &report_obj)?;
    report::write_text_report(&out_md, &render_policy_guard_md(&report_obj))?;

    if strict && !report_obj.overall_ok {
        bail!(
            "strict critical policy guard failed: violations={}. See {}",
            report_obj.violation_count,
            out_json.display()
        );
    }

    println!("[release::policy-guard] PASS");
    Ok(())
}

fn render_policy_guard_md(report_obj: &PolicyGuardReport) -> String {
    let mut md = String::new();
    md.push_str("# Critical Policy Guard\n\n");
    md.push_str(&format!("- generated_utc: {}\n", report_obj.generated_utc));
    md.push_str(&format!("- strict: {}\n", report_obj.strict));
    md.push_str(&format!("- overall_ok: {}\n", report_obj.overall_ok));
    md.push_str(&format!("- scanned_files: {}\n", report_obj.scanned_files));
    md.push_str(&format!(
        "- violation_count: {}\n\n",
        report_obj.violation_count
    ));
    if report_obj.violations.is_empty() {
        md.push_str("No forbidden patterns found.\n");
        return md;
    }

    md.push_str("## Violations\n\n");
    for item in &report_obj.violations {
        md.push_str(&format!(
            "- {}:{} [{}] {}\n",
            item.path, item.line, item.severity, item.pattern
        ));
    }
    md
}
