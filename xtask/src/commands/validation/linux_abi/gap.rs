use anyhow::Result;
use serde::Serialize;
use std::collections::HashSet;
use std::fs;

use crate::utils::{paths, report};
use crate::config;

const STUB_TOKENS: &[&str] = &[
    "linux_nosys()",
    "errno::ENOSYS",
    "linux_errno(linux::ENOSYS)",
];

const PARTIAL_TOKENS: &[&str] = &[
    "errno::EOPNOTSUPP",
    "linux_errno(linux::EOPNOTSUPP)",
];

#[derive(Serialize, Clone)]
pub struct GapEntry {
    pub category: String,
    pub token: String,
    pub file: String,
    pub line: usize,
    pub function: String,
}

#[derive(Serialize)]
pub struct GapSummary {
    pub scanned_files: usize,
    pub total_gaps: usize,
    pub stub_count: usize,
    pub partial_or_feature_gated_count: usize,
}

#[derive(Serialize)]
pub struct GapReport {
    pub summary: GapSummary,
    pub entries: Vec<GapEntry>,
}

pub fn run() -> Result<()> {
    println!("[linux-abi::gap-inventory] Scanning ABI gap inventory from native Rust");

    let root = paths::repo_root();
    let out_dir = paths::resolve("reports/abi_gap_inventory");
    paths::ensure_dir(&out_dir)?;

    let targets = &[
        config::KERNEL_COMPAT_PATH,
        config::KERNEL_SHIM_PATH,
    ];

    let mut entries: Vec<GapEntry> = Vec::new();
    let mut scanned = 0usize;

    for base in targets {
        let dir = root.join(base);
        if !dir.exists() { continue; }
        for file in walkdir::WalkDir::new(&dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|x| x == "rs").unwrap_or(false))
        {
            scanned += 1;
            let text = fs::read_to_string(file.path()).unwrap_or_default();
            let rel = file.path().strip_prefix(&root)
                .unwrap_or(file.path())
                .to_string_lossy()
                .replace('\\', "/");

            scan_tokens(&text, &rel, "stub", STUB_TOKENS, &mut entries);
            scan_tokens(&text, &rel, "partial_or_feature_gated", PARTIAL_TOKENS, &mut entries);
        }
    }

    // Deduplicate
    let mut seen = HashSet::new();
    entries.retain(|e| {
        let key = format!("{}:{}:{}:{}", e.file, e.category, e.line, e.function);
        if seen.contains(&key) {
            false
        } else {
            seen.insert(key);
            true
        }
    });

    // Filter noise
    entries.retain(|e| {
        !matches!(e.function.as_str(), "<module_scope>" | "linux_nosys" | "no_sys" | "sys_linux_shim")
            && !e.function.starts_with("test_")
    });

    let stub_count = entries.iter().filter(|e| e.category == "stub").count();
    let partial_count = entries.iter().filter(|e| e.category == "partial_or_feature_gated").count();

    let summary = GapSummary {
        scanned_files: scanned,
        total_gaps: entries.len(),
        stub_count,
        partial_or_feature_gated_count: partial_count,
    };

    let payload = GapReport { summary, entries };
    report::write_json_report(&out_dir.join("summary.json"), &payload)?;

    let mut md = String::from("# Linux ABI Gap Inventory\n\n");
    md.push_str(&format!("- scanned_files: {}\n", scanned));
    md.push_str(&format!("- total_gaps: {}\n", payload.summary.total_gaps));
    md.push_str(&format!("- stub_count: {}\n", stub_count));
    md.push_str(&format!("- partial_or_feature_gated_count: {}\n\n", partial_count));
    md.push_str("| Category | Function | File | Line | Token |\n");
    md.push_str("|---|---|---|---:|---|\n");
    for e in &payload.entries {
        md.push_str(&format!("| {} | {} | {} | {} | {} |\n", e.category, e.function, e.file, e.line, e.token));
    }
    fs::write(out_dir.join("summary.md"), md)?;

    println!("[linux-abi::gap-inventory] PASS ({} gaps in {} files)", payload.summary.total_gaps, scanned);
    Ok(())
}

fn scan_tokens(text: &str, file: &str, category: &str, tokens: &[&str], out: &mut Vec<GapEntry>) {
    let find_fn = |offset: usize, text: &str| -> String {
        let before = &text[..offset];
        if let Some(fn_pos) = before.rfind(" fn ") {
            let after_fn = &before[fn_pos + 4..];
            let name_end = after_fn.find('(').unwrap_or(after_fn.len());
            let name = after_fn[..name_end].trim();
            if !name.is_empty() {
                return name.to_string();
            }
        }
        "<module_scope>".to_string()
    };

    for token in tokens {
        let mut start = 0;
        while let Some(idx) = text[start..].find(token) {
            let abs_idx = start + idx;
            let line = text[..abs_idx].chars().filter(|&c| c == '\n').count() + 1;
            out.push(GapEntry {
                category: category.to_string(),
                token: token.to_string(),
                file: file.to_string(),
                line,
                function: find_fn(abs_idx, text),
            });
            start = abs_idx + token.len();
        }
    }
}
