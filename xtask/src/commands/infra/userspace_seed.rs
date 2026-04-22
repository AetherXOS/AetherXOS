use crate::utils::logging;
use anyhow::{Context, Result};
use serde::Deserialize;
use serde::Serialize;
use std::fs;
use std::path::Path;

use crate::commands::infra::apt_binary_seed;
use crate::commands::infra::flutter_engine_seed;
use crate::commands::infra::installer_policy::InstallerPolicy;
use crate::commands::infra::installer_profile::{InstallerSelection, PackageManager};

#[derive(Debug, Deserialize)]
struct BundleDescriptor {
    bundle_id: String,
    version: String,
}

#[derive(Debug, Serialize)]
struct SeedManifest<'a> {
    profile: &'a str,
    package_manager: &'a str,
    mirror: Option<&'a str>,
    selected_apps: &'a [String],
    package_count: usize,
    download_artifact_count: usize,
    smoke_command_count: usize,
}

pub fn inject_seed(
    initramfs_root: &Path,
    selection: &InstallerSelection,
    policy: &InstallerPolicy,
    bundle_dir: &Path,
) -> Result<()> {
    let etc_aethercore = initramfs_root.join("etc/aethercore");
    let bundle_dst = initramfs_root.join("usr/share/aethercore/userspace_apps");
    let bin_dir = initramfs_root.join("usr/bin");
    let lib_aethercore = initramfs_root.join("usr/lib/aethercore");

    fs::create_dir_all(&etc_aethercore)?;
    fs::create_dir_all(&bundle_dst)?;
    fs::create_dir_all(&bin_dir)?;
    fs::create_dir_all(&lib_aethercore)?;

    // Prepare APT binary seed for live package installation
    if let Err(e) = apt_binary_seed::prepare_apt_seed(initramfs_root) {
        logging::warn(
            "userspace-seed",
            "APT binary seed preparation failed (non-critical)",
            &[("error", &e.to_string())],
        );
    }

    // Prepare Flutter engine seed for desktop app support
    if let Err(e) = flutter_engine_seed::prepare_flutter_seed(initramfs_root) {
        logging::warn(
            "userspace-seed",
            "Flutter engine seed preparation failed (non-critical)",
            &[("error", &e.to_string())],
        );
    }
    let package_list = selection.packages.join("\n");
    fs::write(
        etc_aethercore.join("apt-preload-packages.txt"),
        format!("{}\n", package_list),
    )?;

    let seed_manifest = SeedManifest {
        profile: &selection.profile,
        package_manager: match selection.package_manager {
            PackageManager::Apt => "apt",
            PackageManager::Pacman => "pacman",
        },
        mirror: selection.mirror.as_deref(),
        selected_apps: &selection.selected_apps,
        package_count: selection.packages.len(),
        download_artifact_count: selection.download_artifacts.len(),
        smoke_command_count: selection.smoke_commands.len(),
    };
    fs::write(
        etc_aethercore.join("installer-selection.json"),
        serde_json::to_string_pretty(&seed_manifest)?,
    )?;
    fs::write(
        etc_aethercore.join("selected-app-targets.txt"),
        format!("{}\n", selection.selected_apps.join("\n")),
    )?;
    fs::write(
        etc_aethercore.join("installer-policy.json"),
        serde_json::to_string_pretty(policy)?,
    )?;

    let mut mirrors = Vec::new();
    if let Some(primary) = selection.mirror.as_deref() {
        mirrors.push(primary.to_string());
    }
    mirrors.extend(selection.mirror_fallbacks.iter().cloned());
    mirrors.dedup();
    fs::write(
        etc_aethercore.join("mirror-failover.list"),
        format!("{}\n", mirrors.join("\n")),
    )?;
    fs::write(
        etc_aethercore.join("checksum-policy.conf"),
        format!(
            "CHECKSUM_REQUIRED={}\n",
            if policy.checksum_required { "1" } else { "0" }
        ),
    )?;
    fs::write(
        etc_aethercore.join("metadata-signature-policy.conf"),
        format!(
            "METADATA_SIGNATURE_REQUIRED={}\nMETADATA_SIGNATURE_MODE={}\n",
            if policy.metadata_signature_required {
                "1"
            } else {
                "0"
            },
            policy.metadata_signature_mode
        ),
    )?;
    fs::write(
        etc_aethercore.join("apt-trusted-keyrings.list"),
        format!("{}\n", policy.apt_trusted_keyring_paths.join("\n")),
    )?;
    fs::write(
        etc_aethercore.join("pacman-keyring-dir.path"),
        format!("{}\n", policy.pacman_keyring_dir),
    )?;
    fs::write(
        etc_aethercore.join("artifact-ledger.path"),
        format!("{}\n", policy.artifact_ledger_path),
    )?;
    fs::write(
        etc_aethercore.join("postinstall-hooks.list"),
        format!("{}\n", policy.postinstall_hooks.join("\n")),
    )?;
    fs::write(
        etc_aethercore.join("package-pins.list"),
        format!("{}\n", policy.package_pins.join("\n")),
    )?;
    fs::write(
        etc_aethercore.join("installer-timeout.conf"),
        format!(
            "INSTALL_TIMEOUT_SECONDS={}\n",
            policy.install_timeout_seconds
        ),
    )?;
    fs::write(
        etc_aethercore.join("transaction-journal.path"),
        format!("{}\n", policy.transaction_log_path),
    )?;
    fs::write(
        etc_aethercore.join("transaction-state.path"),
        format!("{}\n", policy.transaction_state_path),
    )?;
    fs::write(
        etc_aethercore.join("event-log.path"),
        format!("{}\n", policy.event_log_path),
    )?;
    fs::write(
        etc_aethercore.join("resume-marker.path"),
        format!("{}\n", policy.resume_marker_path),
    )?;
    fs::write(
        etc_aethercore.join("rollback-marker.path"),
        format!("{}\n", policy.rollback_marker_path),
    )?;
    fs::write(
        etc_aethercore.join("smoke-timeout.conf"),
        format!("SMOKE_TIMEOUT_SECONDS={}\n", policy.smoke_timeout_seconds),
    )?;
    let download_artifact_lines = selection
        .download_artifacts
        .iter()
        .map(|artifact| {
            format!(
                "{}|{}|{}|{}",
                artifact.id,
                artifact.url,
                artifact.sha256.as_deref().unwrap_or(""),
                artifact.destination
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(
        etc_aethercore.join("download-artifacts.list"),
        format!("{}\n", download_artifact_lines),
    )?;
    fs::write(
        etc_aethercore.join("smoke-commands.list"),
        format!("{}\n", selection.smoke_commands.join("\n")),
    )?;

    let mut copied_bundles: Vec<(String, String)> = Vec::new();
    if bundle_dir.exists() {
        for entry in
            fs::read_dir(bundle_dir).context("failed to read userspace app bundle directory")?
        {
            let entry = entry?;
            let src = entry.path();
            if !src.is_file() {
                continue;
            }
            if src.extension().and_then(|v| v.to_str()) != Some("json") {
                continue;
            }

            let Some(name) = src.file_name() else {
                continue;
            };

            let raw = fs::read_to_string(&src)
                .with_context(|| format!("failed to read bundle descriptor: {}", src.display()))?;
            if let Ok(bundle) = serde_json::from_str::<BundleDescriptor>(&raw) {
                copied_bundles.push((bundle.bundle_id, bundle.version));
            }

            fs::copy(&src, bundle_dst.join(name)).with_context(|| {
                format!(
                    "failed to copy bundle descriptor into initramfs: {}",
                    src.display()
                )
            })?;
        }
    }

    copied_bundles.sort_by(|a, b| a.0.cmp(&b.0));
    let mut manifest = String::from("# AetherCore userspace bundle seed manifest\n");
    for (id, version) in copied_bundles {
        manifest.push_str(&format!("{}={}\n", id, version));
    }
    fs::write(etc_aethercore.join("userspace-bundles.manifest"), manifest)?;

        let apt_abi_contract = format!(
                "# AetherCore userspace ABI contract for production package-manager workloads\n\
abi.surface=aethercore-linux-compat\n\
abi.profile={}\n\
required.mounts=/proc,/sys,/dev/shm,/run,/tmp\n\
required.diskfs=/var/lib/aethercore\n\
required.tools=apt-get,dpkg,xz\n\
required.syscalls=openat2,statx,renameat2,faccessat2,clone3,epoll_pwait2\n\
flutter.requested={}\n",
                selection.profile,
                selection.selected_apps.iter().any(|app| app == "flutter")
        );
        fs::write(
                lib_aethercore.join("userspace-apt-abi-contract.txt"),
                apt_abi_contract,
        )?;

        let abi_check_script = r#"#!/bin/sh
set -eu

missing=0

check_path() {
    p="$1"
    if [ ! -e "$p" ]; then
        echo "[userspace-abi-check] missing path: $p"
        missing=1
    fi
}

check_mountpoint() {
    m="$1"
    if ! grep -q " $m " /proc/mounts 2>/dev/null; then
        echo "[userspace-abi-check] mount missing: $m"
        missing=1
    fi
}

check_path /proc/sys/aethercore/abi/platform
check_path /proc/sys/aethercore/abi/abi_version_major
check_path /proc/sys/aethercore/abi/abi_version_minor
check_path /proc/sys/aethercore/abi/abi_version_patch

check_mountpoint /proc
check_mountpoint /sys
check_mountpoint /tmp
check_mountpoint /run
check_mountpoint /dev/shm

if [ ! -d /var/lib/aethercore ]; then
    echo "[userspace-abi-check] persistent package state directory missing: /var/lib/aethercore"
    missing=1
fi

if ! command -v apt-get >/dev/null 2>&1 && ! command -v pacman >/dev/null 2>&1; then
    echo "[userspace-abi-check] no supported package manager in PATH"
    missing=1
fi

if [ "$missing" -ne 0 ]; then
    echo "[userspace-abi-check] FAIL"
    exit 1
fi

echo "[userspace-abi-check] OK"
exit 0
"#;
        fs::write(bin_dir.join("aethercore-userspace-abi-check"), abi_check_script)?;

    let mirror = selection.mirror.as_deref().unwrap_or("");
    let template = include_str!("userspace_seed_script.sh");
    let script = template
        .replace("{mirror}", mirror)
        .replace("{retry_max}", &policy.retry_max_attempts.to_string())
        .replace("{retry_backoff}", &policy.retry_backoff_seconds.to_string())
        .replace("{install_timeout}", &policy.install_timeout_seconds.to_string())
        .replace("{profile}", &selection.profile)
        .replace("{apps}", &selection.selected_apps.join(","));
    fs::write(bin_dir.join("aethercore-apt-seed"), script)?;

    Ok(())
}
