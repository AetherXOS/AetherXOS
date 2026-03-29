use super::*;
use super::file_ops_support::set_mtime_now;
use crate::modules::posix::time;

#[derive(Debug, Clone, Copy)]
pub struct PosixFsStats {
    pub f_type: u64,
    pub f_bsize: u64,
    pub f_blocks: u64,
    pub f_bfree: u64,
    pub f_bavail: u64,
    pub f_files: u64,
    pub f_ffree: u64,
    pub f_fsid: u64,
    pub f_namelen: u64,
}

pub fn stat(fs_id: u32, path: &str) -> Result<PosixStat, PosixErrno> {
    let normalized = normalize_path(path)?;
    if let Some(devfs) = devfs_context(fs_id) {
        sync_devfs_runtime_nodes(&devfs);
        let md = devfs.stat(&normalized, TaskId(0)).map_err(map_fs_error)?;
        return Ok(PosixStat {
            size: md.size,
            mode: md.mode as u16,
            uid: md.uid,
            gid: md.gid,
            is_dir: (md.mode & 0o170000) == 0o040000,
            is_symlink: (md.mode & 0o170000) == 0o120000,
            ino: 0,
            atime: md.atime as i64,
            mtime: md.mtime as i64,
            ctime: md.ctime as i64,
        });
    }
    let contexts = FS_CONTEXTS.lock();
    let fs = contexts.get(&fs_id).ok_or(PosixErrno::BadFileDescriptor)?;
    let md = fs.stat(&normalized, TaskId(0)).map_err(map_fs_error)?;
    Ok(PosixStat {
        size: md.size,
        mode: md.mode as u16,
        uid: md.uid,
        gid: md.gid,
        is_dir: (md.mode & 0o170000) == 0o040000,
        is_symlink: (md.mode & 0o170000) == 0o120000,
        ino: 0,
        atime: md.atime as i64,
        mtime: md.mtime as i64,
        ctime: md.ctime as i64,
    })
}

pub fn statfs(fs_id: u32) -> Result<PosixFsStats, PosixErrno> {
    let contexts = FS_CONTEXTS.lock();
    let fs = contexts.get(&fs_id).ok_or(PosixErrno::BadFileDescriptor)?;
    let _ = fs;
    let free_pages = crate::modules::allocators::bitmap_pmm::get_free_pages() as u64;
    let total_pages = crate::modules::allocators::bitmap_pmm::PMM_TOTAL_PAGES as u64;
    Ok(PosixFsStats {
        f_type: 0,
        f_bsize: 4096,
        f_blocks: total_pages,
        f_bfree: free_pages,
        f_bavail: free_pages,
        f_files: 0,
        f_ffree: 0,
        f_fsid: fs_id as u64,
        f_namelen: 255,
    })
}

pub fn fstat(fd: u32) -> Result<PosixStat, PosixErrno> {
    let shared = {
        let table = FILE_TABLE.lock();
        table
            .get(&fd)
            .ok_or(PosixErrno::BadFileDescriptor)?
            .file
            .clone()
    };
    let md = shared.handle.lock().stat().map_err(map_fs_error)?;

    Ok(PosixStat {
        size: md.size,
        mode: md.mode as u16,
        uid: md.uid,
        gid: md.gid,
        is_dir: (md.mode as u32 & 0o170000) == 0o040000,
        is_symlink: (md.mode as u32 & 0o170000) == 0o120000,
        ino: 0,
        atime: md.atime as i64,
        mtime: md.mtime as i64,
        ctime: md.ctime as i64,
    })
}

pub fn lstat(fs_id: u32, path: &str) -> Result<PosixStat, PosixErrno> {
    stat(fs_id, path)
}

pub fn access(fs_id: u32, path: &str) -> Result<bool, PosixErrno> {
    let normalized = normalize_path(path)?;
    if let Some(devfs) = devfs_context(fs_id) {
        sync_devfs_runtime_nodes(&devfs);
        return match devfs.stat(&normalized, TaskId(0)) {
            Ok(_) => Ok(true),
            Err("device not found") | Err("not found") => Ok(false),
            Err(err) => Err(map_fs_error(err)),
        };
    }
    let contexts = FS_CONTEXTS.lock();
    let fs = contexts.get(&fs_id).ok_or(PosixErrno::BadFileDescriptor)?;
    match fs.stat(&normalized, TaskId(0)) {
        Ok(_) => Ok(true),
        Err("file not found") => Ok(false),
        Err(err) => Err(map_fs_error(err)),
    }
}

pub fn mkdir(fs_id: u32, path: &str, mode: u16) -> Result<(), PosixErrno> {
    let _ = mode;
    let normalized = normalize_path(path)?;
    if devfs_context(fs_id).is_some() {
        return Err(PosixErrno::NotSupported);
    }
    let contexts = FS_CONTEXTS.lock();
    let fs = contexts.get(&fs_id).ok_or(PosixErrno::BadFileDescriptor)?;
    fs.mkdir(&normalized, TaskId(0)).map_err(map_fs_error)
}

pub fn rmdir(fs_id: u32, path: &str) -> Result<(), PosixErrno> {
    let normalized = normalize_path(path)?;
    if devfs_context(fs_id).is_some() {
        return Err(PosixErrno::NotSupported);
    }
    let contexts = FS_CONTEXTS.lock();
    let fs = contexts.get(&fs_id).ok_or(PosixErrno::BadFileDescriptor)?;
    fs.rmdir(&normalized, TaskId(0)).map_err(map_fs_error)
}

pub fn rename(fs_id: u32, old_path: &str, new_path: &str) -> Result<(), PosixErrno> {
    let old_n = normalize_path(old_path)?;
    let new_n = normalize_path(new_path)?;
    if devfs_context(fs_id).is_some() {
        return Err(PosixErrno::NotSupported);
    }
    let contexts = FS_CONTEXTS.lock();
    let fs = contexts.get(&fs_id).ok_or(PosixErrno::BadFileDescriptor)?;
    fs.rename(&old_n, &new_n, TaskId(0)).map_err(map_fs_error)?;

    if let Some(files) = FILE_INDEX.lock().get_mut(&fs_id) {
        if files.remove(&old_n) {
            files.insert(new_n.clone());
        }
        let old_prefix = alloc::format!("{}/", old_n);
        let new_prefix = alloc::format!("{}/", new_n);
        let affected: alloc::vec::Vec<String> = files
            .iter()
            .filter(|p| p.starts_with(&old_prefix))
            .cloned()
            .collect();
        for entry in affected {
            files.remove(&entry);
            files.insert(entry.replacen(&old_prefix, &new_prefix, 1));
        }
    }

    Ok(())
}

pub fn opendir(fs_id: u32, path: &str) -> Result<u32, PosixErrno> {
    let normalized = normalize_path(path)?;
    if let Some(devfs) = devfs_context(fs_id) {
        sync_devfs_runtime_nodes(&devfs);
        let entries = devfs
            .readdir(&normalized, TaskId(0))
            .map_err(map_fs_error)?;
        let mut out = VecDeque::new();
        for entry in entries {
            let name = entry.name;
            if name.is_empty() {
                continue;
            }
            out.push_back(name);
        }
        let dirfd = NEXT_DIRFD.fetch_add(1, Ordering::Relaxed);
        DIR_TABLE.lock().insert(dirfd, out);
        return Ok(dirfd);
    }
    let contexts = FS_CONTEXTS.lock();
    let fs = contexts.get(&fs_id).ok_or(PosixErrno::BadFileDescriptor)?;
    let entries = fs.readdir(&normalized, TaskId(0)).map_err(map_fs_error)?;
    let mut out = VecDeque::new();
    for entry in entries {
        let name = entry.name;
        if name.is_empty() {
            continue;
        }
        out.push_back(name);
    }
    let dirfd = NEXT_DIRFD.fetch_add(1, Ordering::Relaxed);
    DIR_TABLE.lock().insert(dirfd, out);
    Ok(dirfd)
}

pub fn readdir(dirfd: u32) -> Result<Option<String>, PosixErrno> {
    let mut table = DIR_TABLE.lock();
    let dir = table.get_mut(&dirfd).ok_or(PosixErrno::BadFileDescriptor)?;
    Ok(dir.pop_front())
}

pub fn closedir(dirfd: u32) -> Result<(), PosixErrno> {
    let removed = DIR_TABLE.lock().remove(&dirfd);
    if removed.is_some() {
        Ok(())
    } else {
        Err(PosixErrno::BadFileDescriptor)
    }
}

pub fn scandir(fs_id: u32, path: &str) -> Result<alloc::vec::Vec<String>, PosixErrno> {
    let dirfd = opendir(fs_id, path)?;
    let mut out = alloc::vec::Vec::new();
    while let Some(name) = readdir(dirfd)? {
        out.push(name);
    }
    closedir(dirfd)?;
    Ok(out)
}

pub fn truncate(fs_id: u32, path: &str, len: usize) -> Result<(), PosixErrno> {
    let normalized = normalize_path(path)?;
    if devfs_context(fs_id).is_some() {
        return Err(PosixErrno::NotSupported);
    }
    let contexts = FS_CONTEXTS.lock();
    let fs = contexts.get(&fs_id).ok_or(PosixErrno::BadFileDescriptor)?;
    let mut data = fs.read_all(&normalized).map_err(map_fs_error)?;
    data.resize(len, 0);
    let _ = fs.write_all(&normalized, &data).map_err(map_fs_error)?;
    set_mtime_now(fs_id, &normalized);
    Ok(())
}

pub fn ftruncate(fd: u32, len: usize) -> Result<(), PosixErrno> {
    let table = FILE_TABLE.lock();
    let desc = table.get(&fd).ok_or(PosixErrno::BadFileDescriptor)?;
    let res = desc
        .file
        .handle
        .lock()
        .truncate(len as u64)
        .map_err(map_fs_error);
    res
}

pub fn poll(fds: &mut [(u32, u16, &mut u16)]) -> Result<usize, PosixErrno> {
    let table = FILE_TABLE.lock();
    let mut ready = 0usize;
    for (fd, events, revents) in fds.iter_mut() {
        if let Some(desc) = table.get(fd) {
            **revents = (desc.file.handle.lock().poll_events().bits() as u16) & *events;
            if **revents != 0 {
                ready += 1;
            }
        } else {
            **revents = crate::modules::posix_consts::net::POLLNVAL as u16;
        }
    }
    Ok(ready)
}

pub fn chmod(fs_id: u32, path: &str, mode: u16) -> Result<(), PosixErrno> {
    let normalized = normalize_path(path)?;
    if let Some(devfs) = devfs_context(fs_id) {
        sync_devfs_runtime_nodes(&devfs);
        return devfs
            .chmod(&normalized, mode, TaskId(0))
            .map_err(map_fs_error);
    }
    let contexts = FS_CONTEXTS.lock();
    let fs = contexts.get(&fs_id).ok_or(PosixErrno::BadFileDescriptor)?;
    fs.chmod(&normalized, mode, TaskId(0)).map_err(map_fs_error)
}

pub fn fchmod(fd: u32, mode: u16) -> Result<(), PosixErrno> {
    let shared = {
        let table = FILE_TABLE.lock();
        table
            .get(&fd)
            .ok_or(PosixErrno::BadFileDescriptor)?
            .file
            .clone()
    };
    chmod(shared.fs_id, &shared.path, mode)
}

pub fn chown(fs_id: u32, path: &str, uid: u32, gid: u32) -> Result<(), PosixErrno> {
    let normalized = normalize_path(path)?;
    if let Some(devfs) = devfs_context(fs_id) {
        sync_devfs_runtime_nodes(&devfs);
        return devfs
            .chown(&normalized, uid, gid, TaskId(0))
            .map_err(map_fs_error);
    }
    let contexts = FS_CONTEXTS.lock();
    let fs = contexts.get(&fs_id).ok_or(PosixErrno::BadFileDescriptor)?;
    fs.chown(&normalized, uid, gid, TaskId(0))
        .map_err(map_fs_error)
}

pub fn fchown(fd: u32, uid: u32, gid: u32) -> Result<(), PosixErrno> {
    let shared = {
        let table = FILE_TABLE.lock();
        table
            .get(&fd)
            .ok_or(PosixErrno::BadFileDescriptor)?
            .file
            .clone()
    };
    chown(shared.fs_id, &shared.path, uid, gid)
}

pub fn link(fs_id: u32, old_path: &str, new_path: &str) -> Result<(), PosixErrno> {
    let old_n = normalize_path(old_path)?;
    let new_n = normalize_path(new_path)?;
    let contexts = FS_CONTEXTS.lock();
    let fs = contexts.get(&fs_id).ok_or(PosixErrno::BadFileDescriptor)?;
    fs.link(&old_n, &new_n, TaskId(0)).map_err(map_fs_error)?;
    FILE_INDEX
        .lock()
        .entry(fs_id)
        .or_insert_with(BTreeSet::new)
        .insert(new_n);
    Ok(())
}

pub fn symlink(fs_id: u32, target: &str, link_path: &str) -> Result<(), PosixErrno> {
    let target_n = normalize_path(target)?;
    let link_n = normalize_path(link_path)?;
    let contexts = FS_CONTEXTS.lock();
    let fs = contexts.get(&fs_id).ok_or(PosixErrno::BadFileDescriptor)?;
    fs.symlink(&target_n, &link_n, TaskId(0))
        .map_err(map_fs_error)
}

pub fn readlink(fs_id: u32, path: &str) -> Result<String, PosixErrno> {
    let normalized = normalize_path(path)?;
    let contexts = FS_CONTEXTS.lock();
    let fs = contexts.get(&fs_id).ok_or(PosixErrno::BadFileDescriptor)?;
    fs.readlink(&normalized, TaskId(0)).map_err(map_fs_error)
}

pub fn utimensat(fs_id: u32, path: &str) -> Result<(), PosixErrno> {
    let now = time::monotonic_timespec();
    utimes(fs_id, path, now, now)
}

pub fn futimens(fd: u32, atime: PosixTimespec, mtime: PosixTimespec) -> Result<(), PosixErrno> {
    let table = FILE_TABLE.lock();
    let desc = table.get(&fd).ok_or(PosixErrno::BadFileDescriptor)?;
    utimes(desc.file.fs_id, &desc.file.path, atime, mtime)
}

pub fn utimes(
    fs_id: u32,
    path: &str,
    atime: PosixTimespec,
    mtime: PosixTimespec,
) -> Result<(), PosixErrno> {
    let normalized = normalize_path(path)?;
    let contexts = FS_CONTEXTS.lock();
    let fs = contexts.get(&fs_id).ok_or(PosixErrno::BadFileDescriptor)?;
    fs.set_times(&normalized, atime.sec as u64, mtime.sec as u64, TaskId(0))
        .map_err(map_fs_error)
}

pub fn futimes(fd: u32, atime: PosixTimespec, mtime: PosixTimespec) -> Result<(), PosixErrno> {
    futimens(fd, atime, mtime)
}

