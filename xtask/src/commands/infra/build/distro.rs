use crate::constants;
use crate::utils::{context, logging, net, paths, registry, ui};
use aethercore_common::TargetArch;
use anyhow::{Result, bail};
use std::fs;
use std::str::FromStr;

/// Constructs a specialized ISO based on an existing Linux distribution,
/// injecting the AetherXOS kernel and bootloader configuration.
pub fn build_distro_iso(
    distro: Option<String>,
    version: Option<String>,
    variant: Option<String>,
    arch: Option<TargetArch>,
) -> Result<()> {
    // 0. Pre-flight checks are handled globally in main.rs via run_audit()
    // 1. Load Registry
    let reg = registry::DistroRegistry::load_default()?;

    // 2. Interactive Flow if not fully specified
    let non_interactive =
        distro.is_some() && version.is_some() && variant.is_some() && arch.is_some();
    let (selected_distro, selected_ver, selected_var, selected_arch) =
        resolve_distro_interactively(&reg, distro, version, variant, arch)?;

    logging::info(
        "distro-iso",
        "starting distro-based ISO construction",
        &[
            ("distro", &selected_distro),
            ("version", &selected_ver),
            ("variant", &selected_var),
            ("arch", selected_arch.as_str()),
        ],
    );

    // 3. Compile our kernel first
    super::kernel::build_kernel(
        selected_arch,
        false,
        aethercore_common::KernelFeatures::VFS | aethercore_common::KernelFeatures::DRIVERS,
    )?;

    let arch_norm = selected_arch.as_str().replace('-', "_");
    let image = find_image(
        &reg,
        &selected_distro,
        &selected_ver,
        &selected_var,
        &arch_norm,
    )
    .ok_or_else(|| anyhow::anyhow!("Image not found in registry"))?;

    let url = image.url();
    logging::info("distro-iso", "using base image URL", &[("url", url)]);

    // 4. Prepare staging directory
    let stage_dir = constants::paths::boot_image_stage_boot();
    let iso_root = stage_dir.parent().unwrap();
    paths::ensure_dir(&stage_dir)?;

    // 5. Download and Extract (Caching & Hashing)
    let cache_dir = context::out_dir().join("guest_cache");
    paths::ensure_dir(&cache_dir)?;

    let filename = url.split('/').last().unwrap_or("rootfs.tar.xz");
    let target_archive = cache_dir.join(filename);

    let mut download_needed = !target_archive.exists();
    let hashes = image.hashes();
    let expected_size = image.size_bytes();

    let max_download_attempts = crate::utils::config::max_download_attempts();
    let mut download_attempts = 0usize;
    loop {
        if !download_needed {
            // If caller provided all selection params, operate non-interactively by default.
            if non_interactive {
                // automated checks: size/hash mismatches trigger redownload automatically
                if let Some(size) = expected_size {
                    if let Ok(meta) = fs::metadata(&target_archive) {
                        if meta.len() != size {
                            logging::warn(
                                "distro-iso",
                                "local image size mismatch detected; scheduling redownload",
                                &[
                                    ("expected", &format!("{} bytes", size)),
                                    ("actual", &format!("{} bytes", meta.len())),
                                ],
                            );
                            download_needed = true;
                        }
                    }
                }

                if !download_needed && !hashes.is_empty() {
                    logging::info("distro-iso", "verifying cached image hashes", &[]);
                    let algos: Vec<_> = hashes.keys().copied().collect();
                    let actual_hashes = crate::utils::calculate_hashes(&target_archive, &algos)?;
                    for (algo, expected) in &hashes {
                        if let Some(actual) = actual_hashes.get(algo) {
                            if actual != expected {
                                logging::warn(
                                    "distro-iso",
                                    "hash mismatch detected; scheduling redownload",
                                    &[
                                        ("algo", &format!("{:?}", algo)),
                                        ("expected", expected),
                                        ("actual", actual),
                                    ],
                                );
                                download_needed = true;
                                break;
                            }
                        }
                    }
                }
            } else {
                // 1. Interactive Choice (Only if we didn't just decide to redownload)
                if !ui::confirm(
                    &format!("Cached image found: {}. Use existing?", filename),
                    true,
                )? {
                    logging::info("distro-iso", "user requested redownload", &[]);
                    download_needed = true;
                }

                // 2. Automated Checks (if user wanted to use it)
                if !download_needed {
                    // Check Size
                    if let Some(size) = expected_size {
                        if let Ok(meta) = fs::metadata(&target_archive) {
                            if meta.len() != size {
                                logging::warn(
                                    "distro-iso",
                                    "local image size mismatch detected",
                                    &[
                                        ("expected", &format!("{} bytes", size)),
                                        ("actual", &format!("{} bytes", meta.len())),
                                    ],
                                );

                                let options = ["Use current (risky)", "Redownload", "Abort"];
                                let choice = ui::select("How do you want to proceed?", &options)?;
                                match *choice {
                                    "Redownload" => download_needed = true,
                                    "Abort" => {
                                        bail!("Operation aborted by user due to size mismatch")
                                    }
                                    _ => logging::info(
                                        "distro-iso",
                                        "proceeding with existing image despite size mismatch",
                                        &[],
                                    ),
                                }
                            }
                        }
                    }

                    // Check Hashes (only if not already redownloading)
                    if !download_needed && !hashes.is_empty() {
                        logging::info("distro-iso", "verifying cached image hashes", &[]);
                        let algos: Vec<_> = hashes.keys().copied().collect();
                        let actual_hashes =
                            crate::utils::calculate_hashes(&target_archive, &algos)?;

                        for (algo, expected) in &hashes {
                            if let Some(actual) = actual_hashes.get(algo) {
                                if actual != expected {
                                    logging::warn(
                                        "distro-iso",
                                        "hash mismatch detected",
                                        &[
                                            ("algo", &format!("{:?}", algo)),
                                            ("expected", expected),
                                            ("actual", actual),
                                        ],
                                    );

                                    let options = ["Use current (risky)", "Redownload", "Abort"];
                                    let choice = ui::select(
                                        &format!("Hash mismatch ({:?}). Proceed?", algo),
                                        &options,
                                    )?;
                                    match *choice {
                                        "Redownload" => {
                                            download_needed = true;
                                            break;
                                        }
                                        "Abort" => {
                                            bail!("Operation aborted by user due to hash mismatch")
                                        }
                                        _ => logging::info(
                                            "distro-iso",
                                            "proceeding with existing image despite hash mismatch",
                                            &[],
                                        ),
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if download_needed {
            download_attempts = download_attempts.saturating_add(1);
            if target_archive.exists() {
                fs::remove_file(&target_archive).ok();
            }
            logging::info("distro-iso", "downloading base distro rootfs", &[]);
            let download_result = net::download_with_configured_retries(url, &target_archive);

            if let Err(e) = download_result {
                logging::warn("distro-iso", "download failed", &[("err", &e.to_string())]);
                if non_interactive {
                    if download_attempts >= max_download_attempts {
                        bail!(
                            "Download failed after {} attempts: {}",
                            download_attempts,
                            e
                        );
                    } else {
                        logging::info(
                            "distro-iso",
                            "transient download problem; retrying",
                            &[("attempt", &format!("{}", download_attempts))],
                        );
                        download_needed = true;
                        continue;
                    }
                } else {
                    // Interactive: ask user whether to retry
                    if ui::confirm(&format!("Download failed: {}. Retry?", e), true)? {
                        download_needed = true;
                        continue;
                    } else {
                        bail!("Download aborted by user: {}", e);
                    }
                }
            }

            // Post-download verification
            if !hashes.is_empty() {
                let algos: Vec<_> = hashes.keys().copied().collect();
                let actual_hashes = crate::utils::calculate_hashes(&target_archive, &algos)?;
                let mut mismatch = false;
                for (algo, expected) in &hashes {
                    if let Some(actual) = actual_hashes.get(algo) {
                        if actual != expected {
                            logging::warn(
                                "distro-iso",
                                "post-download hash mismatch",
                                &[
                                    ("algo", &format!("{:?}", algo)),
                                    ("expected", expected),
                                    ("actual", actual),
                                ],
                            );
                            mismatch = true;
                        }
                    }
                }

                if mismatch {
                    if non_interactive {
                        if download_attempts >= max_download_attempts {
                            bail!(
                                "Hash mismatch persisted after {} attempts",
                                download_attempts
                            );
                        }
                        logging::warn("distro-iso", "hash mismatch; scheduling redownload", &[]);
                        download_needed = true;
                        continue;
                    } else {
                        if !ui::confirm(
                            "Hash verification failed after download. Use anyway?",
                            false,
                        )? {
                            bail!("Hash verification failed after download");
                        }
                    }
                }
            }
        }

        logging::info("distro-iso", "extracting distro rootfs into ISO root", &[]);
        match super::rootfs::extract_rootfs_archive(&target_archive, iso_root) {
            Ok(_) => break, // Success!
            Err(e) if e.to_string().contains("REDOWNLOAD_REQUESTED") => {
                logging::warn(
                    "distro-iso",
                    "extraction requested redownload. retrying...",
                    &[],
                );
                download_needed = true;
                continue;
            }
            Err(e) => return Err(e),
        }
    }

    // 6. Copy our kernel
    let target_triple = selected_arch.to_bare_metal_triple();
    let kernel_src = paths::resolve(&format!("target/{}/debug/aethercore", target_triple));

    logging::info(
        "distro-iso",
        "validating kernel ELF integrity",
        &[("path", &kernel_src.to_string_lossy())],
    );
    crate::utils::elf::validate_elf(&kernel_src)?;

    fs::copy(&kernel_src, stage_dir.join("aethercore.elf"))?;

    // 7. Copy Limine binaries — with self-healing auto-fetch
    let limine_bin_dir = paths::resolve("artifacts/limine/bin");
    paths::ensure_dir(&iso_root.join("EFI/BOOT"))?;

    ensure_limine_binaries(&limine_bin_dir)?;

    let bios_sys = limine_bin_dir.join("limine-bios.sys");
    // Copy to /boot (stage dir)
    fs::copy(
        limine_bin_dir.join("limine-bios-cd.bin"),
        stage_dir.join("limine-bios-cd.bin"),
    )?;
    fs::copy(&bios_sys, stage_dir.join("limine-bios.sys"))?;
    fs::copy(
        limine_bin_dir.join("limine-uefi-cd.bin"),
        stage_dir.join("limine-uefi-cd.bin"),
    )?;
    fs::copy(
        limine_bin_dir.join("BOOTX64.EFI"),
        iso_root.join("EFI/BOOT/BOOTX64.EFI"),
    )?;

    // Also mirror limine-bios.sys to ISO root for BIOS boot compatibility
    fs::copy(&bios_sys, iso_root.join("limine-bios.sys"))?;

    // 8. Build Initramfs (Interactive)
    let mut final_initrd = None;
    if ui::confirm("Include AetherXOS Initramfs (initrd)?", true)? {
        let initramfs_src = paths::resolve("artifacts/initramfs_root");
        let initramfs_dst = stage_dir.join("initramfs.cpio.gz");
        let initrd_dst = stage_dir.join("initrd.cpio.gz");
        if initramfs_src.exists() {
            crate::commands::infra::initramfs::build(&initramfs_src, &initramfs_dst)?;
            fs::copy(&initramfs_dst, &initrd_dst)?;
            final_initrd = Some("initrd.cpio.gz");
        } else if initramfs_dst.exists() {
            logging::info(
                "distro-iso",
                "initramfs source missing; reusing staged initrd",
                &[("path", &initramfs_dst.to_string_lossy())],
            );
            if !initrd_dst.exists() {
                fs::copy(&initramfs_dst, &initrd_dst)?;
            }
            final_initrd = Some("initrd.cpio.gz");
        } else {
            logging::warn(
                "distro-iso",
                "initramfs source missing, skipping initrd",
                &[],
            );
        }
    } else {
        logging::info("distro-iso", "skipping initramfs as requested", &[]);
    }

    // 9. Finalize
    crate::commands::infra::limine::generate_configs(
        &stage_dir,
        "aethercore.elf",
        final_initrd,
        constants::defaults::run::KERNEL_APPEND,
    )?;

    // Copy limine.conf to root as well for backup
    fs::copy(stage_dir.join("limine.conf"), iso_root.join("limine.conf"))?;

    let out_iso = context::out_dir().join(format!(
        "aetherxos-{}-{}.iso",
        selected_distro, selected_var
    ));
    crate::commands::infra::iso::finalize_iso_from_root(iso_root, &out_iso)?;

    logging::ready(
        "distro-iso",
        "distro-based ISO completed",
        out_iso.to_string_lossy(),
    );
    Ok(())
}

fn resolve_distro_interactively(
    reg: &registry::DistroRegistry,
    distro: Option<String>,
    version: Option<String>,
    variant: Option<String>,
    arch: Option<TargetArch>,
) -> Result<(String, String, String, TargetArch)> {
    // 1. Distro
    let selected_distro = match distro {
        Some(d) if reg.distros.contains_key(&d) => d,
        _ => {
            let mut keys: Vec<_> = reg.distros.keys().cloned().collect();
            keys.sort();
            if keys.len() == 1 {
                keys[0].clone()
            } else {
                ui::select("Select Target Distribution", &keys)?.to_string()
            }
        }
    };

    let dval = &reg.distros[&selected_distro];

    // 2. Version
    let selected_ver = match version {
        Some(v) if dval.versions.contains_key(&v) => v,
        _ => {
            let mut keys: Vec<_> = dval.versions.keys().cloned().collect();
            keys.sort_by(|a, b| b.cmp(a)); // Descending for versions
            if keys.len() == 1 {
                keys[0].clone()
            } else {
                ui::select(&format!("Select {} Version", selected_distro), &keys)?.to_string()
            }
        }
    };

    let vval = &dval.versions[&selected_ver];

    // 3. Variant — accept explicit variant strings with fuzzy matching to avoid interactive prompts
    let selected_var = match variant {
        Some(v) if vval.variants.contains_key(&v) => v,
        Some(v) => {
            // Try fuzzy matching: exact-contains, starts_with, ends_with, replace underscores/hyphens
            let keys: Vec<String> = vval.variants.keys().cloned().collect();
            let v_norm = v.to_ascii_lowercase();
            // direct contains
            if let Some(k) = keys.iter().find(|k| k.to_ascii_lowercase() == v_norm) {
                k.clone()
            } else if let Some(k) = keys
                .iter()
                .find(|k| k.to_ascii_lowercase().contains(&v_norm))
            {
                k.clone()
            } else if let Some(k) = keys
                .iter()
                .find(|k| v_norm.contains(&k.to_ascii_lowercase()))
            {
                k.clone()
            } else {
                // try replacing -/_ variants
                let v_alt = v_norm.replace('-', "_");
                if let Some(k) = keys.iter().find(|k| k.to_ascii_lowercase() == v_alt) {
                    k.clone()
                } else {
                    // Fallback to interactive selection if fuzzy-match fails
                    let mut keys_sorted: Vec<_> = keys.clone();
                    keys_sorted.sort();
                    if keys_sorted.len() == 1 {
                        keys_sorted[0].clone()
                    } else {
                        ui::select(&format!("Select {} Variant", selected_distro), &keys_sorted)?
                            .to_string()
                    }
                }
            }
        }
        None => {
            let mut keys: Vec<_> = vval.variants.keys().cloned().collect();
            keys.sort();
            if keys.len() == 1 {
                keys[0].clone()
            } else {
                ui::select(&format!("Select {} Variant", selected_distro), &keys)?.to_string()
            }
        }
    };

    let var_map = &vval.variants[&selected_var];

    // 4. Arch
    let selected_arch = match arch {
        Some(a) if var_map.contains_key(a.as_str()) => a,
        _ => {
            let keys: Vec<_> = var_map.keys().cloned().collect();
            if keys.len() == 1 {
                TargetArch::from_str(&keys[0])
                    .map_err(|_| anyhow::anyhow!("Invalid arch in registry: {}", keys[0]))?
            } else {
                let choice =
                    ui::select(&format!("Select {} Architecture", selected_distro), &keys)?;
                TargetArch::from_str(&choice)
                    .map_err(|_| anyhow::anyhow!("Invalid arch selected: {}", choice))?
            }
        }
    };

    Ok((selected_distro, selected_ver, selected_var, selected_arch))
}

fn find_image(
    reg: &registry::DistroRegistry,
    distro: &str,
    version: &str,
    variant: &str,
    arch: &str,
) -> Option<registry::DistroImage> {
    reg.distros
        .get(distro)?
        .versions
        .get(version)?
        .variants
        .get(variant)?
        .get(arch)?
        .first()
        .cloned()
}

/// Validates that all required Limine bootloader binaries are present and non-empty.
/// If any are missing or corrupt, auto-fetches them with user confirmation.
fn ensure_limine_binaries(limine_bin_dir: &std::path::Path) -> Result<()> {
    const REQUIRED: &[&str] = &[
        "limine-bios.sys",
        "limine-bios-cd.bin",
        "limine-uefi-cd.bin",
        "BOOTX64.EFI",
    ];

    let mut needs_fetch = false;

    for name in REQUIRED {
        let path = limine_bin_dir.join(name);
        if !path.exists() {
            logging::warn("limine", "binary missing", &[("file", name)]);
            needs_fetch = true;
        } else if fs::metadata(&path)?.len() == 0 {
            logging::warn("limine", "binary is corrupt (0 bytes)", &[("file", name)]);
            fs::remove_file(&path)?; // Remove the broken file so download can proceed
            needs_fetch = true;
        }
    }

    if needs_fetch {
        logging::info(
            "limine",
            "auto-fetching missing or corrupt Limine binaries",
            &[],
        );
        crate::commands::infra::setup::download::fetch_limine_binaries().map_err(|e| {
            anyhow::anyhow!(
                "Limine auto-fetch failed: {}\n\
                 Tip: Check your internet connection, then run 'cargo xtask setup limine'.",
                e
            )
        })?;

        // Re-verify after fetch
        for name in REQUIRED {
            let path = limine_bin_dir.join(name);
            if !path.exists() || fs::metadata(&path)?.len() == 0 {
                anyhow::bail!(
                    "Limine binary '{}' is still invalid after fetch. \
                     Please manually download Limine v7.x from https://github.com/limine-bootloader/limine",
                    name
                );
            }
        }

        logging::info("limine", "all binaries verified after auto-fetch", &[]);
    } else {
        logging::info("limine", "all binaries present and valid", &[]);
    }

    Ok(())
}
