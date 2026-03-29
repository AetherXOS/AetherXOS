use super::*;
use super::file_types_support::BoxedFile;

pub fn register_posix_handle(
    handle: Arc<Mutex<dyn crate::modules::vfs::File>>,
) -> Result<u32, PosixErrno> {
    Ok(register_handle(0, String::from("socket:"), handle, true))
}

pub fn register_handle(
    fs_id: u32,
    path: String,
    handle: Arc<Mutex<dyn crate::modules::vfs::File>>,
    can_write: bool,
) -> u32 {
    let shared = Arc::new(SharedFile {
        fs_id,
        path: path.clone(),
        handle,
        offset: Mutex::new(0),
        flags: Mutex::new(if can_write { 0x2 } else { 0x0 }), // Simplified O_RDWR
    });

    let fd = NEXT_FD.fetch_add(1, Ordering::Relaxed);
    FILE_TABLE.lock().insert(
        fd,
        PosixFileDesc {
            file: shared,
            cloexec: false,
        },
    );
    if fs_id != 0 {
        FILE_INDEX
            .lock()
            .entry(fs_id)
            .or_insert_with(BTreeSet::new)
            .insert(path);
    }
    fd
}

pub fn dup(oldfd: u32) -> Result<u32, PosixErrno> {
    let mut table = FILE_TABLE.lock();
    let desc = table
        .get(&oldfd)
        .ok_or(PosixErrno::BadFileDescriptor)?
        .clone();
    let newfd = NEXT_FD.fetch_add(1, Ordering::Relaxed);
    table.insert(
        newfd,
        PosixFileDesc {
            file: desc.file,
            cloexec: false,
        },
    );
    Ok(newfd)
}

pub fn dup2(oldfd: u32, newfd: u32) -> Result<u32, PosixErrno> {
    let mut table = FILE_TABLE.lock();
    let desc = table
        .get(&oldfd)
        .ok_or(PosixErrno::BadFileDescriptor)?
        .clone();
    if oldfd == newfd {
        return Ok(newfd);
    }
    table.insert(
        newfd,
        PosixFileDesc {
            file: desc.file,
            cloexec: false,
        },
    );
    Ok(newfd)
}

pub fn open(fs_id: u32, path: &str, create: bool) -> Result<u32, PosixErrno> {
    let normalized = normalize_path(path)?;
    let tid = crate::interfaces::TaskId(crate::modules::posix::process::gettid());
    if let Some(devfs) = devfs_context(fs_id) {
        sync_devfs_runtime_nodes(&devfs);
        if create {
            return Err(PosixErrno::NotSupported);
        }
        let handle = devfs.open(&normalized, tid).map_err(map_fs_error)?;
        return Ok(register_handle(
            fs_id,
            normalized,
            Arc::new(Mutex::new(BoxedFile { inner: handle })),
            true,
        ));
    }
    let contexts = FS_CONTEXTS.lock();
    let fs = contexts.get(&fs_id).ok_or(PosixErrno::BadFileDescriptor)?;

    let handle = if create {
        fs.create(&normalized, tid).map_err(map_fs_error)?
    } else {
        fs.open(&normalized, tid).map_err(map_fs_error)?
    };

    Ok(register_handle(
        fs_id,
        normalized,
        Arc::new(Mutex::new(BoxedFile { inner: handle })),
        true,
    ))
}

pub fn creat(fs_id: u32, path: &str, mode: u16) -> Result<u32, PosixErrno> {
    let _ = mode;
    open(fs_id, path, true)
}

pub fn close(fd: u32) -> Result<(), PosixErrno> {
    let removed = FILE_TABLE.lock().remove(&fd);
    if removed.is_some() {
        Ok(())
    } else {
        Err(PosixErrno::BadFileDescriptor)
    }
}

pub fn read(fd: u32, buf: &mut [u8]) -> Result<usize, PosixErrno> {
    let shared = {
        let table = FILE_TABLE.lock();
        table
            .get(&fd)
            .ok_or(PosixErrno::BadFileDescriptor)?
            .file
            .clone()
    };

    let mut offset = shared.offset.lock();
    let mut handle = shared.handle.lock();
    let old_vfs_off = handle
        .seek(crate::modules::vfs::SeekFrom::Current(0))
        .unwrap_or(0);
    let _ = handle.seek(crate::modules::vfs::SeekFrom::Start(*offset));

    let n = handle.read(buf).map_err(map_fs_error)?;
    *offset += n as u64;

    let _ = handle.seek(crate::modules::vfs::SeekFrom::Start(old_vfs_off));
    Ok(n)
}

pub fn pread(fd: u32, buf: &mut [u8], offset: u64) -> Result<usize, PosixErrno> {
    let shared = {
        let table = FILE_TABLE.lock();
        table
            .get(&fd)
            .ok_or(PosixErrno::BadFileDescriptor)?
            .file
            .clone()
    };

    let mut handle = shared.handle.lock();
    let old = handle
        .seek(crate::modules::vfs::SeekFrom::Current(0))
        .unwrap_or(0);
    handle
        .seek(crate::modules::vfs::SeekFrom::Start(offset))
        .map_err(map_fs_error)?;
    let n = handle.read(buf).map_err(map_fs_error)?;
    let _ = handle.seek(crate::modules::vfs::SeekFrom::Start(old));
    Ok(n)
}

pub fn write(fd: u32, buf: &[u8]) -> Result<usize, PosixErrno> {
    let shared = {
        let table = FILE_TABLE.lock();
        table
            .get(&fd)
            .ok_or(PosixErrno::BadFileDescriptor)?
            .file
            .clone()
    };

    let mut offset = shared.offset.lock();
    let flags = *shared.flags.lock();
    let mut handle = shared.handle.lock();
    let old_vfs_off = handle
        .seek(crate::modules::vfs::SeekFrom::Current(0))
        .unwrap_or(0);
    if (flags & crate::modules::posix_consts::fs::O_APPEND as u32) != 0 {
        if let Ok(end) = handle.seek(crate::modules::vfs::SeekFrom::End(0)) {
            *offset = end;
        }
    } else {
        let _ = handle.seek(crate::modules::vfs::SeekFrom::Start(*offset));
    }

    let n = handle.write(buf).map_err(map_fs_error)?;
    *offset += n as u64;

    let _ = handle.seek(crate::modules::vfs::SeekFrom::Start(old_vfs_off));
    Ok(n)
}

pub fn pwrite(fd: u32, buf: &[u8], offset: u64) -> Result<usize, PosixErrno> {
    let shared = {
        let table = FILE_TABLE.lock();
        table
            .get(&fd)
            .ok_or(PosixErrno::BadFileDescriptor)?
            .file
            .clone()
    };

    let mut handle = shared.handle.lock();
    let old = handle
        .seek(crate::modules::vfs::SeekFrom::Current(0))
        .unwrap_or(0);
    handle
        .seek(crate::modules::vfs::SeekFrom::Start(offset))
        .map_err(map_fs_error)?;
    let n = handle.write(buf).map_err(map_fs_error)?;
    let _ = handle.seek(crate::modules::vfs::SeekFrom::Start(old));
    Ok(n)
}

pub fn readv(
    fd: u32,
    iovs: &mut [crate::modules::vfs::types::IoVecMut],
) -> Result<usize, PosixErrno> {
    let shared = {
        let table = FILE_TABLE.lock();
        table
            .get(&fd)
            .ok_or(PosixErrno::BadFileDescriptor)?
            .file
            .clone()
    };
    let res = shared
        .handle
        .lock()
        .read_vectored(iovs)
        .map_err(map_fs_error);
    res
}

pub fn writev(fd: u32, iovs: &[crate::modules::vfs::types::IoVec]) -> Result<usize, PosixErrno> {
    let shared = {
        let table = FILE_TABLE.lock();
        table
            .get(&fd)
            .ok_or(PosixErrno::BadFileDescriptor)?
            .file
            .clone()
    };
    let res = shared
        .handle
        .lock()
        .write_vectored(iovs)
        .map_err(map_fs_error);
    res
}

pub fn lseek(fd: u32, offset: i64, whence: SeekWhence) -> Result<u64, PosixErrno> {
    let shared = {
        let table = FILE_TABLE.lock();
        table
            .get(&fd)
            .ok_or(PosixErrno::BadFileDescriptor)?
            .file
            .clone()
    };

    let mut off = shared.offset.lock();
    let vfs_whence = match whence {
        SeekWhence::Set => crate::modules::vfs::SeekFrom::Start(offset as u64),
        SeekWhence::Cur => crate::modules::vfs::SeekFrom::Current(offset + (*off as i64)),
        SeekWhence::End => crate::modules::vfs::SeekFrom::End(offset),
    };

    let res = shared
        .handle
        .lock()
        .seek(vfs_whence)
        .map_err(map_fs_error)?;
    *off = res;
    Ok(res)
}

pub fn fsync(fd: u32) -> Result<(), PosixErrno> {
    let shared = {
        let table = FILE_TABLE.lock();
        table
            .get(&fd)
            .ok_or(PosixErrno::BadFileDescriptor)?
            .file
            .clone()
    };
    let res = shared.handle.lock().flush().map_err(map_fs_error);
    res
}

pub fn fdatasync(fd: u32) -> Result<(), PosixErrno> {
    fsync(fd)
}

