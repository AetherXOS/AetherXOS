use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

pub fn root() -> &'static Path {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
        .as_path()
}

pub fn path(rel: &str) -> PathBuf {
    root().join(rel)
}

pub fn read(rel: &str) -> String {
    std::fs::read_to_string(path(rel)).unwrap_or_else(|err| {
        panic!("failed to read {rel}: {err}");
    })
}

pub fn has(name: &str) -> bool {
    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path_var).any(|base| {
        let direct = base.join(name);
        let shell = base.join(format!("{name}.exe"));
        direct.is_file() || shell.is_file()
    })
}

pub fn lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}
