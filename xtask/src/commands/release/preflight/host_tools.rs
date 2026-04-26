use anyhow::{Result, bail};
use regex::Regex;
use serde::Serialize;

use crate::config;
use crate::utils::{paths, process, report};

#[derive(Serialize)]
pub struct HostToolCheck {
    pub id: String,
    pub required: bool,
    pub found: bool,
    pub detected_binary: Option<String>,
    pub detected_version: Option<String>,
    pub min_version: Option<String>,
    pub version_ok: Option<bool>,
    pub detail: String,
    pub remediation: String,
}

#[derive(Serialize)]
pub struct HostToolVerifyReport {
    pub generated_utc: String,
    pub strict: bool,
    pub overall_ok: bool,
    pub required_missing: usize,
    pub checks: Vec<HostToolCheck>,
}

pub fn host_tool_verify(strict: bool) -> Result<()> {
    println!("[release::host-tool-verify] Checking host toolchain/runtime dependencies");
    let root = paths::repo_root();

    type ToolSpec<'a> = (&'a str, bool, &'a [&'a str], Option<&'a str>, &'a str);
    let specs: [ToolSpec; 7] = [
        (
            "rustc",
            true,
            &["rustc"],
            Some("1.85.0"),
            "Install Rust toolchain and ensure rustc is available on PATH",
        ),
        (
            "cargo",
            true,
            &["cargo"],
            Some("1.85.0"),
            "Install Rust cargo and ensure cargo is available on PATH",
        ),
        (
            "git",
            true,
            &["git"],
            Some("2.40.0"),
            "Install Git and ensure git is available on PATH",
        ),
        (
            "qemu-system-x86_64",
            true,
            &["qemu-system-x86_64", "qemu-system-x86_64.exe"],
            None,
            "Install QEMU system package and expose qemu-system-x86_64 on PATH",
        ),
        (
            "qemu-img",
            true,
            &["qemu-img", "qemu-img.exe"],
            None,
            "Install QEMU image utilities and expose qemu-img on PATH",
        ),
        (
            "xorriso",
            true,
            &["xorriso", "xorriso.exe"],
            None,
            "Install xorriso for ISO generation workflows",
        ),
        (
            "python",
            false,
            &["python", "python3"],
            None,
            "Install Python for optional reporting/migration tooling",
        ),
    ];

    let mut checks = Vec::with_capacity(specs.len());
    for (id, required, binaries, min_version, remediation) in specs {
        let detected = process::first_available_binary(binaries).map(|bin| bin.to_string());
        let found = detected.is_some();
        let detected_version = detected
            .as_deref()
            .and_then(|binary| capture_binary_version(id, binary));
        let version_ok = match (detected_version.as_deref(), min_version) {
            (Some(v), Some(min)) => compare_semver_ge(v, min),
            _ => None,
        };
        let effective_ok = found && version_ok.unwrap_or(true);
        checks.push(HostToolCheck {
            id: id.to_string(),
            required,
            found: effective_ok,
            detected_binary: detected.clone(),
            detected_version: detected_version.clone(),
            min_version: min_version.map(|value| value.to_string()),
            version_ok,
            detail: if effective_ok {
                format!(
                    "found via {} version={} min={}",
                    detected.unwrap_or_else(|| "unknown".to_string()),
                    detected_version.unwrap_or_else(|| "unknown".to_string()),
                    min_version.unwrap_or("n/a")
                )
            } else {
                format!(
                    "missing or below minimum version; candidates={} detected_version={} min={}",
                    binaries.join(","),
                    detected_version.unwrap_or_else(|| "unknown".to_string()),
                    min_version.unwrap_or("n/a")
                )
            },
            remediation: remediation.to_string(),
        });
    }

    let required_missing = checks
        .iter()
        .filter(|check| check.required && !check.found)
        .count();
    let overall_ok = required_missing == 0;

    let report_obj = HostToolVerifyReport {
        generated_utc: report::utc_now_iso(),
        strict,
        overall_ok,
        required_missing,
        checks,
    };

    let out_json = root.join(config::repo_paths::HOST_TOOL_VERIFY_JSON);
    let out_md = root.join(config::repo_paths::HOST_TOOL_VERIFY_MD);
    report::write_json_report(&out_json, &report_obj)?;
    report::write_text_report(&out_md, &render_host_tool_verify_md(&report_obj))?;

    if strict && !report_obj.overall_ok {
        bail!(
            "strict host tool verify failed: required_missing={}. See {}",
            report_obj.required_missing,
            out_json.display()
        );
    }

    println!("[release::host-tool-verify] PASS");
    Ok(())
}

fn render_host_tool_verify_md(report_obj: &HostToolVerifyReport) -> String {
    let mut md = String::new();
    md.push_str("# Host Tool Verify\n\n");
    md.push_str(&format!("- generated_utc: {}\n", report_obj.generated_utc));
    md.push_str(&format!("- strict: {}\n", report_obj.strict));
    md.push_str(&format!("- overall_ok: {}\n", report_obj.overall_ok));
    md.push_str(&format!(
        "- required_missing: {}\n\n",
        report_obj.required_missing
    ));
    md.push_str("## Checks\n\n");
    for check in &report_obj.checks {
        md.push_str(&format!(
            "- [{}] {} (required={})\n",
            if check.found { "x" } else { " " },
            check.id,
            check.required
        ));
        if let Some(version) = &check.detected_version {
            md.push_str(&format!("  - detected_version: {}\n", version));
        }
        if let Some(min_version) = &check.min_version {
            md.push_str(&format!("  - min_version: {}\n", min_version));
        }
        if let Some(version_ok) = check.version_ok {
            md.push_str(&format!("  - version_ok: {}\n", version_ok));
        }
        md.push_str(&format!("  - detail: {}\n", check.detail));
        md.push_str(&format!("  - remediation: {}\n", check.remediation));
    }
    md
}

fn capture_binary_version(id: &str, binary: &str) -> Option<String> {
    let args = if id == "rustc" || id == "cargo" {
        vec!["-V"]
    } else {
        vec!["--version"]
    };
    let output = std::process::Command::new(binary)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let version_re = Regex::new(r"(\d+)\.(\d+)\.(\d+)").ok()?;
    let captures = version_re.captures(&text)?;
    Some(format!(
        "{}.{}.{}",
        captures.get(1)?.as_str(),
        captures.get(2)?.as_str(),
        captures.get(3)?.as_str()
    ))
}

fn compare_semver_ge(actual: &str, required: &str) -> Option<bool> {
    let parse = |value: &str| -> Option<(u32, u32, u32)> {
        let mut parts = value.split('.');
        let major = parts.next()?.parse::<u32>().ok()?;
        let minor = parts.next()?.parse::<u32>().ok()?;
        let patch = parts.next()?.parse::<u32>().ok()?;
        Some((major, minor, patch))
    };
    let actual_v = parse(actual)?;
    let required_v = parse(required)?;
    Some(actual_v >= required_v)
}
