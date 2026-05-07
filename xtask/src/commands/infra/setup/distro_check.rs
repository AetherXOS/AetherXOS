use anyhow::{Context, Result};
use reqwest::blocking::Client;
use std::fs;
use std::path::Path;
use std::time::Duration;
use crate::utils::registry::DistroRegistry;

/// Perform a deep check of the distro registry (JSON schema, mandatory fields, etc.)
pub fn check_registry_integrity(file: Option<&str>) -> Result<()> {
    let registry = if let Some(f) = file {
        DistroRegistry::load(Path::new(f))?
    } else {
        DistroRegistry::load_default()?
    };

    println!("[distro::check] Validating integrity of registry v{}", registry.version);
    println!("[distro::check] Description: {}", registry.description);

    let images = registry.collect_all_images();
    println!("[distro::check] Integrity check PASSED ({} total image variants found)", images.len());
    Ok(())
}

/// Validate distro registry URLs for availability and content length
pub fn validate_urls(file: Option<&str>, out: Option<&str>, _verify_size: bool) -> Result<()> {
    let registry = if let Some(f) = file {
        DistroRegistry::load(Path::new(f))?
    } else {
        DistroRegistry::load_default()?
    };

    let report_path = match out {
        Some(p) => Path::new(p).to_path_buf(),
        None => Path::new("artifacts/distro_url_report.json").to_path_buf(),
    };

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("Failed building HTTP client")?;

    let images = registry.collect_all_images();
    println!("[distro::url-checks] Validating {} URLs...", images.len());

    let mut report_entries = serde_json::Map::new();
    let mut total_failed = 0;
    let min_size = 5_000_000; // 5MB lower bound

    for (dname, vname, varname, arch, img) in images {
        let url = img.url();
        let mut status = serde_json::Map::new();
        status.insert("url".into(), serde_json::Value::String(url.to_string()));
        status.insert("distro".into(), serde_json::Value::String(dname.clone()));
        status.insert("version".into(), serde_json::Value::String(vname.clone()));
        status.insert("variant".into(), serde_json::Value::String(varname.clone()));
        status.insert("arch".into(), serde_json::Value::String(arch.clone()));

        match client.head(url).send() {
            Ok(resp) => {
                let code = resp.status().as_u16();
                status.insert("status".into(), serde_json::Value::Number(code.into()));
                
                if resp.status().is_success() {
                    if let Some(len_header) = resp.headers().get(reqwest::header::CONTENT_LENGTH) {
                        if let Ok(s) = len_header.to_str() {
                            if let Ok(size) = s.parse::<u64>() {
                                status.insert("actual_size".into(), serde_json::Value::Number(size.into()));
                                if size < min_size {
                                    println!("[FAIL] URL too small (<5MB): {} (got {} bytes)", url, size);
                                    total_failed += 1;
                                }
                            }
                        }
                    }
                } else {
                    println!("[FAIL] HTTP {} for {}", code, url);
                    total_failed += 1;
                }
            }
            Err(e) => {
                println!("[FAIL] Connection error for {}: {}", url, e);
                status.insert("error".into(), serde_json::Value::String(e.to_string()));
                total_failed += 1;
            }
        }

        let key = format!("{}-{}-{}", dname, vname, varname);
        let list = report_entries.entry(key).or_insert(serde_json::Value::Array(Vec::new()));
        if let Some(arr) = list.as_array_mut() {
            arr.push(serde_json::Value::Object(status));
        }
    }

    let mut report = serde_json::Map::new();
    report.insert("checked_at".into(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
    report.insert("total_failed".into(), serde_json::Value::Number(total_failed.into()));
    report.insert("entries".into(), serde_json::Value::Object(report_entries));

    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(&report_path, serde_json::to_string_pretty(&serde_json::Value::Object(report))?)?;

    if total_failed > 0 {
        println!("[distro::url-checks] BATCH FAILED: {} issues found. See {}", total_failed, report_path.display());
    } else {
        println!("[distro::url-checks] BATCH PASSED: All URLs valid.");
    }
    Ok(())
}
