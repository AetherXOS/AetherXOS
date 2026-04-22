use anyhow::Result;
use sha2::{Digest, Sha256};
use std::fs;

use crate::constants;
use crate::utils::paths;
use crate::utils::report;

use super::models::PcrReport;

pub fn pcr_report() -> Result<()> {
    println!("[secureboot::pcr] Generating TPM PCR / event-log summary");

    let event_log = paths::resolve("artifacts/tpm/eventlog.bin");
    let report_path = constants::paths::secureboot_pcr_report();

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
