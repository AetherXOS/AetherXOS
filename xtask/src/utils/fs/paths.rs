use std::sync::OnceLock;
use std::path::{Path, PathBuf};
use crate::utils::context;

static WORKSPACE_ROOT: OnceLock<PathBuf> = OnceLock::new();

pub fn repo_root() -> &'static Path {
    WORKSPACE_ROOT.get_or_init(|| context::repo_root())
}

pub fn resolve(relative: impl AsRef<Path>) -> PathBuf {
    let p = relative.as_ref();
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        repo_root().join(p)
    }
}

pub fn kernel_src(rel: impl AsRef<Path>) -> PathBuf { resolve("kernel/src").join(rel) }
pub fn artifacts() -> PathBuf { resolve("artifacts") }
pub fn staging() -> PathBuf { artifacts().join("stage") }

pub fn userspace_src(name: impl AsRef<Path>) -> PathBuf {
    resolve("kernel/src/userspace").join(name)
}

// Compatibility wrappers for existing code
pub fn resolve_str(p: &str) -> PathBuf { resolve(p) }
pub fn kernel_src_rel(rel: &str) -> PathBuf { kernel_src(rel) }

pub fn ensure_dir(dir: &Path) -> std::io::Result<()> {
    if !dir.exists() {
        std::fs::create_dir_all(dir)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repo_root_resolution() {
        let root = repo_root();
        assert!(root.is_absolute());
        assert!(root.exists());
    }

    #[test]
    fn test_path_resolution_mechanics() {
        let p = resolve("artifacts");
        assert!(p.is_absolute());
        assert!(p.ends_with("artifacts"));
    }
}
