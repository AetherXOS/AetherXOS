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
        println!(
            "[userspace-seed] ⚠️ APT binary seed preparation failed (non-critical): {}",
            e
        );
    }

    // Prepare Flutter engine seed for desktop app support
    if let Err(e) = flutter_engine_seed::prepare_flutter_seed(initramfs_root) {
        println!(
            "[userspace-seed] ⚠️ Flutter engine seed preparation failed (non-critical): {}",
            e
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
    match selection.profile.as_str() {
        "debian" | "pardus" => {
            mirrors.push("https://deb.debian.org/debian".to_string());
            mirrors.push("http://ftp.debian.org/debian".to_string());
        }
        "arch" => {
            mirrors.push("https://geo.mirror.pkgbuild.com".to_string());
            mirrors.push("https://mirror.rackspace.com/archlinux".to_string());
        }
        _ => {
            mirrors.push("https://deb.debian.org/debian".to_string());
        }
    }
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
    let script = format!(
        r#"#!/bin/sh
set -eu

PKG_FILE="/etc/aethercore/apt-preload-packages.txt"
APP_FILE="/etc/aethercore/selected-app-targets.txt"
MIRROR_FILE="/etc/aethercore/mirror-failover.list"
ARTIFACT_FILE="/etc/aethercore/download-artifacts.list"
SMOKE_FILE="/etc/aethercore/smoke-commands.list"
HOOK_FILE="/etc/aethercore/postinstall-hooks.list"
PIN_FILE="/etc/aethercore/package-pins.list"
CHECKSUM_FILE="/etc/aethercore/checksum-policy.conf"
METADATA_SIG_POLICY_FILE="/etc/aethercore/metadata-signature-policy.conf"
APT_KEYRING_LIST_FILE="/etc/aethercore/apt-trusted-keyrings.list"
PACMAN_KEYRING_DIR_FILE="/etc/aethercore/pacman-keyring-dir.path"
ARTIFACT_LEDGER_PATH_FILE="/etc/aethercore/artifact-ledger.path"
TX_PATH_FILE="/etc/aethercore/transaction-journal.path"
TX_STATE_PATH_FILE="/etc/aethercore/transaction-state.path"
EV_PATH_FILE="/etc/aethercore/event-log.path"
RESUME_PATH_FILE="/etc/aethercore/resume-marker.path"
ROLLBACK_PATH_FILE="/etc/aethercore/rollback-marker.path"
SMOKE_TIMEOUT_FILE="/etc/aethercore/smoke-timeout.conf"
MIRROR="{mirror}"
RETRY_MAX={retry_max}
RETRY_BACKOFF={retry_backoff}
INSTALL_TIMEOUT={install_timeout}
ABI_CHECK_BIN="/usr/bin/aethercore-userspace-abi-check"

read_first_line() {{
    if [ -f "$1" ]; then
        head -n 1 "$1"
    fi
}}

TX_LOG="$(read_first_line "$TX_PATH_FILE")"
TX_STATE="$(read_first_line "$TX_STATE_PATH_FILE")"
ARTIFACT_LEDGER="$(read_first_line "$ARTIFACT_LEDGER_PATH_FILE")"
EV_LOG="$(read_first_line "$EV_PATH_FILE")"
RESUME_MARKER="$(read_first_line "$RESUME_PATH_FILE")"
ROLLBACK_MARKER="$(read_first_line "$ROLLBACK_PATH_FILE")"
PREV_RESUME_STATE=""
PREV_ROLLBACK_STATE=""
SMOKE_TIMEOUT="$(grep '^SMOKE_TIMEOUT_SECONDS=' "$SMOKE_TIMEOUT_FILE" 2>/dev/null | head -n 1 | cut -d'=' -f2)"
if [ -z "$SMOKE_TIMEOUT" ]; then
    SMOKE_TIMEOUT=60
fi

if [ -n "$TX_LOG" ]; then
    mkdir -p "$(dirname "$TX_LOG")" || true
fi
if [ -n "$TX_STATE" ]; then
    mkdir -p "$(dirname "$TX_STATE")" || true
fi
if [ -n "$ARTIFACT_LEDGER" ]; then
    mkdir -p "$(dirname "$ARTIFACT_LEDGER")" || true
    : > "$ARTIFACT_LEDGER" || true
fi
if [ -n "$EV_LOG" ]; then
    mkdir -p "$(dirname "$EV_LOG")" || true
fi
if [ -n "$RESUME_MARKER" ] && [ -f "$RESUME_MARKER" ]; then
    PREV_RESUME_STATE="$(head -n 1 "$RESUME_MARKER" || true)"
fi
if [ -n "$ROLLBACK_MARKER" ] && [ -f "$ROLLBACK_MARKER" ]; then
    PREV_ROLLBACK_STATE="$(head -n 1 "$ROLLBACK_MARKER" || true)"
fi
if [ -n "$RESUME_MARKER" ]; then
    mkdir -p "$(dirname "$RESUME_MARKER")" || true
    echo "seed-start" > "$RESUME_MARKER" || true
fi
if [ -n "$ROLLBACK_MARKER" ]; then
    mkdir -p "$(dirname "$ROLLBACK_MARKER")" || true
    echo "seed-start" > "$ROLLBACK_MARKER" || true
fi

log_event() {{
    if [ -n "$EV_LOG" ]; then
        printf "%s %s\n" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$1" >> "$EV_LOG" || true
    fi
}}

log_tx() {{
    if [ -n "$TX_LOG" ]; then
        printf "%s\n" "$1" >> "$TX_LOG" || true
    fi
}}

set_tx_state() {{
    stage="$1"
    if [ -n "$TX_STATE" ]; then
        printf "%s %s\n" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$stage" > "$TX_STATE" || true
    fi
}}

append_artifact_ledger() {{
    art_path="$1"
    if [ -n "$ARTIFACT_LEDGER" ]; then
        printf "%s\n" "$art_path" >> "$ARTIFACT_LEDGER" || true
    fi
}}

cleanup_artifacts_from_ledger() {{
    if [ -z "$ARTIFACT_LEDGER" ] || [ ! -f "$ARTIFACT_LEDGER" ]; then
        return 0
    fi

    while IFS= read -r item; do
        [ -z "$item" ] && continue
        case "$item" in
            /var/cache/aethercore/*|/opt/aethercore/artifacts/*)
                rm -f "$item" || true
                log_tx "ROLLBACK_CLEANUP_OK path=$item"
                ;;
            *)
                log_tx "ROLLBACK_CLEANUP_SKIP path=$item"
                ;;
        esac
    done < "$ARTIFACT_LEDGER"
}}

replay_previous_state() {{
    if [ -z "$PREV_RESUME_STATE" ] && [ -z "$PREV_ROLLBACK_STATE" ]; then
        return 0
    fi

    log_tx "REPLAY previous resume=$PREV_RESUME_STATE rollback=$PREV_ROLLBACK_STATE"
    log_event "installer-seed-replay resume=$PREV_RESUME_STATE rollback=$PREV_ROLLBACK_STATE"

    case "$PREV_ROLLBACK_STATE" in
        rollback-required:*)
            cleanup_artifacts_from_ledger
            if [ -n "$ROLLBACK_MARKER" ]; then
                printf "replay-in-progress:%s\n" "${{PREV_ROLLBACK_STATE#rollback-required:}}" > "$ROLLBACK_MARKER" || true
            fi
            ;;
        seed-start)
            cleanup_artifacts_from_ledger
            if [ -n "$ROLLBACK_MARKER" ]; then
                echo "replay-in-progress:interrupted-previous-run" > "$ROLLBACK_MARKER" || true
            fi
            ;;
        *)
            ;;
    esac
}}

run_with_retry() {{
    attempt=1
    while [ "$attempt" -le "$RETRY_MAX" ]; do
        if "$@"; then
            return 0
        fi
        sleep "$RETRY_BACKOFF"
        attempt=$((attempt+1))
    done
    return 1
}}

checksum_required() {{
    if [ ! -f "$CHECKSUM_FILE" ]; then
        return 1
    fi
    grep -q '^CHECKSUM_REQUIRED=1$' "$CHECKSUM_FILE"
}}

metadata_signature_required() {{
    if [ ! -f "$METADATA_SIG_POLICY_FILE" ]; then
        return 1
    fi
    grep -q '^METADATA_SIGNATURE_REQUIRED=1$' "$METADATA_SIG_POLICY_FILE"
}}

metadata_signature_mode() {{
    if [ ! -f "$METADATA_SIG_POLICY_FILE" ]; then
        echo "presence"
        return 0
    fi
    mode="$(grep '^METADATA_SIGNATURE_MODE=' "$METADATA_SIG_POLICY_FILE" 2>/dev/null | head -n 1 | cut -d'=' -f2)"
    if [ -z "$mode" ]; then
        echo "presence"
    else
        echo "$mode"
    fi
}}

mark_failed_state() {{
    reason="$1"
    cleanup_artifacts_from_ledger
    if [ -n "$RESUME_MARKER" ]; then
        printf "failed:%s\n" "$reason" > "$RESUME_MARKER" || true
    fi
    if [ -n "$ROLLBACK_MARKER" ]; then
        printf "rollback-required:%s\n" "$reason" > "$ROLLBACK_MARKER" || true
    fi
    log_tx "ROLLBACK seed-install reason=$reason"
    set_tx_state "rollback:$reason"
    log_event "installer-seed-failed reason=$reason"
}}

fail_install() {{
    reason="$1"
    mark_failed_state "$reason"
    exit 1
}}

fetch_url() {{
    src="$1"
    dst="$2"
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$src" -o "$dst"
        return $?
    fi
    if command -v wget >/dev/null 2>&1; then
        wget -qO "$dst" "$src"
        return $?
    fi
    return 127
}}

verify_sha256() {{
    expected="$1"
    file="$2"
    if [ -z "$expected" ]; then
        if checksum_required; then
            return 1
        fi
        return 0
    fi
    if command -v sha256sum >/dev/null 2>&1; then
        actual="$(sha256sum "$file" | awk '{{print $1}}')"
        [ "$actual" = "$expected" ]
        return $?
    fi
    return 127
}}

download_artifact() {{
    art_id="$1"
    art_url="$2"
    art_sha="$3"
    art_dst="$4"
    tmp="${{art_dst}}.tmp"
    mkdir -p "$(dirname "$art_dst")" || true

    if echo "$art_url" | grep -q '__MIRROR__'; then
        if [ -f "$MIRROR_FILE" ]; then
            while IFS= read -r mirror_item; do
                [ -z "$mirror_item" ] && continue
                candidate_url="$(printf '%s' "$art_url" | sed "s|__MIRROR__|$mirror_item|g")"
                if run_with_retry fetch_url "$candidate_url" "$tmp" && verify_sha256 "$art_sha" "$tmp"; then
                    mv "$tmp" "$art_dst"
                    append_artifact_ledger "$art_dst"
                    log_tx "ARTIFACT_OK id=$art_id url=$candidate_url dst=$art_dst"
                    return 0
                fi
            done < "$MIRROR_FILE"
        fi
    else
        if run_with_retry fetch_url "$art_url" "$tmp" && verify_sha256 "$art_sha" "$tmp"; then
            mv "$tmp" "$art_dst"
            append_artifact_ledger "$art_dst"
            log_tx "ARTIFACT_OK id=$art_id url=$art_url dst=$art_dst"
            return 0
        fi
    fi

    rm -f "$tmp" || true
    log_tx "ARTIFACT_FAIL id=$art_id url=$art_url dst=$art_dst"
    return 1
}}

validate_repo_metadata() {{
    if ! metadata_signature_required; then
        return 0
    fi

    mode="$(metadata_signature_mode)"

    apt_verify_file_with_keyrings() {{
        signed="$1"
        content="${{2:-}}"
        if [ ! -f "$APT_KEYRING_LIST_FILE" ]; then
            return 1
        fi
        while IFS= read -r keyring; do
            [ -z "$keyring" ] && continue
            [ ! -f "$keyring" ] && continue
            if [ -n "$content" ]; then
                if gpgv --keyring "$keyring" "$signed" "$content" >/dev/null 2>&1; then
                    return 0
                fi
            else
                if gpgv --keyring "$keyring" "$signed" >/dev/null 2>&1; then
                    return 0
                fi
            fi
        done < "$APT_KEYRING_LIST_FILE"
        return 1
    }}

    apt_verify_release_metadata() {{
        command -v gpgv >/dev/null 2>&1 || return 1
        found=0
        failed=0

        for rel in /var/lib/apt/lists/*_InRelease; do
            [ -f "$rel" ] || continue
            found=1
            apt_verify_file_with_keyrings "$rel" || failed=1
        done

        for sig in /var/lib/apt/lists/*_Release.gpg; do
            [ -f "$sig" ] || continue
            rel="${{sig%_Release.gpg}}_Release"
            [ -f "$rel" ] || continue
            found=1
            apt_verify_file_with_keyrings "$sig" "$rel" || failed=1
        done

        [ "$found" -eq 1 ] && [ "$failed" -eq 0 ]
    }}

    pacman_verify_release_metadata() {{
        command -v pacman-key >/dev/null 2>&1 || return 1
        if [ ! -d /var/lib/pacman/sync ]; then
            return 1
        fi

        keyring_dir="$(read_first_line "$PACMAN_KEYRING_DIR_FILE")"
        [ -n "$keyring_dir" ] || return 1
        [ -d "$keyring_dir" ] || return 1

        found=0
        failed=0
        for sig in /var/lib/pacman/sync/*.db.sig; do
            [ -f "$sig" ] || continue
            db="${{sig%.sig}}"
            [ -f "$db" ] || continue
            found=1
            pacman-key --gpgdir "$keyring_dir" --verify "$sig" "$db" >/dev/null 2>&1 || failed=1
        done
        [ "$found" -eq 1 ] && [ "$failed" -eq 0 ]
    }}

    if [ "$mode" = "presence" ]; then
        if command -v apt-get >/dev/null 2>&1; then
            if [ ! -d /var/lib/apt/lists ]; then
                return 1
            fi
            count="$(find /var/lib/apt/lists -maxdepth 1 \( -name '*_InRelease' -o -name '*_Release.gpg' \) | wc -l | tr -d ' ')"
            [ "${{count:-0}}" -gt 0 ]
            return $?
        fi

        if command -v pacman >/dev/null 2>&1; then
            if [ ! -d /var/lib/pacman/sync ]; then
                return 1
            fi
            count="$(find /var/lib/pacman/sync -maxdepth 1 -name '*.db.sig' | wc -l | tr -d ' ')"
            [ "${{count:-0}}" -gt 0 ]
            return $?
        fi

        return 1
    fi

    if command -v apt-get >/dev/null 2>&1; then
        apt_verify_release_metadata
        return $?
    fi

    if command -v pacman >/dev/null 2>&1; then
        pacman_verify_release_metadata
        return $?
    fi

    return 1
}}

if [ -f "$PKG_FILE" ]; then
  echo "[aethercore-apt-seed] package targets:"
  cat "$PKG_FILE"
fi

if [ -f "$APP_FILE" ]; then
    echo "[aethercore-apt-seed] app targets:"
    cat "$APP_FILE"
fi

if [ -f "$SMOKE_FILE" ] && [ -s "$SMOKE_FILE" ]; then
    echo "[aethercore-apt-seed] smoke commands:"
    cat "$SMOKE_FILE"
fi

if [ -f "$ARTIFACT_FILE" ] && [ -s "$ARTIFACT_FILE" ]; then
    echo "[aethercore-apt-seed] download artifacts:"
    cat "$ARTIFACT_FILE"
    while IFS='|' read -r art_id art_url art_sha art_dst; do
        [ -z "$art_id" ] && continue
        [ -z "$art_url" ] && continue
        [ -z "$art_dst" ] && continue
        if ! download_artifact "$art_id" "$art_url" "$art_sha" "$art_dst"; then
            log_event "artifact-download-failed id=$art_id"
            if checksum_required; then
                fail_install "artifact-download-failed:$art_id"
            fi
        fi
    done < "$ARTIFACT_FILE"
fi

echo "[aethercore-apt-seed] bundle descriptors available under /usr/share/aethercore/userspace_apps"
if [ -x "$ABI_CHECK_BIN" ]; then
    echo "[aethercore-apt-seed] running userspace ABI preflight"
    "$ABI_CHECK_BIN" || fail_install "userspace-abi-preflight-failed"
fi
replay_previous_state
log_event "installer-seed-start"
log_tx "BEGIN seed-install profile={profile} apps={apps}"
set_tx_state "begin"
log_tx "STAGE artifact-fetch"
set_tx_state "stage:artifact-fetch"

if command -v apt-get >/dev/null 2>&1; then
  export DEBIAN_FRONTEND=noninteractive
    if [ -f "$MIRROR_FILE" ] && [ -f /etc/apt/sources.list ]; then
        first_mirror="$(head -n 1 "$MIRROR_FILE" || true)"
        if [ -n "$first_mirror" ]; then
            sed -i "s|http://[^ ]*debian.org/debian|$first_mirror|g" /etc/apt/sources.list || true
        fi
    elif [ -n "$MIRROR" ] && [ -f /etc/apt/sources.list ]; then
        sed -i "s|http://[^ ]*debian.org/debian|$MIRROR|g" /etc/apt/sources.list || true
    fi
    run_with_retry apt-get update || fail_install "apt-update-failed"
    validate_repo_metadata || fail_install "repo-metadata-signature-check-failed"
    if [ -f "$PIN_FILE" ] && [ -s "$PIN_FILE" ]; then
        while IFS= read -r pin_line; do
            [ -z "$pin_line" ] && continue
            apt-mark hold "$pin_line" || true
        done < "$PIN_FILE"
    fi
    log_tx "STAGE package-install"
    set_tx_state "stage:package-install"
    timeout "$INSTALL_TIMEOUT" xargs -r apt-get install -y < "$PKG_FILE" || fail_install "apt-install-failed"

    # Production package-manager sanity probes
    if ! command -v xz >/dev/null 2>&1; then
        run_with_retry apt-get install -y xz-utils || fail_install "apt-xz-utils-probe-failed"
    fi
    if [ -f "$APP_FILE" ] && grep -Eq '^flutter$' "$APP_FILE"; then
        run_with_retry apt-get install -y flutter || fail_install "apt-flutter-install-failed"
        command -v flutter >/dev/null 2>&1 || fail_install "flutter-runtime-not-found-after-install"
    fi
elif command -v pacman >/dev/null 2>&1; then
    if [ -f "$MIRROR_FILE" ] && [ -f /etc/pacman.d/mirrorlist ]; then
        first_mirror="$(head -n 1 "$MIRROR_FILE" || true)"
        if [ -n "$first_mirror" ]; then
            printf "Server = %s/$repo/os/$arch\n" "$first_mirror" > /etc/pacman.d/mirrorlist || true
        fi
    elif [ -n "$MIRROR" ] && [ -f /etc/pacman.d/mirrorlist ]; then
        printf "Server = %s/$repo/os/$arch\n" "$MIRROR" > /etc/pacman.d/mirrorlist || true
    fi
    run_with_retry pacman -Syy --noconfirm || fail_install "pacman-sync-failed"
    validate_repo_metadata || fail_install "repo-metadata-signature-check-failed"
    log_tx "STAGE package-install"
    set_tx_state "stage:package-install"
    timeout "$INSTALL_TIMEOUT" xargs -r pacman -S --needed --noconfirm < "$PKG_FILE" || fail_install "pacman-install-failed"
else
    fail_install "no-supported-package-manager"
fi

if [ -f "$HOOK_FILE" ] && [ -s "$HOOK_FILE" ]; then
    log_tx "STAGE postinstall-hooks"
    set_tx_state "stage:postinstall-hooks"
    while IFS= read -r hook; do
        [ -z "$hook" ] && continue
        sh -c "$hook" || true
    done < "$HOOK_FILE"
fi

if [ -f "$SMOKE_FILE" ] && [ -s "$SMOKE_FILE" ]; then
    log_tx "STAGE app-smoke"
    set_tx_state "stage:app-smoke"
    while IFS= read -r smoke_cmd; do
        [ -z "$smoke_cmd" ] && continue
        if timeout "$SMOKE_TIMEOUT" sh -c "$smoke_cmd"; then
            log_tx "SMOKE_OK cmd=$smoke_cmd"
        else
            log_tx "SMOKE_FAIL cmd=$smoke_cmd"
            fail_install "app-smoke-failed"
        fi
    done < "$SMOKE_FILE"
fi

if [ -n "$RESUME_MARKER" ]; then
    echo "seed-complete" > "$RESUME_MARKER" || true
fi
if [ -n "$ROLLBACK_MARKER" ]; then
    echo "seed-committed" > "$ROLLBACK_MARKER" || true
fi
log_tx "COMMIT seed-install"
set_tx_state "commit"
log_event "installer-seed-complete"
"#,
        mirror = mirror,
        retry_max = policy.retry_max_attempts,
        retry_backoff = policy.retry_backoff_seconds,
        install_timeout = policy.install_timeout_seconds,
        profile = selection.profile,
        apps = selection.selected_apps.join(",")
    );
    fs::write(bin_dir.join("aethercore-apt-seed"), script)?;

    Ok(())
}
