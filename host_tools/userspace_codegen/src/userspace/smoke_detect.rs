use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub fn detect_host_c_compiler() -> Option<PathBuf> {
    if let Ok(cc) = env::var("CC") {
        let trimmed = cc.trim();
        if !trimmed.is_empty() {
            if let Some(found) = find_on_path(trimmed) {
                return Some(found);
            }
        }
    }
    for candidate in ["clang", "cc", "gcc"] {
        if let Some(found) = find_on_path(candidate) {
            return Some(found);
        }
    }
    None
}

pub fn detect_llvm_readobj() -> Option<PathBuf> {
    if let Some(found) = find_on_path("llvm-readobj") {
        return Some(found);
    }
    let clang = detect_host_c_compiler()?;
    let base = if cfg!(windows) {
        "llvm-readobj.exe"
    } else {
        "llvm-readobj"
    };
    sibling_if_exists(&clang, base)
}

pub fn detect_qemu_binary() -> Option<PathBuf> {
    if let Some(found) = find_on_path("qemu-system-x86_64") {
        return Some(found);
    }
    if cfg!(windows) {
        let program_files = env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".into());
        let candidate = PathBuf::from(program_files)
            .join("qemu")
            .join("qemu-system-x86_64.exe");
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

pub fn sibling_if_exists(path: &Path, base_name: &str) -> Option<PathBuf> {
    let candidate = path.with_file_name(if cfg!(windows) && !base_name.ends_with(".exe") {
        format!("{base_name}.exe")
    } else {
        base_name.to_string()
    });
    candidate.exists().then_some(candidate)
}

pub fn find_on_path(binary: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    let binary_names = if cfg!(windows) && !binary.ends_with(".exe") {
        vec![format!("{binary}.exe"), binary.to_string()]
    } else {
        vec![binary.to_string()]
    };
    for dir in env::split_paths(&path) {
        for name in &binary_names {
            let candidate = dir.join(name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}

pub fn compile_flags(userspace_dir: &Path, compiler: &Path) -> (String, PathBuf, Vec<OsString>) {
    let compiler_name = compiler
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mut target_mode = "host".to_string();
    let mut linker_bin = compiler.to_path_buf();
    let mut cflags = vec![
        OsString::from("-ffreestanding"),
        OsString::from("-fno-builtin"),
        OsString::from("-nostdlib"),
        OsString::from("-I"),
        userspace_dir.as_os_str().to_os_string(),
    ];
    if compiler_name.contains("clang") {
        target_mode = "elf".to_string();
        cflags.insert(0, OsString::from("--target=x86_64-unknown-none-elf"));
        if let Some(ld_lld) = sibling_if_exists(compiler, "ld.lld") {
            linker_bin = ld_lld;
        }
    }
    (target_mode, linker_bin, cflags)
}
