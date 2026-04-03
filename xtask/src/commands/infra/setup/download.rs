use crate::constants;
use crate::utils::paths;
use crate::utils::process;
use anyhow::{Context, Result, bail};

const LIMINE_ARTIFACTS: &[(&str, &str)] = &[
    (
        "limine-bios.sys",
        "https://raw.githubusercontent.com/limine-bootloader/limine/v7.0-branch-binary/limine-bios.sys",
    ),
    (
        "limine-bios-cd.bin",
        "https://raw.githubusercontent.com/limine-bootloader/limine/v7.0-branch-binary/limine-bios-cd.bin",
    ),
    (
        "limine-uefi-cd.bin",
        "https://raw.githubusercontent.com/limine-bootloader/limine/v7.0-branch-binary/limine-uefi-cd.bin",
    ),
    (
        "BOOTX64.EFI",
        "https://raw.githubusercontent.com/limine-bootloader/limine/v7.0-branch-binary/BOOTX64.EFI",
    ),
];

fn try_download_file(url: &str, destination: &std::path::Path) -> bool {
    let dest = destination.to_string_lossy().to_string();
    let powershell_script = format!(
        "$ProgressPreference='SilentlyContinue'; Invoke-WebRequest -Uri '{}' -OutFile '{}'",
        url,
        dest.replace('\'', "''")
    );

    process::run_first_success(&[
        ("curl", &["-fsSL", "-o", dest.as_str(), url]),
        ("wget", &["-qO", dest.as_str(), url]),
        (
            "powershell",
            &["-NoProfile", "-Command", powershell_script.as_str()],
        ),
    ])
}

/// Automates synchronization of Limine EFI/BIOS binaries from upstream sources.
/// Allows cross-platform construction of bootable ISOs without manual configuration.
pub(crate) fn fetch_limine_binaries() -> Result<()> {
    println!(
        "[setup::fetch] Connecting to upstream vendor registries for Limine payload distribution..."
    );

    let dest_dir = constants::paths::limine_bin_dir();
    paths::ensure_dir(&dest_dir)
        .context("Failed establishing directory boundaries for limiting vendored bins")?;

    for (filename, url) in LIMINE_ARTIFACTS {
        let dest_file = dest_dir.join(filename);
        println!(
            "[setup::fetch] -> Streaming object with fallback download backends: {}",
            filename
        );

        if !try_download_file(url, &dest_file) {
            bail!("Remote host denied binary download or connection dropped forcefully.");
        }
    }

    println!(
        "[setup::fetch] Synchronization sequence successful. OS wrapper mechanisms updated to latest stable protocol."
    );
    Ok(())
}
