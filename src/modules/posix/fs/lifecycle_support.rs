use super::*;

pub fn mount_ramfs(path: &str) -> Result<u32, PosixErrno> {
    let fs =
        crate::modules::vfs::disk_fs::DiskFsLibrary::mount_ramfs_at(path).map_err(map_fs_error)?;
    let id = NEXT_FS_ID.fetch_add(1, Ordering::Relaxed);
    FS_CONTEXTS.lock().insert(id, fs);
    CWD_INDEX.lock().insert(id, String::from("/"));
    FILE_INDEX.lock().insert(id, BTreeSet::new());
    Ok(id)
}

pub fn mount_devfs(path: &str) -> Result<u32, PosixErrno> {
    let _ = normalize_path(path)?;
    let id = NEXT_FS_ID.fetch_add(1, Ordering::Relaxed);
    let devfs = Arc::new(DevFs::new());
    register_builtin_devfs_nodes(&devfs);
    sync_devfs_runtime_nodes(&devfs);
    DEVFS_CONTEXTS.lock().insert(id, devfs);
    CWD_INDEX.lock().insert(id, String::from("/"));
    FILE_INDEX.lock().insert(id, BTreeSet::new());
    Ok(id)
}

pub fn default_fs_id() -> Result<u32, PosixErrno> {
    FS_CONTEXTS
        .lock()
        .keys()
        .next()
        .copied()
        .ok_or(PosixErrno::BadFileDescriptor)
}

pub fn unmount(fs_id: u32) -> Result<(), PosixErrno> {
    if DEVFS_CONTEXTS.lock().remove(&fs_id).is_some() {
        CWD_INDEX.lock().remove(&fs_id);
        FILE_INDEX.lock().remove(&fs_id);
        return Ok(());
    }
    let fs = FS_CONTEXTS
        .lock()
        .remove(&fs_id)
        .ok_or(PosixErrno::BadFileDescriptor)?;
    let map_ids: alloc::vec::Vec<u32> = MMAP_TABLE
        .lock()
        .iter()
        .filter(|(_, map)| map.fs_id == fs_id)
        .map(|(id, _)| *id)
        .collect();
    for map_id in map_ids {
        let _ = msync(map_id);
        MMAP_TABLE.lock().remove(&map_id);
    }
    CWD_INDEX.lock().remove(&fs_id);
    FILE_INDEX.lock().remove(&fs_id);
    fs.unmount().map_err(map_fs_error)
}