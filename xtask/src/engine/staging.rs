use crate::utils::logging;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct StagingArea {
    pub root: PathBuf,
}

impl StagingArea {
    pub fn new(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    pub fn clear(&self) -> Result<()> {
        if self.root.exists() {
            logging::info(
                "STAGING",
                "Clearing staging area",
                &[("path", &self.root.to_string_lossy())],
            );

            // On Windows, directories can be locked by indexed searches or antivirus.
            // We'll try a few times before giving up.
            let mut retries = 3;
            while retries > 0 {
                if let Err(e) = fs::remove_dir_all(&self.root) {
                    if retries == 1 {
                        return Err(e).context(format!(
                            "Failed to clear staging area at {}",
                            self.root.display()
                        ));
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    retries -= 1;
                } else {
                    break;
                }
            }
            fs::create_dir_all(&self.root)?;
        }
        Ok(())
    }

    pub fn copy_file(&self, src: &Path, rel_dest: &str) -> Result<PathBuf> {
        let dest = self.root.join(rel_dest);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, &dest)?;
        Ok(dest)
    }

    pub fn write_file(&self, rel_dest: &str, content: &[u8]) -> Result<PathBuf> {
        let dest = self.root.join(rel_dest);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&dest, content)?;
        Ok(dest)
    }
}
