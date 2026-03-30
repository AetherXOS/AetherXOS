use anyhow::{Result, bail};

use crate::cli::SetupAction;
use crate::utils::process;

/// Entry point for `cargo xtask setup <action>`.
pub fn execute(action: &SetupAction) -> Result<()> {
    match action {
        SetupAction::Audit => {
            println!("[setup::audit] Auditing host environment for missing dependencies");
            run_audit()
        }
        SetupAction::Repair => {
            // ... other code ...
            println!("[setup::repair] Auto-repairing host environment");
            run_repair()
        }
        SetupAction::Bootstrap => {
            println!("[setup::bootstrap] Full workspace bootstrap pipeline");
            println!("[setup::bootstrap]   1. Environment audit");
            println!("[setup::bootstrap]   2. Auto-repair");
            println!("[setup::bootstrap]   3. Build + smoke test");
            println!("[setup::bootstrap]   4. Dashboard generation");
            run_audit()?;
            run_repair()?;
            crate::commands::infra::build::execute(&crate::cli::BuildAction::Full)?;
            crate::commands::ops::qemu::smoke_test()?;
            crate::commands::dashboard::execute(&crate::cli::DashboardAction::Build)
        }
        SetupAction::BootstrapFlutter { outdir, flutter_url, kernel_image } => {
            println!("[setup::bootstrap-flutter] Orchestrating Debian rootfs + Flutter bootstrap: outdir={} flutter_url={} kernel_image={}", outdir, flutter_url, kernel_image);
            run_bootstrap_flutter(outdir, flutter_url, kernel_image)
        }
    }
}

fn run_bootstrap_flutter(outdir: &str, flutter_url: &str, kernel_image: &str) -> Result<()> {
    // Use the external script under xtask/scripts if available.
    let script = crate::utils::paths::resolve("xtask/scripts/bootstrap_flutter.sh");
    if !script.exists() {
        bail!("bootstrap script not found: {}", script.display());
    }

    let out_abs = crate::utils::paths::resolve(outdir);
    crate::utils::paths::ensure_dir(&out_abs)?;

    let script_path = script.to_string_lossy().to_string();
    let out_path = out_abs.to_string_lossy().to_string();

    // Prefer native bash when available (Linux, macOS, or Git Bash on Windows).
    if crate::utils::process::which("bash") {
        process::run_checked("bash", &[&script_path, &out_path, flutter_url, kernel_image])?;
        return Ok(());
    }

    // Docker fallback (works on Windows without WSL if Docker Desktop is installed).
    if crate::utils::process::which("docker") {
        let script_dir = script.parent().unwrap().to_string_lossy().to_string();
        // Run a small Debian container, install runtime deps, then execute the script from the mounted /scripts dir.
        let docker_cmd = format!(
            "apt-get update && apt-get install -y debootstrap curl fakeroot dpkg rsync e2fsprogs util-linux losetup && bash /scripts/bootstrap_flutter.sh /out '{}' '{}' '{}'",
            out_path.replace('\'' , "'\\''"),
            flutter_url.replace('\'' , "'\\''"),
            kernel_image.replace('\'' , "'\\''"),
        );
        process::run_checked(
            "docker",
            &[
                "run",
                "--rm",
                "-v",
                &format!("{}:/scripts:ro", script_dir),
                "-v",
                &format!("{}:/out", out_path),
                "debian:stable-slim",
                "bash",
                "-lc",
                &docker_cmd,
            ],
        )?;
        return Ok(());
    }

    // As a last resort on Windows, try WSL if present.
    if cfg!(windows) && crate::utils::process::which("wsl") {
        // Convert Windows paths to WSL /mnt style
        fn to_wsl_path(p: &str) -> String {
            let mut s = p.replace('\\', "/");
            if s.len() >= 2 && s.as_bytes()[1] == b':' {
                let drive = s.chars().next().unwrap().to_ascii_lowercase();
                let rest = &s[2..];
                s = format!("/mnt/{}/{}", drive, rest.trim_start_matches('/'));
            }
            s
        }
        let script_wsl = to_wsl_path(&script_path);
        let out_wsl = to_wsl_path(&out_path);
        process::run_checked("wsl", &["bash", &script_wsl, &out_wsl, flutter_url, kernel_image])?;
        return Ok(());
    }

    bail!("No suitable execution environment found: need bash, docker, or wsl available on PATH");
}

fn run_audit() -> Result<()> {
    let required = [
        "cargo",
        "rustc",
        "python",
        "npm",
    ];
    let mut missing = Vec::new();
    for bin in required {
        if process::which(bin) {
            println!("[setup::audit] OK: {}", bin);
        } else {
            println!("[setup::audit] MISSING: {}", bin);
            missing.push(bin.to_string());
        }
    }
    if process::which("qemu-system-x86_64") {
        println!("[setup::audit] OK: qemu-system-x86_64");
    } else {
        println!("[setup::audit] WARN: qemu-system-x86_64 not found (smoke tests limited)");
    }
    if missing.is_empty() {
        Ok(())
    } else {
        bail!("missing required tools: {}", missing.join(", "))
    }
}

fn run_repair() -> Result<()> {
    // Native repair is intentionally conservative: validate core toolchain and
    // install workspace dependencies where package managers are already present.
    if process::which("npm") {
        let dashboard_dir = crate::utils::paths::resolve("dashboard");
        let _ = process::run_status_in_dir("npm", &["install"], &dashboard_dir)?;
    }
    if process::which("python") {
        let _ = process::run_status_in_dir("python", &["-m", "pip", "install", "-r", "requirements.txt"], &crate::utils::paths::repo_root());
    }
    println!("[setup::repair] Completed best-effort dependency repair");
    Ok(())
}
