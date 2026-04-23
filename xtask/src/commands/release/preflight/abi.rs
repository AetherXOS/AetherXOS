use anyhow::{Context, Result, bail};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::config;
use crate::utils::{paths, report};

#[derive(Serialize, Deserialize, Clone)]
pub struct AbiConstEntry {
    pub scope: String,
    pub name: String,
    pub value: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AbiSnapshot {
    pub generated_utc: String,
    pub source_files: Vec<String>,
    pub entries: Vec<AbiConstEntry>,
}

#[derive(Serialize, Clone)]
pub struct AbiChange {
    pub scope: String,
    pub name: String,
    pub old_value: Option<u64>,
    pub new_value: Option<u64>,
}

#[derive(Serialize)]
pub struct AbiDriftReport {
    pub generated_utc: String,
    pub baseline_path: String,
    pub baseline_created: bool,
    pub overall_ok: bool,
    pub added: Vec<AbiChange>,
    pub removed: Vec<AbiChange>,
    pub changed: Vec<AbiChange>,
    pub baseline_count: usize,
    pub current_count: usize,
}

pub fn abi_drift_report(baseline: Option<&str>, strict: bool) -> Result<()> {
    println!("[release::abi-drift] Building ABI drift report");
    let root = paths::repo_root();
    let baseline_rel = baseline.unwrap_or(config::repo_paths::ABI_DRIFT_BASELINE_JSON);
    let baseline_path = root.join(baseline_rel);

    let current = collect_abi_snapshot(&root)?;

    let baseline_exists = baseline_path.exists();
    if !baseline_exists {
        report::write_json_report(&baseline_path, &current)?;
    }

    let baseline_snapshot = if baseline_exists {
        let text = fs::read_to_string(&baseline_path)
            .with_context(|| format!("failed reading ABI baseline: {}", baseline_path.display()))?;
        serde_json::from_str::<AbiSnapshot>(&text)
            .with_context(|| format!("failed parsing ABI baseline: {}", baseline_path.display()))?
    } else {
        current.clone()
    };

    let baseline_map = baseline_snapshot
        .entries
        .iter()
        .map(|entry| ((entry.scope.clone(), entry.name.clone()), entry.value))
        .collect::<BTreeMap<_, _>>();
    let current_map = current
        .entries
        .iter()
        .map(|entry| ((entry.scope.clone(), entry.name.clone()), entry.value))
        .collect::<BTreeMap<_, _>>();

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();

    for ((scope, name), value) in &current_map {
        match baseline_map.get(&(scope.clone(), name.clone())) {
            None => added.push(AbiChange {
                scope: scope.clone(),
                name: name.clone(),
                old_value: None,
                new_value: Some(*value),
            }),
            Some(old) if old != value => changed.push(AbiChange {
                scope: scope.clone(),
                name: name.clone(),
                old_value: Some(*old),
                new_value: Some(*value),
            }),
            _ => {}
        }
    }

    for ((scope, name), value) in &baseline_map {
        if !current_map.contains_key(&(scope.clone(), name.clone())) {
            removed.push(AbiChange {
                scope: scope.clone(),
                name: name.clone(),
                old_value: Some(*value),
                new_value: None,
            });
        }
    }

    let overall_ok = added.is_empty() && removed.is_empty() && changed.is_empty();
    let drift = AbiDriftReport {
        generated_utc: report::utc_now_iso(),
        baseline_path: baseline_rel.to_string(),
        baseline_created: !baseline_exists,
        overall_ok,
        added,
        removed,
        changed,
        baseline_count: baseline_snapshot.entries.len(),
        current_count: current.entries.len(),
    };

    let out_json = root.join(config::repo_paths::ABI_DRIFT_REPORT_JSON);
    let out_md = root.join(config::repo_paths::ABI_DRIFT_REPORT_MD);
    report::write_json_report(&out_json, &drift)?;
    report::write_text_report(&out_md, &render_abi_drift_md(&drift))?;

    if strict && !drift.overall_ok {
        bail!(
            "strict ABI drift gate failed: added={} removed={} changed={}. See {}",
            drift.added.len(),
            drift.removed.len(),
            drift.changed.len(),
            out_json.display()
        );
    }

    println!("[release::abi-drift] PASS");
    Ok(())
}

fn collect_abi_snapshot(root: &Path) -> Result<AbiSnapshot> {
    let syscalls_consts = root.join("kernel/src/kernel/syscalls/syscalls_consts.rs");
    let linux_numbers = root.join("kernel/src/kernel/syscalls/syscalls_consts/linux_numbers.rs");

    let nr_entries = parse_const_block(&syscalls_consts, r"pub\s+mod\s+nr\s*\{", "nr")?;
    let linux_nr_entries =
        parse_const_block(&linux_numbers, r"pub\s+mod\s+linux_nr\s*\{", "linux_nr")?;

    let mut entries = nr_entries;
    entries.extend(linux_nr_entries);
    entries.sort_by(|a, b| {
        a.scope
            .cmp(&b.scope)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.value.cmp(&b.value))
    });

    Ok(AbiSnapshot {
        generated_utc: report::utc_now_iso(),
        source_files: vec![
            "kernel/src/kernel/syscalls/syscalls_consts.rs".to_string(),
            "kernel/src/kernel/syscalls/syscalls_consts/linux_numbers.rs".to_string(),
        ],
        entries,
    })
}

fn parse_const_block(
    path: &Path,
    module_header_pattern: &str,
    scope: &str,
) -> Result<Vec<AbiConstEntry>> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed reading ABI source file: {}", path.display()))?;

    let module_re = Regex::new(module_header_pattern)?;
    let start = module_re
        .find(&text)
        .map(|m| m.end())
        .with_context(|| format!("missing module block in {}", path.display()))?;
    let block_tail = text
        .get(start..)
        .with_context(|| format!("failed slicing module block in {}", path.display()))?;

    let mut depth = 1usize;
    let mut end_idx = None;
    for (idx, ch) in block_tail.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    end_idx = Some(idx);
                    break;
                }
            }
            _ => {}
        }
    }

    let close_idx =
        end_idx.with_context(|| format!("unterminated module block in {}", path.display()))?;
    let scanned = block_tail
        .get(..close_idx)
        .with_context(|| format!("failed slicing module body in {}", path.display()))?;

    let const_re = Regex::new(r"pub\s+const\s+([A-Z0-9_]+)\s*:\s*usize\s*=\s*(\d+)\s*;")?;
    let mut entries = Vec::new();
    for cap in const_re.captures_iter(scanned) {
        let name = cap
            .get(1)
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        let value = cap
            .get(2)
            .and_then(|m| m.as_str().parse::<u64>().ok())
            .unwrap_or(0);
        entries.push(AbiConstEntry {
            scope: scope.to_string(),
            name,
            value,
        });
    }
    Ok(entries)
}

fn render_abi_drift_md(report_obj: &AbiDriftReport) -> String {
    let mut md = String::new();
    md.push_str("# ABI Drift Report\n\n");
    md.push_str(&format!("- generated_utc: {}\n", report_obj.generated_utc));
    md.push_str(&format!("- baseline_path: {}\n", report_obj.baseline_path));
    md.push_str(&format!(
        "- baseline_created: {}\n",
        report_obj.baseline_created
    ));
    md.push_str(&format!("- overall_ok: {}\n", report_obj.overall_ok));
    md.push_str(&format!(
        "- baseline_count: {}\n",
        report_obj.baseline_count
    ));
    md.push_str(&format!("- current_count: {}\n", report_obj.current_count));
    md.push_str(&format!("- added: {}\n", report_obj.added.len()));
    md.push_str(&format!("- removed: {}\n", report_obj.removed.len()));
    md.push_str(&format!("- changed: {}\n\n", report_obj.changed.len()));

    if !report_obj.added.is_empty() {
        md.push_str("## Added\n\n");
        for item in &report_obj.added {
            md.push_str(&format!(
                "- {}::{} => {}\n",
                item.scope,
                item.name,
                item.new_value.unwrap_or_default()
            ));
        }
        md.push('\n');
    }

    if !report_obj.removed.is_empty() {
        md.push_str("## Removed\n\n");
        for item in &report_obj.removed {
            md.push_str(&format!(
                "- {}::{} (was {})\n",
                item.scope,
                item.name,
                item.old_value.unwrap_or_default()
            ));
        }
        md.push('\n');
    }

    if !report_obj.changed.is_empty() {
        md.push_str("## Changed\n\n");
        for item in &report_obj.changed {
            md.push_str(&format!(
                "- {}::{} {} -> {}\n",
                item.scope,
                item.name,
                item.old_value.unwrap_or_default(),
                item.new_value.unwrap_or_default()
            ));
        }
        md.push('\n');
    }

    md
}
