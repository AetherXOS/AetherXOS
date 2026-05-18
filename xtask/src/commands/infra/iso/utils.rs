use anyhow::Result;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn unique_iso_root(out_iso: &Path) -> Result<PathBuf> {
    let parent = out_iso.parent().ok_or_else(|| {
        anyhow::anyhow!("Output ISO has no parent directory: {}", out_iso.display())
    })?;
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    Ok(parent.join(format!("iso_root_{}", ts)))
}
