use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

#[derive(Debug)]
pub struct AppContext {
    repo_root: PathBuf,
    outdir: PathBuf,
    host_target: String,
}

static APP_CONTEXT: OnceLock<AppContext> = OnceLock::new();

pub fn init(outdir: PathBuf) -> Result<()> {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask must be nested one level under repo root")
        .to_path_buf();
    let host_target = detect_host_triple()?;

    APP_CONTEXT
        .set(AppContext {
            repo_root,
            outdir,
            host_target,
        })
        .map_err(|_| anyhow::anyhow!("xtask context initialized more than once"))?;

    Ok(())
}

pub fn repo_root() -> PathBuf {
    APP_CONTEXT
        .get()
        .map(|ctx| ctx.repo_root.clone())
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("xtask must be nested one level under repo root")
                .to_path_buf()
        })
}

pub fn out_dir() -> PathBuf {
    APP_CONTEXT
        .get()
        .map(|ctx| ctx.outdir.clone())
        .unwrap_or_else(|| repo_root().join("artifacts"))
}

pub fn host_target() -> Result<&'static str> {
    APP_CONTEXT
        .get()
        .map(|ctx| ctx.host_target.as_str())
        .ok_or_else(|| anyhow::anyhow!("xtask context not initialized"))
}

fn detect_host_triple() -> Result<String> {
    let output = Command::new("rustc")
        .args(["-vV"])
        .output()
        .context("Failed to run rustc -vV")?;

    if !output.status.success() {
        bail!(
            "rustc -vV failed with exit code {}",
            output.status.code().unwrap_or(-1)
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Some(triple) = line.strip_prefix("host: ") {
            return Ok(triple.trim().to_string());
        }
    }

    bail!("Could not detect host triple from rustc output")
}
