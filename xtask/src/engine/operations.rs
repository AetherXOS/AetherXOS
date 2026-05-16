use anyhow::{Result, Context};
use std::path::Path;
use crate::utils::{paths, logging};

/// Higher-level operations that encapsulate common filesystem and process patterns.
pub struct Op;

impl Op {
    /// Copies a file with logging and directory insurance.
    pub fn copy<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dest: Q, label: &str) -> Result<()> {
        let src = src.as_ref();
        let dest = dest.as_ref();
        
        if let Some(parent) = dest.parent() {
            paths::ensure_dir(parent)?;
        }
        
        logging::info(label, "Copying file", &[
            ("from", &src.to_string_lossy()),
            ("to", &dest.to_string_lossy())
        ]);
        
        std::fs::copy(src, dest)
            .with_context(|| format!("Failed to copy {} to {}", src.display(), dest.display()))?;
        Ok(())
    }

    /// Recursively copies a directory.
    pub fn copy_dir<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dest: Q, label: &str) -> Result<()> {
        let src = src.as_ref();
        let dest = dest.as_ref();
        
        logging::info(label, "Copying directory", &[
            ("from", &src.to_string_lossy()),
            ("to", &dest.to_string_lossy())
        ]);
        
        crate::utils::fs::copy_dir_all(src, dest)
            .with_context(|| format!("Failed to copy directory {} to {}", src.display(), dest.display()))?;
        Ok(())
    }

    /// Ensures a directory is clean (exists and is empty).
    pub fn clean_dir<P: AsRef<Path>>(path: P, label: &str) -> Result<()> {
        let path = path.as_ref();
        if path.exists() {
            logging::info(label, "Cleaning directory", &[("path", &path.to_string_lossy())]);
            std::fs::remove_dir_all(path)?;
        }
        paths::ensure_dir(path)?;
        Ok(())
    }
}
