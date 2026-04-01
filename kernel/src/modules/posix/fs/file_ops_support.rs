use super::*;
use crate::modules::posix::time;

pub(super) fn set_mtime_now(fs_id: u32, path: &str) {
    let now = time::monotonic_timespec();
    let contexts = FS_CONTEXTS.lock();
    if let Some(fs) = contexts.get(&fs_id) {
        let _ = fs.set_times(path, now.sec as u64, now.sec as u64, TaskId(0));
    }
}

pub fn unlink(fs_id: u32, path: &str) -> Result<(), PosixErrno> {
    let normalized = normalize_path(path)?;
    if normalized == "/" {
        return Err(PosixErrno::Invalid);
    }
    if let Some(devfs) = devfs_context(fs_id) {
        sync_devfs_runtime_nodes(&devfs);
        devfs.remove(&normalized, TaskId(0)).map_err(map_fs_error)?;
        if let Some(files) = FILE_INDEX.lock().get_mut(&fs_id) {
            files.remove(&normalized);
        }
        return Ok(());
    }
    let contexts = FS_CONTEXTS.lock();
    let fs = contexts.get(&fs_id).ok_or(PosixErrno::BadFileDescriptor)?;
    fs.remove(&normalized, TaskId(0)).map_err(map_fs_error)?;
    if let Some(files) = FILE_INDEX.lock().get_mut(&fs_id) {
        files.remove(&normalized);
    }
    Ok(())
}

pub fn copy_file_range(fs_id: u32, src_path: &str, dst_path: &str) -> Result<usize, PosixErrno> {
    let src_n = normalize_path(src_path)?;
    let dst_n = normalize_path(dst_path)?;

    let contexts = FS_CONTEXTS.lock();
    let fs = contexts.get(&fs_id).ok_or(PosixErrno::BadFileDescriptor)?;
    let data = fs.read_all(&src_n).map_err(map_fs_error)?;
    fs.write_all(&dst_n, &data).map_err(map_fs_error)?;
    set_mtime_now(fs_id, &dst_n);
    FILE_INDEX
        .lock()
        .entry(fs_id)
        .or_insert_with(BTreeSet::new)
        .insert(dst_n);
    Ok(data.len())
}