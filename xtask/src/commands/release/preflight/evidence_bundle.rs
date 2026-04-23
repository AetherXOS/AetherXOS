use anyhow::{Result, bail};
use serde_json::Value;

use crate::config;
use crate::utils::{paths, report};

use super::{ReleaseEvidenceBundle, build_file_entry, evaluate_gate, render_bundle_md};

pub(super) fn run(strict: bool) -> Result<()> {
    println!("[release::evidence-bundle] Building release evidence bundle");

    let root = paths::repo_root();
    let specs = [
        (config::repo_paths::P_TIER_STATUS_JSON, true),
        (
            config::repo_paths::PRODUCTION_ACCEPTANCE_SCORECARD_JSON,
            true,
        ),
        (config::repo_paths::REPRO_BUILD_EVIDENCE_JSON, true),
        (config::repo_paths::SYSCALL_COVERAGE_SUMMARY, true),
        (config::repo_paths::HOST_TOOL_VERIFY_JSON, true),
        (config::repo_paths::CRITICAL_POLICY_GUARD_JSON, true),
        (config::repo_paths::WARNING_AUDIT_JSON, true),
        (config::repo_paths::RELEASE_DIAGNOSTICS_JSON, false),
        (config::repo_paths::ABI_READINESS_SUMMARY, false),
        (config::repo_paths::ERRNO_CONFORMANCE_SUMMARY, false),
        (config::repo_paths::SHIM_ERRNO_SUMMARY, false),
    ];

    let mut entries = Vec::with_capacity(specs.len());
    for (path, required) in specs {
        entries.push(build_file_entry(&root, path, required)?);
    }

    for entry in &mut entries {
        if entry.exists && entry.path.ends_with(".json") {
            let json_path = root.join(&entry.path);
            let text = std::fs::read_to_string(&json_path).map_err(anyhow::Error::from)?;
            let doc: Value = serde_json::from_str(&text).map_err(anyhow::Error::from)?;
            if let Some((ok, detail)) = evaluate_gate(&entry.path, &doc) {
                entry.gate_ok = Some(ok);
                entry.gate_detail = Some(detail);
            }
        }
    }

    let missing_required = entries
        .iter()
        .filter(|entry| entry.required && !entry.exists)
        .map(|entry| entry.path.clone())
        .collect::<Vec<_>>();
    let failing_required_gates = entries
        .iter()
        .filter(|entry| entry.required && entry.gate_ok == Some(false))
        .map(|entry| {
            format!(
                "{} ({})",
                entry.path,
                entry
                    .gate_detail
                    .clone()
                    .unwrap_or_else(|| "gate failed".to_string())
            )
        })
        .collect::<Vec<_>>();

    let bundle = ReleaseEvidenceBundle {
        generated_utc: report::utc_now_iso(),
        strict,
        overall_ok: missing_required.is_empty() && failing_required_gates.is_empty(),
        required_missing: missing_required.len(),
        required_gate_failures: failing_required_gates.len(),
        missing_required,
        failing_required_gates,
        entries,
    };

    let out_json = root.join(config::repo_paths::RELEASE_EVIDENCE_BUNDLE_JSON);
    let out_md = root.join(config::repo_paths::RELEASE_EVIDENCE_BUNDLE_MD);
    report::write_json_report(&out_json, &bundle)?;
    report::write_text_report(&out_md, &render_bundle_md(&bundle))?;

    if strict && !bundle.overall_ok {
        bail!(
            "strict release evidence bundle failed: required_missing={} required_gate_failures={}. See {}",
            bundle.required_missing,
            bundle.required_gate_failures,
            out_json.display()
        );
    }

    println!("[release::evidence-bundle] PASS");
    Ok(())
}
