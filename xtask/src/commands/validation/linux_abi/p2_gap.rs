use anyhow::Result;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use regex::Regex;

use crate::utils::{paths, report};

#[derive(Serialize, Clone, Default)]
pub struct MarkerCounts {
    pub todo: usize,
    pub fixme: usize,
    pub mock: usize,
    pub stub: usize,
    pub unimplemented: usize,
    pub todo_macro: usize,
}

pub fn run_report() -> Result<()> {
    println!("[linux-abi::p2-gap] Generating P2 gap report from source markers");

    let root = paths::repo_root();
    let out_dir = paths::resolve("reports/p2_gap");
    paths::ensure_dir(&out_dir)?;

    let markers = [
        ("todo", Regex::new(r"\bTODO\b")?),
        ("fixme", Regex::new(r"\bFIXME\b")?),
        ("mock", Regex::new(r"(?i)\bmock\b")?),
        ("stub", Regex::new(r"(?i)\bstub\b")?),
        ("unimplemented", Regex::new(r"(?i)\bunimplemented!\b")?),
        ("todo_macro", Regex::new(r"(?i)\btodo!\b")?),
    ];

    let targets = vec!["src", "scripts", "docs"];
    let mut total_counts = MarkerCounts::default();
    let mut by_module: HashMap<String, MarkerCounts> = HashMap::new();

    for target in targets {
        let dir = root.join(target);
        if !dir.exists() { continue; }

        for entry in walkdir::WalkDir::new(&dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
        {
            let path = entry.path();
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            if !matches!(ext, "rs" | "py" | "ps1" | "md" | "toml") { continue; }

            let text = fs::read_to_string(path).unwrap_or_default();
            let mut file_counts = MarkerCounts::default();

            for line in text.lines() {
                // Heuristic: only count markers in comments for code files
                let is_comment = if ext == "rs" {
                    let trimmed = line.trim();
                    trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("*")
                } else {
                    true // count all for script/md files
                };
                if !is_comment { continue; }

                if markers[0].1.is_match(line) { file_counts.todo += 1; total_counts.todo += 1; }
                if markers[1].1.is_match(line) { file_counts.fixme += 1; total_counts.fixme += 1; }
                if markers[2].1.is_match(line) { file_counts.mock += 1; total_counts.mock += 1; }
                if markers[3].1.is_match(line) { file_counts.stub += 1; total_counts.stub += 1; }
                if markers[4].1.is_match(line) { file_counts.unimplemented += 1; total_counts.unimplemented += 1; }
                if markers[5].1.is_match(line) { file_counts.todo_macro += 1; total_counts.todo_macro += 1; }
            }

            let rel = path.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/");
            let module = if rel.starts_with("src/") {
                rel.split('/').nth(1).unwrap_or("core").to_string()
            } else {
                rel.split('/').next().unwrap().to_string()
            };

            let m = by_module.entry(module).or_default();
            m.todo += file_counts.todo;
            m.fixme += file_counts.fixme;
            m.mock += file_counts.mock;
            m.stub += file_counts.stub;
            m.unimplemented += file_counts.unimplemented;
            m.todo_macro += file_counts.todo_macro;
        }
    }

    let payload = serde_json::json!({
        "summary": {
            "totals": total_counts,
            "total_markers": total_counts.todo + total_counts.fixme + total_counts.mock + total_counts.stub + total_counts.unimplemented + total_counts.todo_macro,
            "by_module": by_module,
        }
    });

    report::write_json_report(&out_dir.join("summary.json"), &payload)?;
    println!("[linux-abi::p2-gap] P2 gap report generated with {} total markers", payload["summary"]["total_markers"]);
    Ok(())
}
