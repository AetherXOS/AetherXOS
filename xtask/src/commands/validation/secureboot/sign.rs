use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::constants;
use crate::utils::paths;
use crate::utils::process;
use crate::utils::report;

use super::REPORT_SCHEMA_VERSION;
use super::models::{SignReport, SignRow};

pub fn sign(dry_run: bool, strict_verify: bool) -> Result<()> {
    println!(
        "[secureboot::sign] Signing EFI binaries (dry_run={}, strict_verify={})",
        dry_run, strict_verify
    );

    let efi_dir = constants::paths::boot_image_iso_root().join("EFI/BOOT");
    let out_dir = constants::paths::secureboot_signed_dir();
    let report_path = constants::paths::secureboot_sign_report();

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
            "sbsign selected but key/cert not found; set AETHERCORE_SB_KEY and AETHERCORE_SB_CERT"
                .to_string(),
        );
        error_codes.push("SIGN_SBSIGN_MATERIAL_MISSING".to_string());
    }
    if !dry_run && tool == "pesign" && pesign_profile.is_none() {
        failures.push(
            "pesign selected but cert profile missing; set AETHERCORE_PESIGN_CERT and optional AETHERCORE_PESIGN_NSS_DIR"
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
                        failures.push(format!(
                            "sbsign failed for {} (rc={})",
                            file_name,
                            status.code().unwrap_or(-1)
                        ));
                        error_codes.push("SIGN_SBSIGN_EXEC_FAILED".to_string());
                    }
                    let (verified, verify_detail) = if signed {
                        verify_signed_artifact(&dst, "sbsign", strict_verify)
                    } else {
                        (false, "not signed".to_string())
                    };
                    if signed && !verified {
                        failures.push(format!(
                            "sbsign verification failed for {}: {}",
                            file_name, verify_detail
                        ));
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
                        cmd.args(["-n"]).arg(nss);
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
                        failures.push(format!(
                            "pesign verification failed for {}: {}",
                            file_name, verify_detail
                        ));
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
    println!(
        "[secureboot::sign] {}",
        if summary.ok { "PASS" } else { "FAIL" }
    );
    if summary.ok {
        Ok(())
    } else {
        bail!("secureboot sign failed; see {}", report_path.display())
    }
}

fn resolve_sbsign_material() -> Option<(PathBuf, PathBuf)> {
    if let (Ok(key), Ok(cert)) = (
        std::env::var("AETHERCORE_SB_KEY"),
        std::env::var("AETHERCORE_SB_CERT"),
    ) {
        let key_path = PathBuf::from(key);
        let cert_path = PathBuf::from(cert);
        if key_path.exists() && cert_path.exists() {
            return Some((key_path, cert_path));
        }
    }

    let candidates = [
        (paths::resolve("keys/db.key"), paths::resolve("keys/db.crt")),
        (
            paths::resolve("keys/MOK.key"),
            paths::resolve("keys/MOK.crt"),
        ),
    ];

    candidates
        .into_iter()
        .find(|(key, cert)| key.exists() && cert.exists())
}

fn resolve_pesign_profile() -> Option<(String, Option<PathBuf>)> {
    let cert_name = std::env::var("AETHERCORE_PESIGN_CERT").ok()?;
    if cert_name.trim().is_empty() {
        return None;
    }

    let nss_dir = std::env::var("AETHERCORE_PESIGN_NSS_DIR")
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
            match Command::new("sbverify").args(["--list"]).arg(path).status() {
                Ok(status) if status.success() => {
                    (true, "signature verified with sbverify".to_string())
                }
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
                Ok(status) if status.success() => (
                    true,
                    "signature verified with pesign --show-signature".to_string(),
                ),
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
                (
                    true,
                    "pesign not found for verification; skipped".to_string(),
                )
            }
        }
    } else {
        (false, "unknown signing tool".to_string())
    }
}
