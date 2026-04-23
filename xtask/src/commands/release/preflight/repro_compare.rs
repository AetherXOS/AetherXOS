use anyhow::{Context, Result, bail};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::config;
use crate::utils::{paths, report};

use super::{ReproducibleBuildEvidence, parse_csv_lower, relative_display, reproducible_evidence};

#[derive(Serialize)]
struct ReproCompareMismatch {
    input_path: String,
    host_os: String,
    host_arch: String,
    file: String,
    baseline_sha256: String,
    candidate_sha256: String,
}

#[derive(Serialize)]
struct ReproCompareDoc {
    generated_utc: String,
    strict: bool,
    overall_ok: bool,
    baseline_host: String,
    baseline_commit: String,
    host_matrix_required: Vec<String>,
    host_matrix_missing: Vec<String>,
    scanned_inputs: usize,
    loaded_inputs: usize,
    parse_errors: Vec<String>,
    mismatch_count: usize,
    mismatches: Vec<ReproCompareMismatch>,
}

pub(super) fn run(strict: bool, host_matrix: Option<&str>, inputs: Option<&str>) -> Result<()> {
    println!("[release::repro-compare] Comparing reproducible-build evidence across hosts");
    let root = paths::repo_root();

    reproducible_evidence()?;

    let baseline_path = root.join(config::repo_paths::REPRO_BUILD_EVIDENCE_JSON);
    let baseline_text = fs::read_to_string(&baseline_path).with_context(|| {
        format!(
            "failed reading baseline evidence: {}",
            baseline_path.display()
        )
    })?;
    let baseline: ReproducibleBuildEvidence =
        serde_json::from_str(&baseline_text).with_context(|| {
            format!(
                "failed parsing baseline evidence: {}",
                baseline_path.display()
            )
        })?;

    let candidate_paths = collect_inputs(&root, inputs)?;
    let mut parse_errors = Vec::new();
    let mut loaded = Vec::<(String, ReproducibleBuildEvidence)>::new();

    for path in &candidate_paths {
        let text = match fs::read_to_string(path) {
            Ok(value) => value,
            Err(err) => {
                parse_errors.push(format!("{}: {}", path.display(), err));
                continue;
            }
        };
        match serde_json::from_str::<ReproducibleBuildEvidence>(&text) {
            Ok(doc) => loaded.push((relative_display(&root, path), doc)),
            Err(err) => parse_errors.push(format!("{}: {}", path.display(), err)),
        }
    }

    let baseline_hashes = baseline
        .files
        .iter()
        .filter_map(|file| {
            if file.exists {
                file.sha256
                    .as_ref()
                    .map(|hash| (file.path.clone(), hash.clone()))
            } else {
                None
            }
        })
        .collect::<BTreeMap<_, _>>();

    let mut mismatches = Vec::new();
    for (input_path, doc) in &loaded {
        let candidate_hashes = doc
            .files
            .iter()
            .filter_map(|file| {
                if file.exists {
                    file.sha256
                        .as_ref()
                        .map(|hash| (file.path.clone(), hash.clone()))
                } else {
                    None
                }
            })
            .collect::<BTreeMap<_, _>>();

        for (rel_file, baseline_sha) in &baseline_hashes {
            if let Some(candidate_sha) = candidate_hashes.get(rel_file) {
                if candidate_sha != baseline_sha {
                    mismatches.push(ReproCompareMismatch {
                        input_path: input_path.clone(),
                        host_os: doc.host_os.clone(),
                        host_arch: doc.host_arch.clone(),
                        file: rel_file.clone(),
                        baseline_sha256: baseline_sha.clone(),
                        candidate_sha256: candidate_sha.clone(),
                    });
                }
            }
        }
    }

    let required_hosts = parse_csv_lower(host_matrix.unwrap_or(""));
    let mut observed_hosts = BTreeSet::new();
    observed_hosts.insert(baseline.host_os.to_ascii_lowercase());
    for (_, doc) in &loaded {
        observed_hosts.insert(doc.host_os.to_ascii_lowercase());
    }
    let missing_hosts = required_hosts
        .iter()
        .filter(|host| !observed_hosts.contains(*host))
        .cloned()
        .collect::<Vec<_>>();

    let overall_ok = mismatches.is_empty() && parse_errors.is_empty() && missing_hosts.is_empty();
    let report_obj = ReproCompareDoc {
        generated_utc: report::utc_now_iso(),
        strict,
        overall_ok,
        baseline_host: format!("{} {}", baseline.host_os, baseline.host_arch),
        baseline_commit: baseline
            .git_commit
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
        host_matrix_required: required_hosts,
        host_matrix_missing: missing_hosts,
        scanned_inputs: candidate_paths.len(),
        loaded_inputs: loaded.len(),
        parse_errors,
        mismatch_count: mismatches.len(),
        mismatches,
    };

    let out_json = root.join(config::repo_paths::REPRO_COMPARE_JSON);
    let out_md = root.join(config::repo_paths::REPRO_COMPARE_MD);
    report::write_json_report(&out_json, &report_obj)?;
    report::write_text_report(&out_md, &render_md(&report_obj))?;

    if strict && !report_obj.overall_ok {
        bail!(
            "strict reproducibility-compare failed: mismatches={} parse_errors={} missing_hosts={}. See {}",
            report_obj.mismatch_count,
            report_obj.parse_errors.len(),
            report_obj.host_matrix_missing.len(),
            out_json.display()
        );
    }

    println!("[release::repro-compare] PASS");
    Ok(())
}

fn collect_inputs(root: &Path, inputs: Option<&str>) -> Result<Vec<PathBuf>> {
    if let Some(raw) = inputs {
        let mut result = Vec::new();
        for item in raw.split(',').map(|v| v.trim()).filter(|v| !v.is_empty()) {
            result.push(root.join(item));
        }
        return Ok(result);
    }

    let mut found = Vec::new();
    let nightly_root = root.join("artifacts/nightly_runs");
    if nightly_root.exists() {
        for entry in WalkDir::new(&nightly_root)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file()
                && path
                    .file_name()
                    .and_then(|v| v.to_str())
                    .map(|name| name == "reproducible_build_evidence.json")
                    .unwrap_or(false)
            {
                found.push(path.to_path_buf());
            }
        }
    }

    let reports_root = root.join("reports");
    if reports_root.exists() {
        for entry in WalkDir::new(&reports_root)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file()
                && path
                    .file_name()
                    .and_then(|v| v.to_str())
                    .map(|name| name == "reproducible_build_evidence.json")
                    .unwrap_or(false)
            {
                found.push(path.to_path_buf());
            }
        }
    }

    found.sort();
    found.dedup();
    found.retain(|path| {
        path.strip_prefix(root)
            .map(|rel| rel != Path::new(config::repo_paths::REPRO_BUILD_EVIDENCE_JSON))
            .unwrap_or(true)
    });

    Ok(found)
}

fn render_md(doc: &ReproCompareDoc) -> String {
    let mut md = String::new();
    md.push_str("# Reproducibility Compare\n\n");
    md.push_str(&format!("- generated_utc: {}\n", doc.generated_utc));
    md.push_str(&format!("- strict: {}\n", doc.strict));
    md.push_str(&format!("- overall_ok: {}\n", doc.overall_ok));
    md.push_str(&format!("- baseline_host: {}\n", doc.baseline_host));
    md.push_str(&format!("- baseline_commit: {}\n", doc.baseline_commit));
    md.push_str(&format!("- scanned_inputs: {}\n", doc.scanned_inputs));
    md.push_str(&format!("- loaded_inputs: {}\n", doc.loaded_inputs));
    md.push_str(&format!("- mismatch_count: {}\n", doc.mismatch_count));
    md.push_str(&format!("- parse_errors: {}\n", doc.parse_errors.len()));
    md.push_str(&format!(
        "- host_matrix_missing: {}\n\n",
        doc.host_matrix_missing.len()
    ));

    if !doc.host_matrix_required.is_empty() {
        md.push_str("## Host Matrix\n\n");
        md.push_str(&format!(
            "- required: {}\n",
            doc.host_matrix_required.join(", ")
        ));
        md.push_str(&format!(
            "- missing: {}\n\n",
            if doc.host_matrix_missing.is_empty() {
                "none".to_string()
            } else {
                doc.host_matrix_missing.join(", ")
            }
        ));
    }

    if !doc.parse_errors.is_empty() {
        md.push_str("## Parse Errors\n\n");
        for error in &doc.parse_errors {
            md.push_str(&format!("- {}\n", error));
        }
        md.push('\n');
    }

    if !doc.mismatches.is_empty() {
        md.push_str("## Mismatches\n\n");
        for mismatch in &doc.mismatches {
            md.push_str(&format!(
                "- {} [{} {}] {}\n",
                mismatch.input_path, mismatch.host_os, mismatch.host_arch, mismatch.file
            ));
            md.push_str(&format!("  - baseline: {}\n", mismatch.baseline_sha256));
            md.push_str(&format!("  - candidate: {}\n", mismatch.candidate_sha256));
        }
    }

    md
}
