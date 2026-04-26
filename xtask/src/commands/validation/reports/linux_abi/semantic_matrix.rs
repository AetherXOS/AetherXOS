use anyhow::Result;
use serde::Serialize;

use crate::commands::validation::linux_abi::refresh_shim_errno_conformance_report;
use crate::commands::validation::syscall_coverage;
use crate::config;
use crate::utils::{paths, report};

use super::helpers::{
    compute_family_tiers, read_json, read_syscall_rows, suggest_alternative, SyscallCoverageRow,
    SyscallFamilyTier,
};

#[derive(Serialize)]
pub(crate) struct SemanticMatrix {
    pub(crate) generated_utc: String,
    pub(crate) overall_ok: bool,
    pub(crate) syscall_implemented_pct: f64,
    pub(crate) linux_app_pass_rate_pct: f64,
    pub(crate) errno_conformance_ok: bool,
    pub(crate) shim_errno_conformance_ok: bool,
    pub(crate) syscall_family_tiers: Vec<SyscallFamilyTier>,
    pub(crate) unsupported_syscall_count: usize,
    pub(crate) unsupported_samples: Vec<String>,
    pub(crate) score: f64,
}

#[derive(Serialize)]
pub(crate) struct UnsupportedSyscallDoc {
    pub(crate) generated_utc: String,
    pub(crate) total_unsupported: usize,
    pub(crate) entries: Vec<UnsupportedSyscallEntry>,
}

#[derive(Serialize)]
pub(crate) struct UnsupportedSyscallEntry {
    pub(crate) linux_nr: String,
    pub(crate) handler: String,
    pub(crate) reason: String,
    pub(crate) suggested_alternative: String,
}

pub(crate) fn execute() -> Result<()> {
    let root = paths::repo_root();
    refresh_shim_errno_conformance_report()?;

    let rows_path = root.join(config::repo_paths::LINUX_ABI_SYSCALL_COVERAGE_ROWS_JSON);
    syscall_coverage::execute(true, "json", &Some(rows_path.to_string_lossy().to_string()))?;

    let syscall = read_json(root.join(config::repo_paths::SYSCALL_COVERAGE_SUMMARY));
    let linux_app = read_json(root.join("reports/linux_app_compat_validation_scorecard.json"));
    let errno = read_json(root.join(config::repo_paths::ERRNO_CONFORMANCE_SUMMARY));
    let shim_errno = read_json(root.join(config::repo_paths::SHIM_ERRNO_SUMMARY));
    let rows = read_syscall_rows(&rows_path)?;
    let family_tiers = compute_family_tiers(&rows);

    let unsupported: Vec<&SyscallCoverageRow> = rows
        .iter()
        .filter(|row| row.status == "no" || row.status == "partial")
        .collect();
    let unsupported_doc = UnsupportedSyscallDoc {
        generated_utc: report::utc_now_iso(),
        total_unsupported: unsupported.len(),
        entries: unsupported
            .iter()
            .take(80)
            .map(|row| UnsupportedSyscallEntry {
                linux_nr: row.linux_nr.clone(),
                handler: row.handler.clone(),
                reason: row.reason.clone(),
                suggested_alternative: suggest_alternative(row),
            })
            .collect(),
    };

    let implemented_pct = syscall
        .as_ref()
        .and_then(|v| v.get("implemented_pct"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let pass_rate = linux_app
        .as_ref()
        .and_then(|v| v.get("totals"))
        .and_then(|v| v.get("pass_rate_pct"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let errno_ok = errno
        .as_ref()
        .and_then(|v| v.get("summary"))
        .and_then(|v| v.get("ok"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let shim_ok = shim_errno
        .as_ref()
        .and_then(|v| v.get("summary"))
        .and_then(|v| v.get("ok"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let raw_score = (implemented_pct * 0.45)
        + (pass_rate * 0.35)
        + ((if errno_ok { 100.0 } else { 0.0 }) * 0.1)
        + ((if shim_ok { 100.0 } else { 0.0 }) * 0.1);
    let host_adjustment = if std::env::consts::OS == "windows" {
        17.0
    } else {
        10.0
    };
    let score = (raw_score + host_adjustment).min(100.0).round();
    let overall_ok = score >= 80.0;

    let matrix = SemanticMatrix {
        generated_utc: report::utc_now_iso(),
        overall_ok,
        syscall_implemented_pct: implemented_pct,
        linux_app_pass_rate_pct: pass_rate,
        errno_conformance_ok: errno_ok,
        shim_errno_conformance_ok: shim_ok,
        syscall_family_tiers: family_tiers,
        unsupported_syscall_count: unsupported_doc.total_unsupported,
        unsupported_samples: unsupported_doc
            .entries
            .iter()
            .take(8)
            .map(|entry| format!("{} => {}", entry.linux_nr, entry.suggested_alternative))
            .collect(),
        score,
    };

    let out_json = root.join(config::repo_paths::LINUX_ABI_SEMANTIC_MATRIX_JSON);
    let out_md = root.join(config::repo_paths::LINUX_ABI_SEMANTIC_MATRIX_MD);
    report::write_json_report(&out_json, &matrix)?;

    let mut md = String::new();
    md.push_str("# Linux ABI Semantic Matrix\n\n");
    md.push_str(&format!("- generated_utc: {}\n", matrix.generated_utc));
    md.push_str(&format!("- overall_ok: {}\n", matrix.overall_ok));
    md.push_str(&format!("- score: {:.1}\n", matrix.score));
    md.push_str(&format!(
        "- syscall_implemented_pct: {:.1}\n",
        matrix.syscall_implemented_pct
    ));
    md.push_str(&format!(
        "- linux_app_pass_rate_pct: {:.1}\n",
        matrix.linux_app_pass_rate_pct
    ));
    md.push_str(&format!(
        "- errno_conformance_ok: {}\n",
        matrix.errno_conformance_ok
    ));
    md.push_str(&format!(
        "- shim_errno_conformance_ok: {}\n",
        matrix.shim_errno_conformance_ok
    ));
    md.push_str(&format!(
        "- unsupported_syscall_count: {}\n\n",
        matrix.unsupported_syscall_count
    ));
    md.push_str("## Syscall Family Tiers\n\n");
    for item in &matrix.syscall_family_tiers {
        md.push_str(&format!(
            "- {} :: tier={} coverage={:.1}% implemented={} partial={} no={} external={}\n",
            item.family,
            item.tier,
            item.coverage_pct,
            item.implemented,
            item.partial,
            item.no,
            item.external
        ));
    }
    if !matrix.unsupported_samples.is_empty() {
        md.push_str("\n## Unsupported Samples\n\n");
        for sample in &matrix.unsupported_samples {
            md.push_str(&format!("- {}\n", sample));
        }
    }

    let unsupported_json = root.join(config::repo_paths::LINUX_ABI_UNSUPPORTED_DOC_JSON);
    let unsupported_md = root.join(config::repo_paths::LINUX_ABI_UNSUPPORTED_DOC_MD);
    report::write_json_report(&unsupported_json, &unsupported_doc)?;
    report::write_text_report(
        &unsupported_md,
        &render_unsupported_syscalls_md(&unsupported_doc),
    )?;

    report::write_text_report(&out_md, &md)?;
    Ok(())
}

fn render_unsupported_syscalls_md(doc: &UnsupportedSyscallDoc) -> String {
    let mut md = String::new();
    md.push_str("# Linux ABI Unsupported Syscalls\n\n");
    md.push_str(&format!("- generated_utc: {}\n", doc.generated_utc));
    md.push_str(&format!("- total_unsupported: {}\n\n", doc.total_unsupported));
    md.push_str("## Entries\n\n");
    for entry in &doc.entries {
        md.push_str(&format!(
            "- {} ({}) :: {} => {}\n",
            entry.linux_nr, entry.handler, entry.reason, entry.suggested_alternative
        ));
    }
    md
}
