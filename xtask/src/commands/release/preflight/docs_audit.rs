use anyhow::{Context, Result, bail};
use regex::Regex;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::config;
use crate::utils::{paths, report};

use super::relative_display;

#[derive(Serialize)]
struct DocsCommandAuditIssue {
    path: String,
    line: usize,
    command: String,
    severity: String,
    detail: String,
}

#[derive(Serialize)]
struct DocsCommandAuditDoc {
    generated_utc: String,
    strict: bool,
    overall_ok: bool,
    scanned_files: usize,
    command_hits: usize,
    issue_count: usize,
    issues: Vec<DocsCommandAuditIssue>,
}

pub(super) fn run(strict: bool) -> Result<()> {
    println!("[release::docs-command-audit] Scanning docs for xtask command drift");
    let root = paths::repo_root();
    let mut issues = Vec::new();
    let mut scanned_files = 0usize;
    let mut command_hits = 0usize;

    let command_re = Regex::new(
        r"cargo\s+run\s+-p\s+xtask(?:\s+--target\s+\S+)?\s+--\s+([a-zA-Z0-9-]+)(?:\s+([a-zA-Z0-9-]+))?",
    )?;

    for path in collect_docs_markdown_paths(&root) {
        scanned_files += 1;
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed reading markdown file: {}", path.display()))?;

        for (line_index, line) in text.lines().enumerate() {
            if let Some(caps) = command_re.captures(line) {
                command_hits += 1;
                let top = caps
                    .get(1)
                    .map(|m| m.as_str().to_ascii_lowercase())
                    .unwrap_or_default();
                let sub = caps
                    .get(2)
                    .map(|m| m.as_str().to_ascii_lowercase())
                    .unwrap_or_default();

                if !is_known_top_command(&top) {
                    issues.push(DocsCommandAuditIssue {
                        path: relative_display(&root, &path),
                        line: line_index + 1,
                        command: line.trim().to_string(),
                        severity: "high".to_string(),
                        detail: format!("unknown top-level xtask command '{}'", top),
                    });
                    continue;
                }

                if !sub.is_empty() && !sub.starts_with('-') && !is_known_subcommand(&top, &sub) {
                    issues.push(DocsCommandAuditIssue {
                        path: relative_display(&root, &path),
                        line: line_index + 1,
                        command: line.trim().to_string(),
                        severity: "medium".to_string(),
                        detail: format!("unknown '{}' subcommand '{}'", top, sub),
                    });
                }
            }
        }
    }

    let overall_ok = issues.is_empty();
    let report_obj = DocsCommandAuditDoc {
        generated_utc: report::utc_now_iso(),
        strict,
        overall_ok,
        scanned_files,
        command_hits,
        issue_count: issues.len(),
        issues,
    };

    let out_json = root.join(config::repo_paths::DOCS_COMMAND_AUDIT_JSON);
    let out_md = root.join(config::repo_paths::DOCS_COMMAND_AUDIT_MD);
    report::write_json_report(&out_json, &report_obj)?;
    report::write_text_report(&out_md, &render_docs_command_audit_md(&report_obj))?;

    if strict && !report_obj.overall_ok {
        bail!(
            "strict docs-command-audit failed: issue_count={}. See {}",
            report_obj.issue_count,
            out_json.display()
        );
    }

    println!("[release::docs-command-audit] PASS");
    Ok(())
}

fn collect_docs_markdown_paths(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let rel = relative_display(root, path);
        if rel == "README.md" || rel.starts_with("docs/") || rel.starts_with("xtask/docs/") {
            files.push(path.to_path_buf());
        }
    }
    files
}

fn is_known_top_command(command: &str) -> bool {
    matches!(
        command,
        "build"
            | "run"
            | "test"
            | "setup"
            | "dashboard"
            | "linux-abi"
            | "secureboot"
            | "release"
            | "ab-slot"
            | "core-pressure"
            | "crash-recovery"
            | "glibc"
    )
}

fn is_known_subcommand(top: &str, sub: &str) -> bool {
    match top {
        "setup" => matches!(
            sub,
            "audit" | "repair" | "bootstrap" | "installer-select" | "fetch-bootloader" | "toolchain"
        ),
        "release" => matches!(
            sub,
            "preflight"
                | "candidate-gate"
                | "p0-gate"
                | "p0-acceptance"
                | "p1-nightly"
                | "p1-acceptance"
                | "p0-p1-nightly"
                | "reproducible-evidence"
                | "reproducibility-compare"
                | "docs-command-audit"
                | "evidence-bundle"
                | "abi-drift-report"
                | "diagnostics"
                | "host-tool-verify"
                | "policy-guard"
                | "warning-audit"
                | "gate-fixup"
                | "ci-bundle"
                | "doctor"
                | "gate-report"
                | "export-junit"
                | "explain-failure"
                | "trend-dashboard"
                | "freeze-check"
                | "sbom-audit"
                | "score-normalize"
                | "release-notes"
                | "release-manifest"
                | "support-diagnostics"
                | "abi-perf-gate"
                | "perf-report"
        ),
        "build" => matches!(sub, "full" | "image" | "kernel" | "initramfs" | "app" | "tier-status"),
        "run" => matches!(sub, "smoke" | "live" | "bare-metal-deploy" | "debug" | "pxe-server"),
        "test" => matches!(
            sub,
            "quality-gate"
                | "host"
                | "agent-contract"
                | "all"
                | "posix-conformance"
                | "driver-smoke"
                | "tier"
                | "linux-app-compat"
                | "kernel-refactor-audit"
        ),
        "linux-abi" => matches!(
            sub,
            "gap-inventory"
                | "gate"
                | "errno-conformance"
                | "shim-errno-conformance"
                | "readiness-score"
                | "p2-gap-report"
                | "p2-gap-gate"
                | "semantic-matrix"
                | "trend-dashboard"
                | "workload-catalog"
        ),
        "secureboot" => {
            matches!(sub, "sign" | "sbat-validate" | "pcr-report" | "mok-plan" | "ovmf-matrix")
        }
        "dashboard" => matches!(sub, "build" | "test" | "open" | "agent-start"),
        "glibc" => matches!(sub, "audit" | "closure-gate" | "scorecard" | "compatibility-split"),
        "ab-slot" => matches!(sub, "init" | "stage" | "nightly-flip" | "recovery-gate"),
        _ => true,
    }
}

fn render_docs_command_audit_md(doc: &DocsCommandAuditDoc) -> String {
    let mut md = String::new();
    md.push_str("# Docs Command Audit\n\n");
    md.push_str(&format!("- generated_utc: {}\n", doc.generated_utc));
    md.push_str(&format!("- strict: {}\n", doc.strict));
    md.push_str(&format!("- overall_ok: {}\n", doc.overall_ok));
    md.push_str(&format!("- scanned_files: {}\n", doc.scanned_files));
    md.push_str(&format!("- command_hits: {}\n", doc.command_hits));
    md.push_str(&format!("- issue_count: {}\n\n", doc.issue_count));

    if doc.issues.is_empty() {
        md.push_str("No command drift issues found.\n");
        return md;
    }

    md.push_str("## Issues\n\n");
    for issue in &doc.issues {
        md.push_str(&format!("- {}:{} [{}]\n", issue.path, issue.line, issue.severity));
        md.push_str(&format!("  - detail: {}\n", issue.detail));
        md.push_str(&format!("  - command: {}\n", issue.command));
    }
    md
}