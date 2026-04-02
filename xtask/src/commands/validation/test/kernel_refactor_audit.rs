use anyhow::Result;
use regex::Regex;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;

use crate::constants;
use crate::utils::{paths, report};

#[derive(Serialize)]
struct FileHotspot {
    path: String,
    line_count: usize,
    use_count: usize,
    magic_candidates: Vec<MagicCandidate>,
}

#[derive(Serialize, Clone)]
struct MagicCandidate {
    literal: String,
    count: usize,
}

#[derive(Serialize)]
struct AuditSummary {
    scanned_files: usize,
    long_file_count: usize,
    hotspot_count: usize,
    max_lines_threshold: usize,
    magic_repeat_threshold: usize,
}

#[derive(Serialize)]
struct AuditReport {
    summary: AuditSummary,
    long_files: Vec<FileHotspot>,
    top_coupling_files: Vec<FileHotspot>,
}

pub fn run(max_lines: usize, magic_repeat_threshold: usize) -> Result<()> {
    println!("[test::kernel-refactor-audit] Scanning kernel structure hotspots");

    let root = paths::repo_root();
    let src_kernel = paths::kernel_src("kernel");

    let number_re = Regex::new(r"\b(?:0x[0-9a-fA-F]+|\d{2,})\b")?;

    let mut scanned = 0usize;
    let mut long_files = Vec::new();
    let mut all_files = Vec::new();

    for entry in walkdir::WalkDir::new(&src_kernel)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter(|e| e.path().extension().map(|ext| ext == "rs").unwrap_or(false))
    {
        scanned += 1;
        let abs = entry.path();
        let rel = abs
            .strip_prefix(&root)
            .unwrap_or(abs)
            .to_string_lossy()
            .replace('\\', "/");

        let text = fs::read_to_string(abs).unwrap_or_default();
        let line_count = text.lines().count();
        let use_count = text
            .lines()
            .filter(|l| l.trim_start().starts_with("use "))
            .count();

        let mut counts: HashMap<String, usize> = HashMap::new();
        for cap in number_re.captures_iter(&text) {
            let lit = cap[0].to_string();
            // Ignore clearly safe tiny literals.
            if lit == "0" || lit == "1" || lit == "2" || lit == "3" || lit == "4" {
                continue;
            }
            *counts.entry(lit).or_insert(0) += 1;
        }

        let mut magic_candidates: Vec<MagicCandidate> = counts
            .into_iter()
            .filter(|(_, c)| *c >= magic_repeat_threshold)
            .map(|(literal, count)| MagicCandidate { literal, count })
            .collect();
        magic_candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.count));
        if magic_candidates.len() > 10 {
            magic_candidates.truncate(10);
        }

        let hotspot = FileHotspot {
            path: rel,
            line_count,
            use_count,
            magic_candidates,
        };

        if hotspot.line_count >= max_lines {
            long_files.push(FileHotspot {
                path: hotspot.path.clone(),
                line_count: hotspot.line_count,
                use_count: hotspot.use_count,
                magic_candidates: hotspot.magic_candidates.clone(),
            });
        }

        all_files.push(hotspot);
    }

    all_files.sort_by_key(|file| std::cmp::Reverse(file.use_count));
    let top_coupling_files = all_files.into_iter().take(20).collect::<Vec<_>>();

    long_files.sort_by_key(|file| std::cmp::Reverse(file.line_count));

    let summary = AuditSummary {
        scanned_files: scanned,
        long_file_count: long_files.len(),
        hotspot_count: long_files.len() + top_coupling_files.len(),
        max_lines_threshold: max_lines,
        magic_repeat_threshold,
    };

    let report_obj = AuditReport {
        summary,
        long_files,
        top_coupling_files,
    };

    let out_dir = constants::paths::kernel_refactor_audit_dir();
    paths::ensure_dir(&out_dir)?;
    report::write_json_report(&out_dir.join("summary.json"), &report_obj)?;

    let mut md = String::new();
    md.push_str("# Kernel Refactor Audit\n\n");
    md.push_str(&format!(
        "- scanned_files: {}\n",
        report_obj.summary.scanned_files
    ));
    md.push_str(&format!(
        "- long_file_count: {}\n",
        report_obj.summary.long_file_count
    ));
    md.push_str(&format!(
        "- max_lines_threshold: {}\n",
        report_obj.summary.max_lines_threshold
    ));
    md.push_str(&format!(
        "- magic_repeat_threshold: {}\n\n",
        report_obj.summary.magic_repeat_threshold
    ));

    md.push_str("## Long Files\n\n");
    if report_obj.long_files.is_empty() {
        md.push_str("No long files above threshold.\n\n");
    } else {
        for file in &report_obj.long_files {
            md.push_str(&format!(
                "- {} (lines={}, uses={})\n",
                file.path, file.line_count, file.use_count
            ));
            for m in &file.magic_candidates {
                md.push_str(&format!(
                    "  - magic candidate: {} (count={})\n",
                    m.literal, m.count
                ));
            }
        }
        md.push('\n');
    }

    md.push_str("## Top Coupling Files (by use-count)\n\n");
    for file in &report_obj.top_coupling_files {
        md.push_str(&format!(
            "- {} (uses={}, lines={})\n",
            file.path, file.use_count, file.line_count
        ));
    }

    report::write_text_report(&out_dir.join("summary.md"), &md)?;

    println!(
        "[test::kernel-refactor-audit] done: scanned={} long_files={}",
        report_obj.summary.scanned_files, report_obj.summary.long_file_count
    );

    Ok(())
}
