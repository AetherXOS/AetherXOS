use alloc::string::String;

use super::{
    access, link, lstat, mkdir, normalize_path, open, readlink, rename, stat, symlink, unlink,
    PosixErrno, PosixStat, CWD_INDEX,
};

pub fn joined_child_path(dir: &str, name: &str) -> Result<String, PosixErrno> {
    if name.starts_with('/') {
        return normalize_path(name);
    }
    let base = normalize_path(dir)?;
    if base == "/" {
        normalize_path(&alloc::format!("/{}", name))
    } else {
        normalize_path(&alloc::format!("{}/{}", base, name))
    }
}

pub fn resolve_at_path(fs_id: u32, dir: &str, path: &str) -> Result<String, PosixErrno> {
    let _ = fs_id;
    if path.starts_with('/') {
        return normalize_path(path);
    }
    let dir_n = normalize_path(dir)?;
    joined_child_path(&dir_n, path)
}

pub fn openat(fs_id: u32, dir: &str, path: &str, create: bool) -> Result<u32, PosixErrno> {
    let resolved = resolve_at_path(fs_id, dir, path)?;
    open(fs_id, &resolved, create)
}

pub fn mkdirat(fs_id: u32, dir: &str, path: &str, mode: u16) -> Result<(), PosixErrno> {
    let resolved = resolve_at_path(fs_id, dir, path)?;
    mkdir(fs_id, &resolved, mode)
}

pub fn faccessat(fs_id: u32, dir: &str, path: &str) -> Result<bool, PosixErrno> {
    let resolved = resolve_at_path(fs_id, dir, path)?;
    access(fs_id, &resolved)
}

pub fn fstatat(
    fs_id: u32,
    dir: &str,
    path: &str,
    follow_symlink: bool,
) -> Result<PosixStat, PosixErrno> {
    let resolved = resolve_at_path(fs_id, dir, path)?;
    if follow_symlink {
        stat(fs_id, &resolved)
    } else {
        lstat(fs_id, &resolved)
    }
}

pub fn readlinkat(fs_id: u32, dir: &str, path: &str) -> Result<String, PosixErrno> {
    let resolved = resolve_at_path(fs_id, dir, path)?;
    readlink(fs_id, &resolved)
}

pub fn symlinkat(
    fs_id: u32,
    target: &str,
    new_dir: &str,
    new_name: &str,
) -> Result<(), PosixErrno> {
    let link_path = joined_child_path(new_dir, new_name)?;
    symlink(fs_id, target, &link_path)
}

pub fn linkat(
    fs_id: u32,
    old_dir: &str,
    old_name: &str,
    new_dir: &str,
    new_name: &str,
) -> Result<(), PosixErrno> {
    let old_path = joined_child_path(old_dir, old_name)?;
    let new_path = joined_child_path(new_dir, new_name)?;
    link(fs_id, &old_path, &new_path)
}

pub fn renameat(
    fs_id: u32,
    old_dir: &str,
    old_name: &str,
    new_dir: &str,
    new_name: &str,
) -> Result<(), PosixErrno> {
    let old_p = joined_child_path(old_dir, old_name)?;
    let new_p = joined_child_path(new_dir, new_name)?;
    rename(fs_id, &old_p, &new_p)
}

pub fn unlinkat(fs_id: u32, dir: &str, name: &str) -> Result<(), PosixErrno> {
    let path = joined_child_path(dir, name)?;
    unlink(fs_id, &path)
}

pub fn realpath(fs_id: u32, path: &str) -> Result<String, PosixErrno> {
    let normalized = normalize_path(path)?;
    if access(fs_id, &normalized)? {
        Ok(normalized)
    } else {
        Err(PosixErrno::NoEntry)
    }
}

pub fn chdir(fs_id: u32, path: &str) -> Result<(), PosixErrno> {
    let normalized = normalize_path(path)?;
    let st = stat(fs_id, &normalized)?;
    if !st.is_dir {
        return Err(PosixErrno::Invalid);
    }
    CWD_INDEX.lock().insert(fs_id, normalized);
    Ok(())
}

pub fn getcwd(fs_id: u32) -> Result<String, PosixErrno> {
    CWD_INDEX
        .lock()
        .get(&fs_id)
        .cloned()
        .ok_or(PosixErrno::BadFileDescriptor)
}
