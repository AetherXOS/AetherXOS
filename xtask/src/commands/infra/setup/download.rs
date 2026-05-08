use crate::constants;
use crate::utils::{logging, paths};
use anyhow::{Context, Result, bail};
use flate2::read::GzDecoder;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

// ─── Limine Release Configuration ────────────────────────────────────────────
//
// Limine v12+ ships a single `limine-binary.tar.gz` on GitHub Releases.
//
// Actual tarball layout (verified from v12.x extraction):
//
//   limine-binary/              ← single top-level prefix dir
//     limine-bios.sys
//     limine-bios-cd.bin
//     limine-bios-hdd.h
//     limine-bios-pxe.bin
//     limine-uefi-cd.bin
//     BOOTX64.EFI
//     BOOTAA64.EFI
//     BOOTIA32.EFI
//     BOOTLOONGARCH64.EFI
//     BOOTRISCV64.EFI
//     limine.c
//     Makefile
//     LICENSE
//     limine-tool-windows-x86/  ← sub-dir (not needed here)
//
// We extract all entries directly under `limine-binary/` and copy the
// ones listed in REQUIRED_FILES to `artifacts/limine/bin/`.

const LIMINE_TARBALL_URL: &str =
    "https://github.com/limine-bootloader/limine/releases/latest/download/limine-binary.tar.gz";

/// Files we must copy to `artifacts/limine/bin/` after extraction.
/// Second field: SHA-256 expected hash (hex). Empty = skip verification.
const REQUIRED_FILES: &[(&str, &str)] = &[
    ("limine-bios.sys", ""),
    ("limine-bios-cd.bin", ""),
    ("limine-uefi-cd.bin", ""),
    ("BOOTX64.EFI", ""),
];

/// The prefix directory name inside the tarball (verified from real v12 tarball).
const TARBALL_PREFIX: &str = "limine-binary/";

// ─── Public API ──────────────────────────────────────────────────────────────

/// Fetch and install all required Limine bootloader binaries into `artifacts/limine/bin/`.
///
/// # Pipeline
/// 1. Download `limine-binary.tar.gz` from GitHub Releases (with progress bar).
/// 2. Decompress `.gz` via `flate2`.
/// 3. Walk tar entries; extract files in `limine-binary/` prefix into memory.
/// 4. Write each required file to disk atomically.
/// 5. Verify every file exists and is > 4 KiB.
/// 6. Optionally verify SHA-256 when a hash is set in `REQUIRED_FILES`.
pub fn fetch_limine_binaries() -> Result<()> {
    let dest_dir = constants::paths::limine_bin_dir();
    paths::ensure_dir(&dest_dir).context("Failed to create Limine binary directory")?;

    // ── Fast-path: user pre-extracted the tarball locally ─────────────────
    // If `artifacts/limine-binary/` exists (e.g. user manually extracted the
    // downloaded tarball), copy directly from there — no internet required.
    let local_extracted = crate::utils::paths::resolve("artifacts/limine-binary");
    if local_extracted.is_dir() {
        logging::info(
            "limine",
            "local tarball extract found — skipping download",
            &[("path", &local_extracted.to_string_lossy())],
        );
        return install_from_local_dir(&local_extracted, &dest_dir);
    }

    logging::info(
        "limine",
        "fetching Limine release tarball",
        &[
            ("url", LIMINE_TARBALL_URL),
            ("dest", &dest_dir.to_string_lossy()),
        ],
    );

    // Step 1: Download
    let tarball_bytes =
        download_tarball(LIMINE_TARBALL_URL).context("Tarball download stage failed")?;

    // Step 2+3: Decompress & walk tar
    let extracted = extract_required_files(&tarball_bytes, REQUIRED_FILES, TARBALL_PREFIX)
        .context("Tarball extraction stage failed")?;

    // Step 4: Write to disk atomically
    for (filename, data) in &extracted {
        let dest = dest_dir.join(filename);
        write_atomic(data, &dest)
            .with_context(|| format!("Failed writing '{}'", dest.display()))?;

        logging::info(
            "limine",
            "installed binary",
            &[
                ("file", filename.as_str()),
                ("bytes", &data.len().to_string()),
            ],
        );
    }

    // Step 5: Verify presence + minimum size
    verify_required_files(&dest_dir).context("Post-install verification failed")?;

    // Step 6: Optional SHA-256 verification
    hash_verify_files(&dest_dir).context("Cryptographic verification failed")?;

    logging::info(
        "limine",
        "all binaries installed and verified successfully",
        &[],
    );
    Ok(())
}

/// Copy required files from a pre-extracted `limine-binary/` directory.
fn install_from_local_dir(src_dir: &std::path::Path, dest_dir: &std::path::Path) -> Result<()> {
    logging::info(
        "limine",
        "installing from local directory",
        &[("src", &src_dir.to_string_lossy())],
    );

    for (filename, _) in REQUIRED_FILES {
        let src = src_dir.join(filename);
        if !src.exists() {
            bail!(
                "Required file '{}' not found in local extract dir '{}'.\n  \
                 Delete '{}' to trigger a fresh download.",
                filename,
                src_dir.display(),
                src_dir.display()
            );
        }

        let size = std::fs::metadata(&src)
            .with_context(|| format!("Cannot stat '{}'", src.display()))?
            .len();

        if size < 4096 {
            bail!(
                "Local binary '{}' is suspiciously small ({size} bytes — expected > 4 KiB).",
                filename
            );
        }

        let dest = dest_dir.join(filename);
        std::fs::copy(&src, &dest).with_context(|| {
            format!("Failed to copy '{}' → '{}'", src.display(), dest.display())
        })?;

        logging::info(
            "limine",
            "copied from local extract",
            &[("file", filename), ("bytes", &size.to_string())],
        );
    }

    hash_verify_files(dest_dir).context("Cryptographic verification failed")?;
    logging::info("limine", "all binaries installed from local directory", &[]);
    Ok(())
}

// ─── Download ────────────────────────────────────────────────────────────────

fn download_tarball(url: &str) -> Result<Vec<u8>> {
    let client = reqwest::blocking::Client::builder()
        .use_rustls_tls()
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .context("Failed to construct HTTPS client")?;

    let mut response = client
        .get(url)
        .send()
        .with_context(|| format!("HTTP GET failed for: {url}"))?;

    if !response.status().is_success() {
        bail!(
            "HTTP {} received for {}\n  \
             Verify the Limine release URL exists, or check your internet connection.",
            response.status(),
            url
        );
    }

    let total = response.content_length().unwrap_or(0);
    let pb = crate::utils::ui_orchestrator::MULTI_PROGRESS.add(ProgressBar::new(total));
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.cyan} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
            .progress_chars("█▒░"),
    );

    let mut buf: Vec<u8> = Vec::with_capacity(total.max(1) as usize);
    let mut chunk = [0u8; 65536];

    loop {
        let n = response
            .read(&mut chunk)
            .context("I/O error reading tarball stream")?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        pb.inc(n as u64);
    }

    pb.finish_and_clear();

    if buf.is_empty() {
        bail!("Downloaded tarball is empty — server returned no data.");
    }

    logging::info(
        "limine",
        "download complete",
        &[("url", url), ("bytes", &buf.len().to_string())],
    );

    Ok(buf)
}

// ─── Extraction ──────────────────────────────────────────────────────────────

/// Walk the tar archive and return `(filename, bytes)` for every file in
/// `wanted` that lives directly under `prefix` (e.g. `"limine-binary/"`).
fn extract_required_files(
    tarball: &[u8],
    wanted: &[(&str, &str)],
    prefix: &str,
) -> Result<Vec<(String, Vec<u8>)>> {
    logging::info(
        "limine",
        "decompressing tarball (flate2 + tar)",
        &[
            ("input_bytes", &tarball.len().to_string()),
            ("prefix", prefix),
        ],
    );

    let gz = GzDecoder::new(tarball);
    let mut archive = tar::Archive::new(gz);

    let required_names: std::collections::HashSet<&str> = wanted.iter().map(|(n, _)| *n).collect();

    let mut found: Vec<(String, Vec<u8>)> = Vec::new();
    let mut skipped_errors: Vec<String> = Vec::new();

    let entries = archive.entries().context(
        "Failed to iterate tar entries — the archive may be corrupt, \
             truncated, or not a valid tar.gz",
    )?;

    for entry_result in entries {
        let mut entry = match entry_result {
            Ok(e) => e,
            Err(e) => {
                skipped_errors.push(format!("Unreadable entry: {e}"));
                continue;
            }
        };

        let raw_path: PathBuf = match entry.path() {
            Ok(p) => p.to_path_buf(),
            Err(_) => continue,
        };

        // Only consider direct children of the prefix (not nested subdirs)
        let raw_str = raw_path.to_string_lossy();
        if !raw_str.starts_with(prefix) {
            continue;
        }

        let filename = match raw_path.file_name().and_then(|n| n.to_str()) {
            Some(f) => f,
            None => continue,
        };

        if !required_names.contains(filename) {
            continue; // Not a file we need
        }

        let mut data: Vec<u8> = Vec::new();
        entry.read_to_end(&mut data).with_context(|| {
            format!(
                "I/O error reading tar entry for '{filename}' \
                     (flate2 decompression or tar parsing failed)"
            )
        })?;

        if data.is_empty() {
            bail!(
                "Extraction fault: '{filename}' decompressed to 0 bytes. \
                 The tarball may be corrupt — re-run 'cargo xtask setup limine'."
            );
        }

        logging::info(
            "limine",
            "extracted",
            &[("file", filename), ("bytes", &data.len().to_string())],
        );

        found.push((filename.to_owned(), data));
    }

    // Report any soft errors
    for e in &skipped_errors {
        logging::warn("limine", e, &[]);
    }

    // Ensure all required files were located
    for (req, _) in wanted {
        if !found.iter().any(|(n, _)| n == req) {
            bail!(
                "Required binary '{}' was NOT found under '{}' in the tarball.\n  \
                 The Limine tarball layout may have changed. \
                 Expected prefix: '{}'. \
                 Actual tarball URL: {}",
                req,
                prefix,
                prefix,
                LIMINE_TARBALL_URL
            );
        }
    }

    Ok(found)
}

// ─── Write & Verify ──────────────────────────────────────────────────────────

/// Atomically write `data` to `dest` using a `.tmp` side-file + rename.
fn write_atomic(data: &[u8], dest: &Path) -> Result<()> {
    let tmp = dest.with_extension("tmp");

    {
        let mut f = std::fs::File::create(&tmp)
            .with_context(|| format!("Cannot create temporary file: {}", tmp.display()))?;
        f.write_all(data)
            .with_context(|| format!("Cannot write to temporary file: {}", tmp.display()))?;
    }

    std::fs::rename(&tmp, dest).with_context(|| {
        format!(
            "Atomic rename failed: {} → {}\n  \
             Ensure source and destination are on the same filesystem.",
            tmp.display(),
            dest.display()
        )
    })?;

    Ok(())
}

/// Confirm every required file exists in `dest_dir` and is > 4 KiB.
fn verify_required_files(dest_dir: &Path) -> Result<()> {
    for (filename, _) in REQUIRED_FILES {
        let path = dest_dir.join(filename);

        if !path.exists() {
            bail!(
                "Verification failed: '{}' is missing from '{}' after install.",
                filename,
                dest_dir.display()
            );
        }

        let size = std::fs::metadata(&path)
            .with_context(|| format!("Cannot stat '{}'", path.display()))?
            .len();

        if size < 4096 {
            bail!(
                "Verification failed: '{}' is suspiciously small ({size} bytes). \
                 Expected a minimum of 4 KiB for a valid Limine binary.",
                filename
            );
        }
    }
    Ok(())
}

/// Verify SHA-256 for any file in REQUIRED_FILES that has a non-empty hash.
fn hash_verify_files(dest_dir: &Path) -> Result<()> {
    for (filename, expected_hash) in REQUIRED_FILES {
        if expected_hash.is_empty() {
            logging::warn(
                "limine",
                "SHA-256 verification skipped — populate REQUIRED_FILES to harden supply chain",
                &[("file", filename)],
            );
            continue;
        }

        let actual =
            crate::utils::hash_file(&dest_dir.join(filename), crate::utils::HashAlgo::Sha256)?;

        if &actual != expected_hash {
            std::fs::remove_file(dest_dir.join(filename)).ok();
            bail!(
                "SECURITY ALERT: SHA-256 mismatch for '{}'!\n  \
                 Expected : {}\n  \
                 Actual   : {}\n  \
                 File removed. Possible supply-chain attack or disk corruption.",
                filename,
                expected_hash,
                actual
            );
        }

        logging::info("limine", "SHA-256 verified", &[("file", filename)]);
    }
    Ok(())
}
