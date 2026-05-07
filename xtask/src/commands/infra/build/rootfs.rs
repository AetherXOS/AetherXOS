use anyhow::{Result, anyhow, bail};
use std::path::Path;
use crate::utils::{logging, process, wsl, config};

/// Attempt to extract a tar/tar.gz archive into `dst`, first using host `tar`.
/// On Windows, if host `tar` fails and WSL is available, try extracting via `wsl -- tar ...`
/// Attempt to extract a tar/tar.gz/iso archive into `dst`.
pub fn extract_rootfs_archive(src: &Path, dst: &Path) -> Result<()> {
    let src_s = src.to_string_lossy().to_string();
    let dst_s = dst.to_string_lossy().to_string();
    let is_iso = src_s.to_lowercase().ends_with(".iso");

    let mut tried_tools = Vec::new();

    // 1. Try 7z
    if cfg!(windows) && process::which("7z") && !config::prefer_wsl_extraction() {
        tried_tools.push("7z");
        logging::info("image", "extracting via 7z", &[("src", &src_s)]);
        let (status, stdout, stderr) = process::run_with_output("7z", &["x", &src_s, &format!("-o{}", dst_s), "-y"])?;
        
        if status.success() {
            return Ok(());
        }

        logging::warn("image", "7z extraction failed", &[("status", &status.to_string())]);
        if stdout.contains("Data Error") || stderr.contains("Data Error") {
            logging::error("image", "detected corruption (Data Error) in archive", &[]);
            if config::is_non_interactive() {
                return Err(anyhow!("REDOWNLOAD_REQUESTED"));
            }

            let options = ["Redownload and Retry", "Try other tools anyway", "Abort"];
            let choice = crate::utils::ui::select("7z detected data corruption. What to do?", &options)?;
            if *choice == "Redownload and Retry" {
                return Err(anyhow!("REDOWNLOAD_REQUESTED"));
            }
            if *choice == "Abort" {
                bail!("Extraction aborted due to corruption.");
            }
        }
    }

    // 2. Try host tar (unless WSL preferred and available)
    if !config::prefer_wsl_extraction() && process::which("tar") {
        tried_tools.push("tar");
        logging::info("image", "extracting via host tar", &[("src", &src_s)]);
        let args = if is_iso {
            vec!["-xf", &src_s, "-C", &dst_s]
        } else {
            vec!["-xpf", &src_s, "-C", &dst_s]
        };
        
        let (status, _, stderr) = process::run_with_output("tar", &args)?;
        if status.success() {
            return Ok(());
        }
        
        logging::warn("image", "host tar failed", &[("error", &stderr.trim())]);
        if stderr.contains("Truncated input file") {
            if config::is_non_interactive() {
                return Err(anyhow!("REDOWNLOAD_REQUESTED"));
            }
            if crate::utils::ui::confirm("tar detected truncated file. Redownload?", true)? {
                return Err(anyhow!("REDOWNLOAD_REQUESTED"));
            }
        }
    }

    // 3. WSL Fallback
    if cfg!(windows) && process::which("wsl") {
        tried_tools.push("wsl tar");
        logging::info("image", "attempting extraction via WSL fallback", &[]);

        let src_w = wsl::to_wsl_path(src)?;
        let dst_w = wsl::to_wsl_path(dst)?;

        let cmd = if is_iso {
            format!("tar -xf {} -C {}", src_w, dst_w)
        } else {
            format!(
                "tar -xpf {} -C {} --exclude=dev/* --exclude=proc/* --exclude=sys/* --exclude=run/* --exclude=var/lock --exclude=var/run --exclude=var/spool/mail",
                src_w, dst_w
            )
        };

        if wsl::run_in_wsl(&cmd, &["tar"]).is_ok() {
            return Ok(());
        }
    }

    let err_msg = format!(
        "Failed to extract archive {}. Tried tools: {}. Archive might be corrupted or tools are missing.",
        src_s, tried_tools.join(", ")
    );
    
    let final_options = ["Redownload Image", "Abort"];
    if config::is_non_interactive() {
        return Err(anyhow!("REDOWNLOAD_REQUESTED"));
    }

    let final_choice = crate::utils::ui::select(&format!("{}\nWhat to do?", err_msg), &final_options)?;
    if *final_choice == "Redownload Image" {
        return Err(anyhow!("REDOWNLOAD_REQUESTED"));
    }

    bail!("Extraction failed: {}", err_msg)
}
