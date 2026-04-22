use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

use crate::constants;
use crate::utils::paths;
use crate::utils::process;
use crate::utils::report;

use super::models::{OvmfCaseResult, OvmfSummary};

pub fn ovmf_matrix(dry_run: bool) -> Result<()> {
    println!(
        "[secureboot::ovmf] Running OVMF Secure Boot matrix (dry_run={})",
        dry_run
    );
    let out_dir = constants::paths::secureboot_ovmf_matrix_dir();
    paths::ensure_dir(&out_dir)?;
    let summary_path = out_dir.join("summary.json");

    if dry_run {
        let summary = ovmf_dry_run_summary(report::utc_now_iso());
        report::write_json_report(&summary_path, &summary)?;
        println!(
            "[secureboot::ovmf] DRY-RUN summary={}",
            summary_path.display()
        );
        return Ok(());
    }

    let qemu = find_qemu()?;
    let iso = paths::resolve("artifacts/boot_image/aethercore.iso");
    let ovmf_code = constants::paths::ovmf_dir().join("OVMF_CODE.fd");
    let ovmf_vars = constants::paths::ovmf_dir().join("OVMF_VARS.fd");

    let mut failures = Vec::new();
    if !iso.exists() {
        failures.push(format!("iso missing: {}", iso.display()));
    }
    if !ovmf_code.exists() {
        failures.push(format!("ovmf code missing: {}", ovmf_code.display()));
    }
    if !ovmf_vars.exists() {
        failures.push(format!("ovmf vars missing: {}", ovmf_vars.display()));
    }

    let mut rows = Vec::new();
    if failures.is_empty() {
        rows.push(run_ovmf_case(
            &qemu, &iso, &ovmf_code, &ovmf_vars, false, &out_dir, 25,
        )?);
        rows.push(run_ovmf_case(
            &qemu, &iso, &ovmf_code, &ovmf_vars, true, &out_dir, 25,
        )?);
    }

    for row in &rows {
        if !row.ok {
            failures.push(format!(
                "{}: rc={} timeout={}",
                row.name, row.rc, row.timeout
            ));
        }
    }

    let summary = OvmfSummary {
        generated_utc: report::utc_now_iso(),
        ok: failures.is_empty(),
        dry_run: false,
        rows,
        failures,
    };
    report::write_json_report(&summary_path, &summary)?;
    println!(
        "[secureboot::ovmf] {} summary={}",
        if summary.ok { "PASS" } else { "FAIL" },
        summary_path.display()
    );

    if summary.ok {
        Ok(())
    } else {
        bail!("secureboot OVMF matrix failed")
    }
}

fn ovmf_dry_run_summary(generated_utc: String) -> OvmfSummary {
    OvmfSummary {
        generated_utc,
        ok: true,
        dry_run: true,
        rows: Vec::new(),
        failures: Vec::new(),
    }
}

fn run_ovmf_case(
    qemu: &str,
    iso: &Path,
    ovmf_code: &Path,
    ovmf_vars_template: &Path,
    secure_boot: bool,
    out_dir: &Path,
    timeout_sec: u64,
) -> Result<OvmfCaseResult> {
    let case_name = if secure_boot {
        "secure_on"
    } else {
        "secure_off"
    };
    let vars_copy = out_dir.join(format!("OVMF_VARS_{}.fd", case_name));
    let log_path = out_dir.join(format!("{}.log", case_name));
    fs::copy(ovmf_vars_template, &vars_copy).with_context(|| {
        format!(
            "Failed to copy OVMF vars template to {}",
            vars_copy.display()
        )
    })?;

    let mut child = Command::new(qemu)
        .args([
            "-nographic",
            "-m",
            "1024",
            "-smp",
            "2",
            "-drive",
            &format!(
                "if=pflash,format=raw,readonly=on,file={}",
                ovmf_code.display()
            ),
            "-drive",
            &format!("if=pflash,format=raw,file={}", vars_copy.display()),
            "-cdrom",
            &iso.to_string_lossy(),
            "-boot",
            "d",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to launch QEMU case {}", case_name))?;

    let start = Instant::now();
    let (rc, timeout, output) = match process::wait_child_with_timeout(
        &mut child,
        std::time::Duration::from_secs(timeout_sec),
        std::time::Duration::from_millis(100),
    ) {
        Ok(Some(status)) => {
            let out = process::read_optional_pipe_to_string(child.stdout.take());
            (status.code().unwrap_or(-1), false, out)
        }
        Ok(None) | Err(_) => {
            let _ = child.kill();
            let out = process::read_optional_pipe_to_string(child.stdout.take());
            (-1, true, out)
        }
    };
    let duration_sec = start.elapsed().as_secs_f64();

    report::write_text_report(&log_path, &output)
        .with_context(|| format!("Failed writing OVMF case log {}", log_path.display()))?;
    let ok = rc == 0 && !timeout;

    Ok(OvmfCaseResult {
        name: case_name.to_string(),
        secure_boot,
        ok,
        rc,
        timeout,
        duration_sec,
        log_path: log_path.display().to_string(),
    })
}

fn find_qemu() -> Result<String> {
    process::find_qemu_system_x86_64()
        .ok_or_else(|| anyhow::anyhow!("qemu-system-x86_64 not found in PATH or Program Files"))
}
