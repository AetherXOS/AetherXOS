use anyhow::{Context, Result, bail};
use regex::Regex;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

use crate::config;
use crate::utils::{paths, report};

use crate::commands::release::preflight::models::{PolicyGuardReport, PolicyViolation};

pub fn execute(strict: bool) -> Result<()> {
    println!("[release::policy-guard] Scanning critical kernel modules for forbidden patterns");
    let root = paths::repo_root();

    let targets = [
        "kernel/src/hal",
        "kernel/src/kernel_runtime",
        "kernel/src/kernel/syscalls",
    ];
    let rules = [
        (
            Regex::new(r"\bunimplemented!\b")?,
            "critical",
            "unimplemented!",
        ),
        (Regex::new(r"\btodo!\b")?, "high", "todo!"),
        (Regex::new(r"\bdbg!\s*\(")?, "high", "dbg!(...)"),
        (Regex::new(r"FIXME")?, "medium", "FIXME marker"),
    ];

    let (violations, scanned_files) = scan_policy_targets(&root, &targets, &rules)?;

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

    enforce_strict_policy(strict, &report_obj, &out_json)?;

    if report_obj.overall_ok {
        println!("[release::policy-guard] PASS");
    } else {
        println!(
            "[release::policy-guard] WARN: violations={} (strict=false)",
            report_obj.violation_count
        );
    }
    Ok(())
}

fn scan_policy_targets(
    root: &Path,
    targets: &[&str],
    rules: &[(Regex, &str, &str)],
) -> Result<(Vec<PolicyViolation>, usize)> {
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
                .strip_prefix(root)
                .unwrap_or(abs)
                .to_string_lossy()
                .replace('\\', "/");
            let text = fs::read_to_string(abs)
                .with_context(|| format!("failed reading policy scan target: {}", rel_path))?;
            for (idx, line) in text.lines().enumerate() {
                for (rule, severity, label) in rules {
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

    Ok((violations, scanned_files))
}

fn enforce_strict_policy(
    strict: bool,
    report_obj: &PolicyGuardReport,
    out_json: &Path,
) -> Result<()> {
    if strict && !report_obj.overall_ok {
        bail!(
            "strict critical policy guard failed: violations={}. See {}",
            report_obj.violation_count,
            out_json.display()
        );
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(name: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("xtask_policy_guard_{name}_{stamp}"));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn scan_policy_targets_detects_todo_marker() {
        let root = unique_temp_dir("scan_detect");
        let target_dir = root.join("kernel/src/hal");
        std::fs::create_dir_all(&target_dir).expect("create target dir");
        let file = target_dir.join("sample.rs");
        std::fs::write(&file, "fn x() { todo!(); }\n").expect("write sample");

        let targets = ["kernel/src/hal"];
        let rules = [(Regex::new(r"todo!").expect("regex"), "high", "todo!")];

        let (violations, scanned) = scan_policy_targets(&root, &targets, &rules).expect("scan");
        assert_eq!(scanned, 1);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].pattern, "todo!");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn strict_policy_enforcement_fails_when_violations_exist() {
        let report = PolicyGuardReport {
            generated_utc: "2026-04-26T00:00:00Z".to_string(),
            strict: true,
            overall_ok: false,
            violation_count: 2,
            scanned_files: 1,
            violations: vec![],
        };

        let out = std::path::PathBuf::from("reports/tooling/critical_policy_guard.json");
        let result = enforce_strict_policy(true, &report, &out);
        assert!(result.is_err());
    }

    #[test]
    fn strict_policy_enforcement_passes_when_not_strict() {
        let report = PolicyGuardReport {
            generated_utc: "2026-04-26T00:00:00Z".to_string(),
            strict: false,
            overall_ok: false,
            violation_count: 1,
            scanned_files: 1,
            violations: vec![],
        };

        let out = std::path::PathBuf::from("reports/tooling/critical_policy_guard.json");
        let result = enforce_strict_policy(false, &report, &out);
        assert!(result.is_ok());
    }

    #[test]
    fn render_policy_guard_md_without_violations() {
        let doc = PolicyGuardReport {
            generated_utc: "2026-04-26T00:00:00Z".to_string(),
            strict: false,
            overall_ok: true,
            violation_count: 0,
            scanned_files: 3,
            violations: Vec::new(),
        };

        let md = render_policy_guard_md(&doc);
        assert!(md.contains("# Critical Policy Guard"));
        assert!(md.contains("- overall_ok: true"));
        assert!(md.contains("No forbidden patterns found."));
    }

    #[test]
    fn render_policy_guard_md_with_violations() {
        let doc = PolicyGuardReport {
            generated_utc: "2026-04-26T00:00:00Z".to_string(),
            strict: true,
            overall_ok: false,
            violation_count: 1,
            scanned_files: 1,
            violations: vec![PolicyViolation {
                path: "kernel/src/kernel/syscalls/mock.rs".to_string(),
                line: 77,
                pattern: "todo!".to_string(),
                severity: "high".to_string(),
                snippet: "todo!()".to_string(),
            }],
        };

        let md = render_policy_guard_md(&doc);
        assert!(md.contains("## Violations"));
        assert!(md.contains("kernel/src/kernel/syscalls/mock.rs:77 [high] todo!"));
    }
}
