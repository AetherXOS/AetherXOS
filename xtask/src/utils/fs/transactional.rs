use anyhow::{Result, Context};
use std::path::{Path, PathBuf};
use std::fs;

pub struct AtomicFileWrite {
    pub target: PathBuf,
    pub temp: PathBuf,
}

impl AtomicFileWrite {
    pub fn new(target: impl Into<PathBuf>) -> Self {
        let target = target.into();
        let mut temp = target.clone();
        temp.set_extension("tmp_atomic");
        Self { target, temp }
    }

    pub fn write(&self, content: &[u8]) -> Result<()> {
        fs::write(&self.temp, content)
            .context("Failed to write to temporary file for atomic operation")?;
        
        fs::rename(&self.temp, &self.target)
            .context("Failed to rename temporary file to target (Atomic commit failed)")?;
            
        Ok(())
    }
}

pub fn atomic_write(path: impl AsRef<Path>, content: &[u8]) -> Result<()> {
    AtomicFileWrite::new(path.as_ref()).write(content)
}

pub fn atomic_copy(src: impl AsRef<Path>, dest: impl AsRef<Path>) -> Result<()> {
    let mut temp = dest.as_ref().to_path_buf();
    temp.set_extension("tmp_copy");
    
    fs::copy(src.as_ref(), &temp)
        .context("Failed to copy to temporary location")?;
        
    fs::rename(&temp, dest.as_ref())
        .context("Failed to commit atomic copy")?;
        
    Ok(())
}
