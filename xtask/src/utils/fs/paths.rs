use once_cell::sync::Lazy;
use std::env;
use std::path::PathBuf;

/// Centralized topology of the Aether X OS project structure.
/// This is the SINGLE source of truth for all project paths.
pub struct ProjectLayout {
    pub root: PathBuf,
    pub artifacts: PathBuf,
    pub staging: PathBuf,
    pub distros: PathBuf,
    pub target: PathBuf,
    pub logs: PathBuf,
}

impl ProjectLayout {
    pub fn discover() -> Self {
        let root = env::var("CARGO_MANIFEST_DIR")
            .map(|p| PathBuf::from(p).parent().unwrap().to_path_buf())
            .unwrap_or_else(|_| env::current_dir().unwrap());

        Self {
            root: root.clone(),
            artifacts: root.join("artifacts"),
            staging: root.join("staging"),
            distros: root.join("distros"),
            target: root.join("target"),
            logs: root.join("xtask.log"),
        }
    }

    /// Ensures all critical workspace directories exist.
    pub fn ensure_hermetic(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.artifacts)?;
        Ok(())
    }
}

pub static LAYOUT: Lazy<ProjectLayout> = Lazy::new(ProjectLayout::discover);
