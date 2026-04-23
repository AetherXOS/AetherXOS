use anyhow::{Result, bail};
use std::fs;

use crate::constants;
use crate::utils::report;

use super::REPORT_SCHEMA_VERSION;
use super::models::{SbatReport, SbatRow};

pub fn sbat_validate(strict: bool) -> Result<()> {
    println!(
        "[secureboot::sbat] Validating SBAT metadata (strict={})",
        strict
    );

    let efi_dir = constants::paths::boot_image_iso_root().join("EFI/BOOT");
    let report_path = constants::paths::secureboot_sbat_report();
    let required = ["shimx64.efi", "grubx64.efi"];

    let mut rows = Vec::new();
    let mut failures = Vec::new();
    let mut error_codes = Vec::new();

    for name in &required {
        let path = efi_dir.join(name);
        if !path.exists() {
            failures.push(format!("missing required EFI: {}", name));
            error_codes.push("SBAT_REQUIRED_EFI_MISSING".to_string());
            rows.push(SbatRow {
                file: name.to_string(),
                exists: false,
                has_sbat: false,
            });
            continue;
        }
        let data = fs::read(&path)?;
        let has_sbat =
            data.windows(5).any(|w| w == b"sbat,") || data.windows(5).any(|w| w == b".sbat");
        rows.push(SbatRow {
            file: name.to_string(),
            exists: true,
            has_sbat,
        });
        if !has_sbat {
            failures.push(format!("sbat marker missing: {}", name));
            error_codes.push("SBAT_MARKER_MISSING".to_string());
        }
    }

    let ok = failures.is_empty();
    let status = if ok {
        "PASS"
    } else if strict {
        "FAIL"
    } else {
        "WARN"
    };

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
        bail!(
            "secureboot sbat validation failed; see {}",
            report_path.display()
        );
    }
    Ok(())
}
