use crate::utils::core::config;
use crate::utils::fs::hash::{HashAlgo, hash_file};
use crate::utils::ui::orchestrator::MULTI_PROGRESS;
use anyhow::{Result, bail};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::thread;
use std::time::Duration;

pub fn download_file(url: &str, destination: &Path) -> Result<()> {
    if !url.starts_with("https://") {
        bail!(
            "Insecure URL blocked: {}. Only HTTPS is allowed for premium builds.",
            url
        );
    }

    let client = reqwest::blocking::Client::builder()
        .use_rustls_tls()
        .build()?;

    // We use a .part file for atomic-like resume and final move
    let part_path = destination.with_extension("part");

    let mut downloaded: u64 = 0;
    let mut file = if part_path.exists() {
        downloaded = fs::metadata(&part_path)?.len();
        fs::OpenOptions::new().append(true).open(&part_path)?
    } else {
        fs::File::create(&part_path)?
    };

    let mut response = client
        .get(url)
        .header(reqwest::header::RANGE, format!("bytes={}-", downloaded))
        .send()?;

    if response.status() == reqwest::StatusCode::OK && downloaded > 0 {
        downloaded = 0;
        file = fs::File::create(&part_path)?;
    } else if !response.status().is_success()
        && response.status() != reqwest::StatusCode::PARTIAL_CONTENT
    {
        if response.status() == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
            // Already finished or invalid range?
            // If part exists, let's assume it might be done or we need a clean start
        } else {
            bail!("Download failed: HTTP {}", response.status());
        }
    }

    let content_length = response.content_length().unwrap_or(0);
    let total_size = content_length + downloaded;

    if total_size == downloaded && downloaded > 0 {
        // Already fully downloaded in .part
        fs::rename(&part_path, destination)?;
        return Ok(());
    }

    let pb = MULTI_PROGRESS.add(ProgressBar::new(total_size));
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
        .progress_chars("#>-"));
    pb.set_position(downloaded);

    let mut buffer = [0u8; 32768]; // 32KB buffer for speed
    loop {
        let n = response.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        file.write_all(&buffer[..n])?;
        downloaded += n as u64;
        pb.set_position(downloaded);
    }

    pb.finish_and_clear();

    // Final integrity check before rename
    if fs::metadata(&part_path)?.len() < total_size {
        bail!("Download incomplete: size mismatch for {}", url);
    }

    fs::rename(&part_path, destination)?;
    Ok(())
}

pub fn download_if_needed(
    url: &str,
    destination: &Path,
    expected_size: Option<u64>,
    expected_hash: Option<(&str, HashAlgo)>,
) -> Result<()> {
    if destination.exists() {
        let actual_size = fs::metadata(destination)?.len();
        let mut ok = true;

        if let Some(size) = expected_size {
            if actual_size != size {
                ok = false;
            }
        }

        if ok {
            if let Some((hash, algo)) = expected_hash {
                let actual_hash = hash_file(destination, algo)?;
                if actual_hash != hash {
                    ok = false;
                }
            }
        }

        if ok {
            return Ok(());
        }

        // If not ok, we don't necessarily delete (might resume),
        // but for distro ISOs we might want a clean start if hash fails.
        if let Some((_, _)) = expected_hash {
            fs::remove_file(destination)?;
        }
    }

    download_file(url, destination)?;

    // Verify after download if hash provided
    if let Some((hash, algo)) = expected_hash {
        let actual_hash = hash_file(destination, algo)?;
        if actual_hash != hash {
            fs::remove_file(destination).ok();
            bail!("Post-download security check failed: Hash mismatch!");
        }
    }

    Ok(())
}

/// Download with limited retries and exponential backoff for transient failures.
pub fn download_with_retries(url: &str, destination: &Path, attempts: usize) -> Result<()> {
    let backoff_base = config::download_backoff_base_secs();
    let mut last_err = None;
    for i in 0..attempts {
        match download_file(url, destination) {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_err = Some(e);
                let wait = backoff_base.saturating_mul(2u64.pow(i as u32));
                thread::sleep(Duration::from_secs(wait));
            }
        }
    }

    if let Some(e) = last_err {
        Err(e)
    } else {
        Err(anyhow::anyhow!("Unknown download failure for {}", url))
    }
}

/// Convenience wrapper that uses configured attempt count.
pub fn download_with_configured_retries(url: &str, destination: &Path) -> Result<()> {
    let attempts = config::max_download_attempts();
    download_with_retries(url, destination, attempts)
}
