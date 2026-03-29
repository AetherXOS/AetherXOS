use crate::interfaces::TaskId;
use crate::modules::vfs::FileSystem;
use alloc::boxed::Box;
use alloc::string::String;

#[inline(always)]
fn with_ramfs<T>(
    mount_id: usize,
    f: impl FnOnce(&crate::modules::vfs::RamFs) -> Result<T, &'static str>,
) -> Result<T, &'static str> {
    let instances = super::RAMFS_INSTANCES.lock();
    let (_, fs) = instances
        .iter()
        .find(|(id, _)| *id == mount_id)
        .ok_or(super::ERR_MOUNT_NOT_FOUND)?;
    f(fs)
}

#[inline(always)]
fn with_ramfs_mut<T>(
    mount_id: usize,
    f: impl FnOnce(&mut crate::modules::vfs::RamFs) -> Result<T, &'static str>,
) -> Result<T, &'static str> {
    let mut instances = super::RAMFS_INSTANCES.lock();
    let (_, fs) = instances
        .iter_mut()
        .find(|(id, _)| *id == mount_id)
        .ok_or(super::ERR_MOUNT_NOT_FOUND)?;
    f(fs)
}

pub fn ramfs_used_pages(mount_id: usize) -> Result<usize, &'static str> {
    with_ramfs(mount_id, |fs| Ok(fs.used_pages()))
}

pub fn ramfs_open_file(
    mount_id: usize,
    path: &str,
    tid: TaskId,
) -> Result<Box<dyn crate::modules::vfs::File>, &'static str> {
    with_ramfs(mount_id, |fs| fs.open(path, tid))
}

pub fn ramfs_create_file(
    mount_id: usize,
    path: &str,
    tid: TaskId,
) -> Result<Box<dyn crate::modules::vfs::File>, &'static str> {
    with_ramfs_mut(mount_id, |fs| fs.create(path, tid))
}

pub fn ramfs_remove_file(mount_id: usize, path: &str, tid: TaskId) -> Result<(), &'static str> {
    with_ramfs_mut(mount_id, |fs| fs.remove(path, tid))
}

pub fn ramfs_mkdir(mount_id: usize, path: &str, tid: TaskId) -> Result<(), &'static str> {
    with_ramfs_mut(mount_id, |fs| fs.mkdir(path, tid))
}

pub fn ramfs_rmdir(mount_id: usize, path: &str, tid: TaskId) -> Result<(), &'static str> {
    with_ramfs_mut(mount_id, |fs| fs.rmdir(path, tid))
}

pub fn ramfs_rename(
    mount_id: usize,
    old_path: &str,
    new_path: &str,
    tid: TaskId,
) -> Result<(), &'static str> {
    with_ramfs_mut(mount_id, |fs| fs.rename(old_path, new_path, tid))
}

pub fn ramfs_chmod(
    mount_id: usize,
    path: &str,
    mode: u16,
    tid: TaskId,
) -> Result<(), &'static str> {
    with_ramfs_mut(mount_id, |fs| fs.chmod(path, mode, tid))
}

pub fn ramfs_chown(
    mount_id: usize,
    path: &str,
    uid: u32,
    gid: u32,
    tid: TaskId,
) -> Result<(), &'static str> {
    with_ramfs_mut(mount_id, |fs| fs.chown(path, uid, gid, tid))
}

pub fn ramfs_link(
    mount_id: usize,
    old_path: &str,
    new_path: &str,
    tid: TaskId,
) -> Result<(), &'static str> {
    with_ramfs_mut(mount_id, |fs| fs.link(old_path, new_path, tid))
}

pub fn ramfs_symlink(
    mount_id: usize,
    target: &str,
    link_path: &str,
    tid: TaskId,
) -> Result<(), &'static str> {
    with_ramfs_mut(mount_id, |fs| fs.symlink(target, link_path, tid))
}

pub fn ramfs_readlink(mount_id: usize, path: &str, tid: TaskId) -> Result<String, &'static str> {
    with_ramfs(mount_id, |fs| fs.readlink(path, tid))
}

pub fn ramfs_readdir(
    mount_id: usize,
    path: &str,
    tid: TaskId,
) -> Result<alloc::vec::Vec<crate::modules::vfs::types::DirEntry>, &'static str> {
    with_ramfs(mount_id, |fs| fs.readdir(path, tid))
}

pub fn ramfs_metadata(
    mount_id: usize,
    path: &str,
    tid: TaskId,
) -> Result<crate::modules::vfs::types::FileStats, &'static str> {
    with_ramfs(mount_id, |fs| fs.stat(path, tid))
}

pub fn ramfs_set_times(
    mount_id: usize,
    path: &str,
    atime_sec: i64,
    atime_nsec: i32,
    mtime_sec: i64,
    mtime_nsec: i32,
    tid: TaskId,
) -> Result<(), &'static str> {
    with_ramfs_mut(mount_id, |fs| {
        fs.set_times(path, atime_sec, atime_nsec, mtime_sec, mtime_nsec, tid)
    })
}

pub fn load_initrd_entries(
    mount_id: usize,
    entries: &[(&str, &[u8])],
) -> Result<usize, &'static str> {
    super::INITRD_LOAD_CALLS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);

    with_ramfs_mut(mount_id, |fs| {
        let mut loaded = 0usize;
        for (path, data) in entries.iter().copied() {
            if !super::valid_initrd_path(path) {
                super::INITRD_LOAD_FAILURES.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                return Err("invalid initrd path");
            }

            let mut file = fs.create(path, super::ROOT_TASK_ID)?;

            file.seek(crate::modules::vfs::SeekFrom::Start(0))?;
            let wrote = file.write(data)?;
            file.flush()?;

            if wrote != data.len() {
                super::INITRD_LOAD_FAILURES.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                return Err("short initrd write");
            }

            loaded = loaded.saturating_add(1);
            super::INITRD_LOAD_FILES.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            super::INITRD_LOAD_BYTES.fetch_add(
                data.len() as u64,
                core::sync::atomic::Ordering::Relaxed,
            );
        }

        Ok(loaded)
    })
}

pub fn stats() -> super::MountStats {
    super::MountStats {
        mount_attempts: super::MOUNT_ATTEMPTS.load(core::sync::atomic::Ordering::Relaxed),
        mount_success: super::MOUNT_SUCCESS.load(core::sync::atomic::Ordering::Relaxed),
        mount_failures: super::MOUNT_FAILURES.load(core::sync::atomic::Ordering::Relaxed),
        unmount_attempts: super::UNMOUNT_ATTEMPTS.load(core::sync::atomic::Ordering::Relaxed),
        unmount_success: super::UNMOUNT_SUCCESS.load(core::sync::atomic::Ordering::Relaxed),
        unmount_failures: super::UNMOUNT_FAILURES.load(core::sync::atomic::Ordering::Relaxed),
        unmount_by_path_attempts: super::UNMOUNT_BY_PATH_ATTEMPTS
            .load(core::sync::atomic::Ordering::Relaxed),
        unmount_by_path_success: super::UNMOUNT_BY_PATH_SUCCESS
            .load(core::sync::atomic::Ordering::Relaxed),
        unmount_by_path_failures: super::UNMOUNT_BY_PATH_FAILURES
            .load(core::sync::atomic::Ordering::Relaxed),
        path_validation_failures: super::PATH_VALIDATION_FAILURES
            .load(core::sync::atomic::Ordering::Relaxed),
        initrd_load_calls: super::INITRD_LOAD_CALLS.load(core::sync::atomic::Ordering::Relaxed),
        initrd_load_files: super::INITRD_LOAD_FILES.load(core::sync::atomic::Ordering::Relaxed),
        initrd_load_bytes: super::INITRD_LOAD_BYTES.load(core::sync::atomic::Ordering::Relaxed),
        initrd_load_failures: super::INITRD_LOAD_FAILURES
            .load(core::sync::atomic::Ordering::Relaxed),
        total_mounts: super::mount_count(),
        last_mount_id: super::LAST_MOUNT_ID.load(core::sync::atomic::Ordering::Relaxed),
    }
}
