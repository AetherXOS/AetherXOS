use anyhow::Result;

/// A trait for commands that can be executed independently.
pub trait Executable {
    fn execute(&self) -> Result<()>;
}

/// Check if an executable exists in the system PATH.
pub fn find_in_path(name: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let full_path = dir.join(name);
        if full_path.is_file() {
            return Some(full_path);
        }
        // Also check with .exe for Windows
        #[cfg(windows)]
        {
            let exe_path = dir.join(format!("{}.exe", name));
            if exe_path.is_file() {
                return Some(exe_path);
            }
        }
    }
    None
}
