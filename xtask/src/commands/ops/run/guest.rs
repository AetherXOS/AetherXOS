use anyhow::{Result, Context};
use std::path::Path;
use crate::utils::{logging, paths, context, process};
use crate::constants;

const MIN_VALID_ROOTFS_BYTES: u64 = 1024 * 1024;

pub fn launch_guest_session(
    distro: &Option<String>,
    rootfs: &Option<String>,
    download: bool,
    cache: bool,
    refresh: bool,
    attach: bool,
) -> Result<()> {
    logging::info("run::guest", "🐧 Preparing guest image and launching interactive session", &[]);

    let outdir = context::out_dir();
    let cache_dir = outdir.join("guest_cache");
    let mut resolved: Option<String> = None;

    if let Some(path) = rootfs {
        logging::info("run::guest", &format!("Using provided rootfs: {}", path), &[]);
        if !Path::new(path).exists() {
            return Err(anyhow::anyhow!("Rootfs file not found: {}", path));
        }
        resolved = Some(path.clone());
    } else if let Some(d) = distro {
        let key = d.clone();
        let cached_path = cache_dir.join(format!("{}.tar.gz", key));

        if cached_path.exists() && !refresh && cache && cached_path.metadata()?.len() >= MIN_VALID_ROOTFS_BYTES {
            let size_mb = cached_path.metadata()?.len() / (1024 * 1024);
            logging::info("run::guest", &format!("✓ Using cached distro: {} ({} MB)", key, size_mb), &[]);
            resolved = Some(cached_path.to_string_lossy().to_string());
        } else if download {
            logging::info("run::guest", &format!("📥 Resolving distro URLs for: {}", key), &[]);
            let urls = crate::commands::ops::guest::resolve_distro_urls(&key);
            
            if urls.is_empty() {
                logging::warn("run::guest", &format!("No known URLs for distro '{}'.", key), &[]);
            } else {
                paths::ensure_dir(&cache_dir)?;
                let mut download_succeeded = false;
                
                for (idx, url) in urls.iter().enumerate() {
                    logging::info("run::guest", &format!("Trying URL [{}/{}]: {}", idx + 1, urls.len(), url), &[]);
                    
                    let out = cached_path.to_string_lossy().to_string();
                    let tmp_out = format!("{}.part", out);
                    let _ = std::fs::remove_file(&tmp_out);
                    
                    let curl_ok = process::run_best_effort("curl", &["-fsSL", "--progress-bar", url, "-o", &tmp_out]);
                    let wget_ok = if !curl_ok {
                        process::run_best_effort("wget", &["-q", "--show-progress", "-O", &tmp_out, url])
                    } else { false };

                    if curl_ok || wget_ok {
                        let size_bytes = std::fs::metadata(&tmp_out).map(|m| m.len()).unwrap_or(0);
                        if size_bytes < MIN_VALID_ROOTFS_BYTES {
                            let _ = std::fs::remove_file(&tmp_out);
                            continue;
                        }
                        std::fs::rename(&tmp_out, &out)?;
                        download_succeeded = true;
                        resolved = Some(out);
                        break;
                    }
                }
                if !download_succeeded {
                    logging::warn("run::guest", "Download failed for all attempted URLs", &[]);
                }
            }
        }
    }

    logging::info("run::guest", "Building kernel and boot image...", &[]);
    let bld = crate::cli::BuildAction::Full {
        arch: constants::defaults::build::ARCH,
        bootloader: crate::cli::Bootloader::Limine,
        format: crate::cli::ImageFormat::Iso,
        features: aethercore_common::KernelFeatures::VFS | aethercore_common::KernelFeatures::DRIVERS | aethercore_common::KernelFeatures::LOGGING,
        release: false,
        rootfs: resolved,
    };

    crate::commands::infra::build::execute(&bld).context("Failed building guest boot image")?;

    if attach {
        logging::info("run::guest", "📦 Attaching external rootfs as virtio disk", &[]);
    }

    logging::info("run::guest", "🚀 Launching QEMU with kernel and rootfs...", &[]);
    crate::commands::ops::qemu::interactive().context("QEMU interactive launch failed")?;
    Ok(())
}
