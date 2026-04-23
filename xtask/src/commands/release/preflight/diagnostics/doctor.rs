use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::fs;

use crate::config;
use crate::utils::{paths, report};

use crate::commands::release::preflight::models::{BundleCheck, DoctorReport};

pub fn execute(strict: bool) -> Result<()> {
    println!("[release::doctor] Running release doctor checks");
    let root = paths::repo_root();

    crate::commands::release::preflight::host_tools::host_tool_verify(false)?;
    super::policy_guard::execute(false)?;
    super::warning_audit::execute(false, None)?;
    super::release::execute(false)?;

    let specs = [
        (
            "host_tool_verify",
            config::repo_paths::HOST_TOOL_VERIFY_JSON,
        ),
        (
            "critical_policy_guard",
            config::repo_paths::CRITICAL_POLICY_GUARD_JSON,
        ),
        ("warning_audit", config::repo_paths::WARNING_AUDIT_JSON),
        (
            "release_diagnostics",
            config::repo_paths::RELEASE_DIAGNOSTICS_JSON,
        ),
        (
            "release_evidence_bundle",
            config::repo_paths::RELEASE_EVIDENCE_BUNDLE_JSON,
        ),
    ];

    let mut checks = Vec::new();
    for (id, rel) in specs {
        let path = root.join(rel);
        if !path.exists() {
            checks.push(BundleCheck {
                id: id.to_string(),
                ok: false,
                detail: format!("missing {}", rel),
            });
            continue;
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed reading doctor input: {}", path.display()))?;
        let doc: Value = serde_json::from_str(&text)
            .with_context(|| format!("failed parsing doctor input: {}", path.display()))?;
        let ok = doc
            .get("overall_ok")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        checks.push(BundleCheck {
            id: id.to_string(),
            ok,
            detail: if ok {
                "overall_ok=true".to_string()
            } else {
                "overall_ok=false".to_string()
            },
        });
    }

    let overall_ok = checks.iter().all(|check| check.ok);
    let report_obj = DoctorReport {
        generated_utc: report::utc_now_iso(),
        strict,
        overall_ok,
        checks,
    };

    let out_json = root.join(config::repo_paths::RELEASE_DOCTOR_JSON);
    let out_md = root.join(config::repo_paths::RELEASE_DOCTOR_MD);
    report::write_json_report(&out_json, &report_obj)?;
    report::write_text_report(&out_md, &render_doctor_md(&report_obj))?;

    if strict && !report_obj.overall_ok {
        bail!(
            "strict release doctor failed: one or more checks are red. See {}",
            out_json.display()
        );
    }

    println!("[release::doctor] PASS");
    Ok(())
}

fn render_doctor_md(report_obj: &DoctorReport) -> String {
    let mut md = String::new();
    md.push_str("# Release Doctor\n\n");
    md.push_str(&format!("- generated_utc: {}\n", report_obj.generated_utc));
    md.push_str(&format!("- strict: {}\n", report_obj.strict));
    md.push_str(&format!("- overall_ok: {}\n\n", report_obj.overall_ok));
    md.push_str("## Checks\n\n");
    for check in &report_obj.checks {
        md.push_str(&format!(
            "- [{}] {} ({})\n",
            if check.ok { "x" } else { " " },
            check.id,
            check.detail
        ));
    }
    md
}
