use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_json::Value;
use std::fs;

use crate::commands::release::preflight;
use crate::config;
use crate::utils::{paths, report};

#[derive(Serialize)]
pub(crate) struct SupportCheck {
    pub(crate) id: String,
    pub(crate) ok: bool,
    pub(crate) detail: String,
}

#[derive(Serialize)]
pub(crate) struct SupportDiagnosticsDoc {
    pub(crate) generated_utc: String,
    pub(crate) strict: bool,
    pub(crate) overall_ok: bool,
    pub(crate) commands: Vec<String>,
    pub(crate) status: Vec<SupportCheck>,
}

pub(crate) fn execute(strict: bool) -> Result<()> {
    println!("[release::support-diagnostics] Building support diagnostics bundle");
    let root = paths::repo_root();

    preflight::release_doctor(false)?;
    super::gate_report::execute(None, false)?;
    preflight::export_junit(None, false)?;

    let specs = [
        ("doctor", config::repo_paths::RELEASE_DOCTOR_JSON),
        ("diagnostics", config::repo_paths::RELEASE_DIAGNOSTICS_JSON),
        ("ci_bundle", config::repo_paths::CI_BUNDLE_JSON),
        ("gate_report", config::repo_paths::CI_GATE_REPORT_JSON),
    ];

    let mut status = Vec::new();
    for (id, rel) in specs {
        let path = root.join(rel);
        if !path.exists() {
            status.push(SupportCheck {
                id: id.to_string(),
                ok: false,
                detail: format!("missing {}", rel),
            });
            continue;
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed reading support input: {}", path.display()))?;
        let doc: Value = serde_json::from_str(&text)
            .with_context(|| format!("failed parsing support input: {}", path.display()))?;
        let ok = doc
            .get("overall_ok")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        status.push(SupportCheck {
            id: id.to_string(),
            ok,
            detail: if ok {
                "overall_ok=true".to_string()
            } else {
                "overall_ok=false".to_string()
            },
        });
    }

    let commands = vec![
        "cargo run -p xtask -- release gate-fixup".to_string(),
        "cargo run -p xtask -- release doctor --strict".to_string(),
        "cargo run -p xtask -- release ci-bundle --strict".to_string(),
        "cargo run -p xtask -- release explain-failure".to_string(),
    ];
    let overall_ok = status.iter().all(|item| item.ok);

    let doc = SupportDiagnosticsDoc {
        generated_utc: report::utc_now_iso(),
        strict,
        overall_ok,
        commands,
        status,
    };

    let out_json = root.join(config::repo_paths::SUPPORT_DIAGNOSTICS_JSON);
    let out_md = root.join(config::repo_paths::SUPPORT_DIAGNOSTICS_MD);
    report::write_json_report(&out_json, &doc)?;
    report::write_text_report(&out_md, &render_support_diagnostics_md(&doc))?;

    if strict && !doc.overall_ok {
        bail!(
            "strict support-diagnostics failed; unresolved checks remain. See {}",
            out_json.display()
        );
    }

    println!("[release::support-diagnostics] PASS");
    Ok(())
}

fn render_support_diagnostics_md(doc: &SupportDiagnosticsDoc) -> String {
    let mut md = String::new();
    md.push_str("# Support Diagnostics\n\n");
    md.push_str(&format!("- generated_utc: {}\n", doc.generated_utc));
    md.push_str(&format!("- strict: {}\n", doc.strict));
    md.push_str(&format!("- overall_ok: {}\n\n", doc.overall_ok));
    md.push_str("## Status\n\n");
    for item in &doc.status {
        md.push_str(&format!(
            "- [{}] {} ({})\n",
            if item.ok { "x" } else { " " },
            item.id,
            item.detail
        ));
    }
    md.push_str("\n## Suggested Commands\n\n");
    for cmd in &doc.commands {
        md.push_str(&format!("- {}\n", cmd));
    }
    md
}
