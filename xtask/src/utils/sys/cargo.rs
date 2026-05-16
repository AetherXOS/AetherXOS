use anyhow::Result;

pub fn cargo(args: &[&str]) -> Result<()> {
    crate::utils::sys::process::run_checked("cargo", args)
}

pub fn cargo_in_dir(args: &[&str], dir: &std::path::Path) -> Result<()> {
    crate::utils::sys::process::run_checked_in_dir("cargo", args, dir)
}

pub fn cargo_check_features(
    label: &str,
    features: &str,
    target: Option<&str>,
    release: bool,
) -> Result<()> {
    crate::utils::logging::info(
        "cargo",
        &format!("checking features for {}", label),
        &[("features", features)],
    );
    let mut args = vec!["check", "--features", features];
    if let Some(t) = target {
        args.push("--target");
        args.push(t);
    }
    if release {
        args.push("--release");
    }

    cargo(&args)
}

pub fn detect_host_triple() -> Result<String> {
    crate::utils::sys::process::run_capture("rustc", &["-vV"])?
        .lines()
        .find(|l| l.starts_with("host:"))
        .map(|l| l.replace("host:", "").trim().to_string())
        .ok_or_else(|| anyhow::anyhow!("Failed to detect host triple"))
}
