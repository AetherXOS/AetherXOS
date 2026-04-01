use anyhow::{Context, Result, bail};
use serde::Serialize;
use sha2::{Sha256, Digest};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::cli::SecurebootAction;
use crate::utils::paths;
use crate::utils::process;
use crate::utils::report;

const REPORT_SCHEMA_VERSION: u32 = 1;

/// Entry point for `cargo run -p xtask -- secureboot <action>`.
pub fn execute(action: &SecurebootAction) -> Result<()> {
    match action {
        SecurebootAction::Sign {
            dry_run,
            strict_verify,
        } => sign(*dry_run, *strict_verify),
        SecurebootAction::SbatValidate { strict } => sbat_validate(*strict),
        SecurebootAction::PcrReport => pcr_report(),
        SecurebootAction::MokPlan => mok_plan(),
        SecurebootAction::OvmfMatrix { dry_run } => ovmf_matrix(*dry_run),
    }
}

// ---------------------------------------------------------------------------
// Sign EFI binaries
// Replaces: scripts/secureboot_sign.py
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct SignReport {
    schema_version: u32,
    generated_utc: String,
    ok: bool,
    tool: String,
    targets: usize,
    rows: Vec<SignRow>,
    failures: Vec<String>,
    error_codes: Vec<String>,
}

#[derive(Serialize)]
struct SignRow {
    file: String,
    signed: bool,
    verified: bool,
    tool: String,
    dry_run: bool,
    detail: String,
}

fn sign(dry_run: bool, strict_verify: bool) -> Result<()> {
    println!(
        "[secureboot::sign] Signing EFI binaries (dry_run={}, strict_verify={})",
        dry_run, strict_verify
    );

    let efi_dir = paths::resolve("artifacts/boot_image/iso_root/EFI/BOOT");
    let out_dir = paths::resolve("artifacts/secureboot/signed");
    let report_path = paths::resolve("reports/secureboot/sign_report.json");

    paths::ensure_dir(&out_dir)?;

    if !efi_dir.exists() {
        bail!("EFI directory not found: {}", efi_dir.display());
    }

    // Discover signing tool
    let tool = if dry_run {
        "dry-run".to_string()
    } else if process::which("sbsign") {
        "sbsign".to_string()
    } else if process::which("pesign") {
        "pesign".to_string()
    } else {
        "none".to_string()
    };

    // List EFI targets
    let mut targets: Vec<_> = fs::read_dir(&efi_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|x| x.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("efi"))
        })
        .collect();
    targets.sort_by_key(|e| e.file_name());

    let mut rows = Vec::new();
    let mut failures = Vec::new();
    let mut error_codes = Vec::new();

    if targets.is_empty() {
        failures.push(format!("no EFI targets found in {}", efi_dir.display()));
        error_codes.push("SIGN_NO_TARGETS".to_string());
    }

    if !dry_run && tool == "none" {
        failures.push("no signing tool available (sbsign/pesign)".to_string());
        error_codes.push("SIGN_TOOL_MISSING".to_string());
    }

    let sbsign_material = if !dry_run && tool == "sbsign" {
        resolve_sbsign_material()
    } else {
        None
    };
    let pesign_profile = if !dry_run && tool == "pesign" {
        resolve_pesign_profile()
    } else {
        None
    };
    if !dry_run && tool == "sbsign" && sbsign_material.is_none() {
        failures.push(
            "sbsign selected but key/cert not found; set HYPERCORE_SB_KEY and HYPERCORE_SB_CERT"
                .to_string(),
        );
        error_codes.push("SIGN_SBSIGN_MATERIAL_MISSING".to_string());
    }
    if !dry_run && tool == "pesign" && pesign_profile.is_none() {
        failures.push(
            "pesign selected but cert profile missing; set HYPERCORE_PESIGN_CERT and optional HYPERCORE_PESIGN_NSS_DIR"
                .to_string(),
        );
        error_codes.push("SIGN_PESIGN_PROFILE_MISSING".to_string());
    }

    for entry in &targets {
        let src = entry.path();
        let dst = out_dir.join(entry.file_name());
        let file_name = entry.file_name().to_string_lossy().to_string();

        if dry_run {
            fs::copy(&src, &dst)?;
            rows.push(SignRow {
                file: file_name,
                signed: false,
                verified: false,
                tool: tool.clone(),
                dry_run: true,
                detail: "copied only (dry-run)".to_string(),
            });
            continue;
        }

        match tool.as_str() {
            "sbsign" => {
                if let Some((key, cert)) = &sbsign_material {
                    let status = Command::new("sbsign")
                        .args([
                            "--key",
                            &key.to_string_lossy(),
                            "--cert",
                            &cert.to_string_lossy(),
                            "--output",
                            &dst.to_string_lossy(),
                            &src.to_string_lossy(),
                        ])
                        .status()
                        .with_context(|| format!("failed to execute sbsign for {}", file_name))?;
                    let signed = status.success();
                    if !signed {
                        failures.push(format!("sbsign failed for {} (rc={})", file_name, status.code().unwrap_or(-1)));
                        error_codes.push("SIGN_SBSIGN_EXEC_FAILED".to_string());
                    }
                    let (verified, verify_detail) = if signed {
                        verify_signed_artifact(&dst, "sbsign", strict_verify)
                    } else {
                        (false, "not signed".to_string())
                    };
                    if signed && !verified {
                        failures.push(format!("sbsign verification failed for {}: {}", file_name, verify_detail));
                        error_codes.push("SIGN_SBSIGN_VERIFY_FAILED".to_string());
                    }
                    rows.push(SignRow {
                        file: file_name,
                        signed,
                        verified,
                        tool: tool.clone(),
                        dry_run: false,
                        detail: if signed {
                            format!("signed with sbsign; {}", verify_detail)
                        } else {
                            format!("sbsign rc={}", status.code().unwrap_or(-1))
                        },
                    });
                } else {
                    fs::copy(&src, &dst)?;
                    rows.push(SignRow {
                        file: file_name.clone(),
                        signed: false,
                        verified: false,
                        tool: tool.clone(),
                        dry_run: false,
                        detail: "missing key/cert material".to_string(),
                    });
                    failures.push(format!("missing key/cert material for {}", file_name));
                    error_codes.push("SIGN_SBSIGN_MATERIAL_MISSING".to_string());
                }
            }
            "pesign" => {
                if let Some((cert_name, nss_dir)) = &pesign_profile {
                    let mut cmd = Command::new("pesign");
                    cmd.args(["-s", "-i"])
                        .arg(&src)
                        .args(["-o"])
                        .arg(&dst)
                        .args(["-c", cert_name]);
                    if let Some(nss) = nss_dir {
                        cmd.args(["-n"])
                            .arg(nss);
                    }

                    let status = cmd
                        .status()
                        .with_context(|| format!("failed to execute pesign for {}", file_name))?;
                    let signed = status.success();
                    if !signed {
                        failures.push(format!(
                            "pesign failed for {} (rc={})",
                            file_name,
                            status.code().unwrap_or(-1)
                        ));
                        error_codes.push("SIGN_PESIGN_EXEC_FAILED".to_string());
                    }
                    let (verified, verify_detail) = if signed {
                        verify_signed_artifact(&dst, "pesign", strict_verify)
                    } else {
                        (false, "not signed".to_string())
                    };
                    if signed && !verified {
                        failures.push(format!("pesign verification failed for {}: {}", file_name, verify_detail));
                        error_codes.push("SIGN_PESIGN_VERIFY_FAILED".to_string());
                    }
                    rows.push(SignRow {
                        file: file_name,
                        signed,
                        verified,
                        tool: tool.clone(),
                        dry_run: false,
                        detail: if signed {
                            format!("signed with pesign; {}", verify_detail)
                        } else {
                            format!("pesign rc={}", status.code().unwrap_or(-1))
                        },
                    });
                } else {
                    fs::copy(&src, &dst)?;
                    rows.push(SignRow {
                        file: file_name.clone(),
                        signed: false,
                        verified: false,
                        tool: tool.clone(),
                        dry_run: false,
                        detail: "missing pesign cert profile; file copied unsigned".to_string(),
                    });
                    failures.push(format!("pesign profile not configured for {}", file_name));
                    error_codes.push("SIGN_PESIGN_PROFILE_MISSING".to_string());
                }
            }
            _ => {
                fs::copy(&src, &dst)?;
                rows.push(SignRow {
                    file: file_name.clone(),
                    signed: false,
                    verified: false,
                    tool: tool.clone(),
                    dry_run: false,
                    detail: "no signing tool available; file copied unsigned".to_string(),
                });
                failures.push(format!("unsigned copy for {}", file_name));
                error_codes.push("SIGN_TOOL_MISSING".to_string());
            }
        }
    }

    error_codes.sort();
    error_codes.dedup();

    let summary = SignReport {
        schema_version: REPORT_SCHEMA_VERSION,
        generated_utc: report::utc_now_iso(),
        ok: failures.is_empty(),
        tool,
        targets: targets.len(),
        rows,
        failures,
        error_codes,
    };

    report::write_json_report(&report_path, &summary)?;
    println!("[secureboot::sign] {}", if summary.ok { "PASS" } else { "FAIL" });
    if summary.ok {
        Ok(())
    } else {
        bail!("secureboot sign failed; see {}", report_path.display())
    }
}

fn resolve_sbsign_material() -> Option<(PathBuf, PathBuf)> {
    if let (Ok(key), Ok(cert)) = (
        std::env::var("HYPERCORE_SB_KEY"),
        std::env::var("HYPERCORE_SB_CERT"),
    ) {
        let key_path = PathBuf::from(key);
        let cert_path = PathBuf::from(cert);
        if key_path.exists() && cert_path.exists() {
            return Some((key_path, cert_path));
        }
    }

    let candidates = [
        (paths::resolve("keys/db.key"), paths::resolve("keys/db.crt")),
        (paths::resolve("keys/MOK.key"), paths::resolve("keys/MOK.crt")),
    ];

    candidates
        .into_iter()
        .find(|(key, cert)| key.exists() && cert.exists())
}

fn resolve_pesign_profile() -> Option<(String, Option<PathBuf>)> {
    let cert_name = std::env::var("HYPERCORE_PESIGN_CERT").ok()?;
    if cert_name.trim().is_empty() {
        return None;
    }

    let nss_dir = std::env::var("HYPERCORE_PESIGN_NSS_DIR")
        .ok()
        .and_then(|v| {
            if v.trim().is_empty() {
                None
            } else {
                Some(PathBuf::from(v))
            }
        });

    Some((cert_name, nss_dir))
}

fn verify_signed_artifact(path: &Path, signing_tool: &str, strict_verify: bool) -> (bool, String) {
    if signing_tool == "sbsign" {
        if process::which("sbverify") {
            match Command::new("sbverify")
                .args(["--list"])
                .arg(path)
                .status()
            {
                Ok(status) if status.success() => (true, "signature verified with sbverify".to_string()),
                Ok(status) => (
                    false,
                    format!("sbverify failed rc={}", status.code().unwrap_or(-1)),
                ),
                Err(err) => (false, format!("sbverify execution error: {}", err)),
            }
        } else {
            if strict_verify {
                (
                    false,
                    "sbverify not found and strict_verify=true".to_string(),
                )
            } else {
                (true, "sbverify not found; verification skipped".to_string())
            }
        }
    } else if signing_tool == "pesign" {
        if process::which("pesign") {
            match Command::new("pesign")
                .args(["--show-signature", "--in"])
                .arg(path)
                .status()
            {
                Ok(status) if status.success() => {
                    (true, "signature verified with pesign --show-signature".to_string())
                }
                Ok(status) => (
                    false,
                    format!("pesign verify failed rc={}", status.code().unwrap_or(-1)),
                ),
                Err(err) => (false, format!("pesign verify execution error: {}", err)),
            }
        } else {
            if strict_verify {
                (
                    false,
                    "pesign not found for verification and strict_verify=true".to_string(),
                )
            } else {
                (true, "pesign not found for verification; skipped".to_string())
            }
        }
    } else {
        (false, "unknown signing tool".to_string())
    }
}

// ---------------------------------------------------------------------------
// SBAT validation
// Replaces: scripts/secureboot_sbat_validate.py
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct SbatReport {
    schema_version: u32,
    generated_utc: String,
    ok: bool,
    status: String,
    strict: bool,
    rows: Vec<SbatRow>,
    failures: Vec<String>,
    error_codes: Vec<String>,
}

#[derive(Serialize)]
struct SbatRow {
    file: String,
    exists: bool,
    has_sbat: bool,
}

fn sbat_validate(strict: bool) -> Result<()> {
    println!("[secureboot::sbat] Validating SBAT metadata (strict={})", strict);

    let efi_dir = paths::resolve("artifacts/boot_image/iso_root/EFI/BOOT");
    let report_path = paths::resolve("reports/secureboot/sbat_report.json");
    let required = ["shimx64.efi", "grubx64.efi"];

    let mut rows = Vec::new();
    let mut failures = Vec::new();
    let mut error_codes = Vec::new();

    for name in &required {
        let path = efi_dir.join(name);
        if !path.exists() {
            failures.push(format!("missing required EFI: {}", name));
            error_codes.push("SBAT_REQUIRED_EFI_MISSING".to_string());
            rows.push(SbatRow { file: name.to_string(), exists: false, has_sbat: false });
            continue;
        }
        let data = fs::read(&path)?;
        let has_sbat = data.windows(5).any(|w| w == b"sbat,") || data.windows(5).any(|w| w == b".sbat");
        rows.push(SbatRow { file: name.to_string(), exists: true, has_sbat });
        if !has_sbat {
            failures.push(format!("sbat marker missing: {}", name));
            error_codes.push("SBAT_MARKER_MISSING".to_string());
        }
    }

    let ok = failures.is_empty();
    let status = if ok { "PASS" } else if strict { "FAIL" } else { "WARN" };

    error_codes.sort();
    error_codes.dedup();

    let summary = SbatReport {
        schema_version: REPORT_SCHEMA_VERSION,
        generated_utc: report::utc_now_iso(),
        ok,
        status: status.to_string(),
        strict,
        rows,
        failures,
        error_codes,
    };

    report::write_json_report(&report_path, &summary)?;
    println!("[secureboot::sbat] {}", status);
    if strict && !ok {
        bail!("secureboot sbat validation failed; see {}", report_path.display());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// PCR report
// Replaces: scripts/secureboot_pcr_report.py
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct PcrReport {
    generated_utc: String,
    ok: bool,
    event_log_path: String,
    event_log_exists: bool,
    event_log_size_bytes: u64,
    event_log_sha256: String,
}

fn pcr_report() -> Result<()> {
    println!("[secureboot::pcr] Generating TPM PCR / event-log summary");

    let event_log = paths::resolve("artifacts/tpm/eventlog.bin");
    let report_path = paths::resolve("reports/secureboot/pcr_report.json");

    let exists = event_log.exists();
    let (size, hash) = if exists {
        let data = fs::read(&event_log)?;
        let mut hasher = Sha256::new();
        hasher.update(&data);
        (data.len() as u64, format!("{:x}", hasher.finalize()))
    } else {
        (0u64, String::new())
    };

    let summary = PcrReport {
        generated_utc: report::utc_now_iso(),
        ok: exists,
        event_log_path: event_log.to_string_lossy().to_string(),
        event_log_exists: exists,
        event_log_size_bytes: size,
        event_log_sha256: hash,
    };

    report::write_json_report(&report_path, &summary)?;
    println!("[secureboot::pcr] {}", if exists { "PASS" } else { "WARN" });
    Ok(())
}

// ---------------------------------------------------------------------------
// MOK enrollment plan
// Replaces: scripts/secureboot_mok_plan.py
// ---------------------------------------------------------------------------

fn mok_plan() -> Result<()> {
    println!("[secureboot::mok] Generating MOK enrollment plan");

    let cert = paths::resolve("keys/MOK.cer");
    let out_dir = paths::resolve("reports/secureboot");
    paths::ensure_dir(&out_dir)?;

    let steps = [
        "1) Copy certificate to target machine.",
        "2) Run: mokutil --import <cert>",
        "3) Reboot and enroll key in MOK Manager UI.",
        "4) Verify: mokutil --list-enrolled",
        "5) Reboot and validate shim/grub/loader path.",
    ];

    // Write markdown plan
    let mut md = String::new();
    md.push_str("# Secure Boot MOK Enrollment Plan\n\n");
    md.push_str(&format!("- cert_path: `{}`\n\n", cert.display()));
    md.push_str("## Steps\n\n");
    for step in &steps {
        md.push_str(&format!("- {}\n", step));
    }
    md.push_str("\n## Commands\n\n");
    md.push_str(&format!("- `mokutil --import {}`\n", cert.display()));
    md.push_str("- `mokutil --list-enrolled`\n");
    md.push_str("- `mokutil --test-key <cert>`\n");

    fs::write(out_dir.join("mok_plan.md"), &md)?;
    println!("[secureboot::mok] Plan written: {}", out_dir.join("mok_plan.md").display());
    Ok(())
}

// ---------------------------------------------------------------------------
// OVMF matrix
// Replaces: scripts/secureboot_ovmf_matrix.py
// ---------------------------------------------------------------------------

fn ovmf_matrix(dry_run: bool) -> Result<()> {
    println!("[secureboot::ovmf] Running OVMF Secure Boot matrix (dry_run={})", dry_run);
    let out_dir = paths::resolve("reports/secureboot/ovmf_matrix");
    paths::ensure_dir(&out_dir)?;
    let summary_path = out_dir.join("summary.json");

    if dry_run {
        let summary = OvmfSummary {
            generated_utc: report::utc_now_iso(),
            ok: true,
            dry_run: true,
            rows: Vec::new(),
            failures: Vec::new(),
        };
        report::write_json_report(&summary_path, &summary)?;
        println!("[secureboot::ovmf] DRY-RUN summary={}", summary_path.display());
        return Ok(());
    }

    let qemu = find_qemu()?;
    let iso = paths::resolve("artifacts/boot_image/hypercore.iso");
    let ovmf_code = paths::resolve("artifacts/ovmf/OVMF_CODE.fd");
    let ovmf_vars = paths::resolve("artifacts/ovmf/OVMF_VARS.fd");

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
            &qemu,
            &iso,
            &ovmf_code,
            &ovmf_vars,
            false,
            &out_dir,
            25,
        )?);
        rows.push(run_ovmf_case(
            &qemu,
            &iso,
            &ovmf_code,
            &ovmf_vars,
            true,
            &out_dir,
            25,
        )?);
    }

    for row in &rows {
        if !row.ok {
            failures.push(format!("{}: rc={} timeout={}", row.name, row.rc, row.timeout));
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

#[derive(Serialize)]
struct OvmfSummary {
    generated_utc: String,
    ok: bool,
    dry_run: bool,
    rows: Vec<OvmfCaseResult>,
    failures: Vec<String>,
}

#[derive(Serialize)]
struct OvmfCaseResult {
    name: String,
    secure_boot: bool,
    ok: bool,
    rc: i32,
    timeout: bool,
    duration_sec: f64,
    log_path: String,
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
    let case_name = if secure_boot { "secure_on" } else { "secure_off" };
    let vars_copy = out_dir.join(format!("OVMF_VARS_{}.fd", case_name));
    let log_path = out_dir.join(format!("{}.log", case_name));
    fs::copy(ovmf_vars_template, &vars_copy)
        .with_context(|| format!("Failed to copy OVMF vars template to {}", vars_copy.display()))?;

    let mut child = Command::new(qemu)
        .args([
            "-nographic",
            "-m",
            "1024",
            "-smp",
            "2",
            "-drive",
            &format!("if=pflash,format=raw,readonly=on,file={}", ovmf_code.display()),
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
    let (rc, timeout, output) = match child.wait_timeout(std::time::Duration::from_secs(timeout_sec)) {
        Ok(Some(status)) => {
            let out = child.stdout.take().map(read_pipe_to_string).unwrap_or_default();
            (status.code().unwrap_or(-1), false, out)
        }
        Ok(None) | Err(_) => {
            let _ = child.kill();
            let out = child.stdout.take().map(read_pipe_to_string).unwrap_or_default();
            (-1, true, out)
        }
    };
    let duration_sec = start.elapsed().as_secs_f64();

    fs::write(&log_path, output)
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
    if crate::utils::process::which("qemu-system-x86_64") {
        return Ok("qemu-system-x86_64".to_string());
    }

    let program_files = std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".to_string());
    let candidate = PathBuf::from(program_files)
        .join("qemu")
        .join("qemu-system-x86_64.exe");
    if candidate.exists() {
        return Ok(candidate.display().to_string());
    }

    bail!("qemu-system-x86_64 not found in PATH or Program Files")
}

fn read_pipe_to_string(mut pipe: std::process::ChildStdout) -> String {
    let mut buf = String::new();
    let _ = std::io::Read::read_to_string(&mut pipe, &mut buf);
    buf
}

trait WaitTimeout {
    fn wait_timeout(&mut self, duration: std::time::Duration) -> std::io::Result<Option<std::process::ExitStatus>>;
}

impl WaitTimeout for std::process::Child {
    fn wait_timeout(&mut self, duration: std::time::Duration) -> std::io::Result<Option<std::process::ExitStatus>> {
        let start = Instant::now();
        loop {
            match self.try_wait()? {
                Some(status) => return Ok(Some(status)),
                None => {
                    if start.elapsed() >= duration {
                        return Ok(None);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }
    }
}
