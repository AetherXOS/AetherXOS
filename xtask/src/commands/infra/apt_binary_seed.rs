/// APT binary seeding and provisioning
/// Downloads pre-built apt-get binary and essential package manager tools
/// into the initramfs for live package installation capability.

use crate::utils::logging;
use anyhow::Result;
use serde_json::json;
use std::fs;
use std::path::Path;
use crate::utils::process;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::process::Command;

#[cfg(unix)]
fn run_first_success(candidates: &[(&str, &[&str])]) -> bool {
    candidates.iter().any(|(program, args)| process::run_best_effort(program, args))
}

/// Download and prepare APT binary seed
pub fn prepare_apt_seed(initramfs_root: &Path) -> Result<()> {
    logging::info("apt-seed", "Preparing APT binary seed", &[]);

    let bin_dir = initramfs_root.join("usr/bin");
    let lib_dir = initramfs_root.join("usr/lib");
    let etc_dir = initramfs_root.join("etc");
    let var_dir = initramfs_root.join("var/lib/apt");

    fs::create_dir_all(&bin_dir)?;
    fs::create_dir_all(&lib_dir)?;
    fs::create_dir_all(&etc_dir)?;
    fs::create_dir_all(&var_dir)?;

    // Create essential apt directories
    fs::create_dir_all(var_dir.join("lists/partial"))?;
    fs::create_dir_all(var_dir.join("cache/archives/partial"))?;
    fs::create_dir_all(initramfs_root.join("var/log"))?;

    // Create minimal apt configuration
    let apt_conf = r#"APT::Architecture "amd64";
APT::Build-Essential "build-essential";
Apt::Install-Recommends "false";
Apt::Install-Suggests "false";
"#;

    fs::create_dir_all(etc_dir.join("apt/apt.conf.d"))?;
    fs::write(etc_dir.join("apt/apt.conf.d/90-aethercore"), apt_conf)?;

    // Try to provision APT binary (Unix only)
    #[cfg(unix)]
    let provisioning_mode = {
        let apt_get_path = bin_dir.join("apt-get");
        if !apt_get_path.exists() {
            logging::info("apt-seed", "Attempting to provision apt-get binary", &[]);
            if install_apt_binary_unix(&bin_dir, &lib_dir).is_err() {
                logging::warn("apt-seed", "apt-get binary not available, package installation will require host support", &[]);
                "unix-provision-failed"
            } else {
                "unix-provisioned"
            }
        } else {
            "already-seeded"
        }
    };

    #[cfg(not(unix))]
    let provisioning_mode = {
        logging::info("apt-seed", "APT binary provisioning skipped on non-Unix platform (build host is not Unix)", &[]);
        "non-unix-build-host"
    };

    write_seed_capability_manifest(initramfs_root, provisioning_mode)?;

    logging::ready("apt-seed", "APT binary seed prepared", &initramfs_root.to_string_lossy());
    Ok(())
}

fn write_seed_capability_manifest(initramfs_root: &Path, provisioning_mode: &str) -> Result<()> {
    let bin_dir = initramfs_root.join("usr/bin");
    let lib_dir = initramfs_root.join("usr/lib");
    let aethercore_lib_dir = lib_dir.join("aethercore");
    fs::create_dir_all(&aethercore_lib_dir)?;

    let apt_seeded = bin_dir.join("apt-get").exists() || bin_dir.join("apt").exists();
    let dpkg_seeded = bin_dir.join("dpkg").exists();
    let xz_seeded = bin_dir.join("xz").exists() || bin_dir.join("unxz").exists();
    let loader_seeded = lib_dir.join("ld-linux-x86-64.so.2").exists()
        || lib_dir.join("ld-linux.so.2").exists();

    let manifest = json!({
        "schema": "aethercore.apt.seed.capability.v1",
        "provisioning_mode": provisioning_mode,
        "build_host": std::env::consts::OS,
        "seeded_tools": {
            "apt": apt_seeded,
            "dpkg": dpkg_seeded,
            "xz": xz_seeded
        },
        "seeded_runtime": {
            "dynamic_loader": loader_seeded
        },
        "package_stack_ready": apt_seeded && dpkg_seeded && xz_seeded && loader_seeded
    });

    fs::write(
        aethercore_lib_dir.join("apt-seed-capability.json"),
        serde_json::to_string_pretty(&manifest)?,
    )?;

    if provisioning_mode == "non-unix-build-host" {
        fs::write(
            aethercore_lib_dir.join("apt-seed-host-limitation.txt"),
            "APT binary closure cannot be fully seeded on non-Unix build hosts.\nRun apt-iso on Linux to seed apt-get/dpkg/loader shared libraries for runtime package installation.\n",
        )?;
    }

    Ok(())
}

/// Try to install apt-get binary from available sources (Unix only)
#[cfg(unix)]
fn install_apt_binary_unix(bin_dir: &Path, lib_dir: &Path) -> Result<()> {
    // Check if apt-get is available on host system
    if let Ok(output) = Command::new("which")
        .arg("apt-get")
        .output()
    {
        if output.status.success() {
            // Found apt-get on host, try to copy it to initramfs with dependencies
            if let Ok(path_str) = String::from_utf8(output.stdout) {
                let apt_path = path_str.trim();
                if Path::new(apt_path).exists() {
                    return copy_apt_with_dependencies(apt_path, bin_dir, lib_dir);
                }
            }
        }
    }

    // Try downloading from known sources (requires curl/wget)
    let urls = [
        "https://snapshot.debian.org/archive/debian/20230101T000000Z/pool/main/a/apt/apt_2.2.4-1~bpo11+1_amd64.deb",
        "https://github.com/aethercore-os/apt-binaries/releases/download/v2.2.4/apt-amd64.tar.gz",
    ];

    for url in &urls {
        logging::info("apt-seed", "Trying to download binary", &[("url", url)]);
        if try_download_apt(url, bin_dir).is_ok() {
            return Ok(());
        }
    }

    Err(anyhow::anyhow!("Could not obtain apt-get binary"))
}

/// Copy system apt-get with required dependencies to initramfs (Unix only)
#[cfg(unix)]
fn copy_apt_with_dependencies(apt_exe: &str, bin_dir: &Path, lib_dir: &Path) -> Result<()> {
    use std::io::Read;
    use std::io::Write;

    let apt_path = Path::new(apt_exe);
    if !apt_path.exists() {
        return Err(anyhow::anyhow!("apt-get not found at: {}", apt_exe));
    }

    // Copy apt-get executable
    let dest = bin_dir.join("apt-get");
    let mut src = fs::File::open(apt_path)?;
    let mut dst = fs::File::create(&dest)?;
    let mut buf = [0; 8192];
    loop {
        let n = src.read(&mut buf)?;
        if n == 0 { break; }
        dst.write_all(&buf[..n])?;
    }
    
    dst.sync_all()?;
    fs::set_permissions(&dest, fs::Permissions::from_mode(0o755))?;
    logging::info("apt-seed", "Copied apt-get binary", &[]);

    // Try to collect and copy essential dependencies
    if let Ok(output) = Command::new("ldd")
        .arg(apt_exe)
        .output()
    {
        let ldd_output = String::from_utf8_lossy(&output.stdout);
        for line in ldd_output.lines() {
            if let Some(path_start) = line.find('/') {
                let parts: Vec<&str> = line[path_start..]
                    .split_whitespace()
                    .collect();
                if !parts.is_empty() {
                    let lib_path = parts[0];
                    if let Ok(metadata) = fs::metadata(lib_path) {
                        if metadata.is_file() {
                            let lib_name = Path::new(lib_path)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("");
                            
                            let dest_lib = lib_dir.join(lib_name);

                            if !dest_lib.exists() {
                                match fs::copy(lib_path, &dest_lib) {
                                    Ok(_) => logging::info("apt-seed", "Copied library", &[("library", lib_name)]),
                                    Err(e) => logging::error("apt-seed", "Failed to copy library", &[("library", lib_name), ("error", &e.to_string())]),
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Try to download and extract apt binary from URL
#[cfg(unix)]
fn try_download_apt(url: &str, bin_dir: &Path) -> Result<()> {
    let temp_dir = std::env::temp_dir().join("aethercore-apt-seed");
    fs::create_dir_all(&temp_dir)?;

    let file_name = url.split('/').last().unwrap_or("apt-binary");
    let download_path = temp_dir.join(file_name);

    // Try curl first, then wget
    let download_success = run_first_success(&[
        ("curl", &["-fsSL", url, "-o", download_path.to_str().unwrap_or("")]),
        ("wget", &["-qO", download_path.to_str().unwrap_or(""), url]),
    ]);

    if !download_success {
        return Err(anyhow::anyhow!("Download failed"));
    }

    // Handle single binary file
    if file_name.ends_with(".tar.gz") || file_name.ends_with(".tgz") {
        // Try to extract gzipped tar archive using tar command
        if !run_first_success(&[("tar", &["-xzf", download_path.to_str().unwrap_or(""), "-C", temp_dir.to_str().unwrap_or("")])]) {
            return Err(anyhow::anyhow!("Failed to extract .tar.gz"));
        }

        // Copy extracted binaries
        for entry in fs::read_dir(&temp_dir)? {
            let entry = entry?;
            let path = entry.path();
            let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            
            if fname.starts_with("usr") || fname.starts_with("bin") {
                if let Ok(entries) = fs::read_dir(&path) {
                    for file in entries {
                        if let Ok(file) = file {
                            if let Ok(ft) = file.file_type() {
                                if ft.is_file() {
                                    let name = file.file_name();
                                    let dest = bin_dir.join(&name);
                                    let _ = fs::copy(file.path(), &dest);
                                    let _ = fs::set_permissions(&dest, fs::Permissions::from_mode(0o755));
                                }
                            }
                        }
                    }
                }
            }
        }
    } else {
        // Single binary file
        let dest = bin_dir.join(file_name);
        fs::copy(&download_path, &dest)?;
        fs::set_permissions(&dest, fs::Permissions::from_mode(0o755))?;
    }

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(())
}
