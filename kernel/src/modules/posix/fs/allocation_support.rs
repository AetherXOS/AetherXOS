use super::*;

pub fn posix_fallocate(fd: u32, len: usize) -> Result<(), PosixErrno> {
    posix_fallocate_range(fd, 0, len)
}

pub fn posix_fallocate_range(fd: u32, offset: usize, len: usize) -> Result<(), PosixErrno> {
    let target_len = offset.checked_add(len).ok_or(PosixErrno::Invalid)?;
    let shared = {
        let table = FILE_TABLE.lock();
        table
            .get(&fd)
            .ok_or(PosixErrno::BadFileDescriptor)?
            .file
            .clone()
    };
    let contexts = FS_CONTEXTS.lock();
    let fs = contexts
        .get(&shared.fs_id)
        .ok_or(PosixErrno::BadFileDescriptor)?;
    let mut data = fs.read_all(&shared.path).unwrap_or_default();
    if data.len() < target_len {
        data.resize(target_len, 0);
        let _ = fs.write_all(&shared.path, &data).map_err(map_fs_error)?;
    }
    Ok(())
}

fn punch_hole_range(fd: u32, offset: usize, len: usize) -> Result<(), PosixErrno> {
    let end = offset.checked_add(len).ok_or(PosixErrno::Invalid)?;

    let table = FILE_TABLE.lock();
    let desc = table.get(&fd).ok_or(PosixErrno::BadFileDescriptor)?;
    let contexts = FS_CONTEXTS.lock();
    let fs = contexts
        .get(&desc.file.fs_id)
        .ok_or(PosixErrno::BadFileDescriptor)?;

    let mut data = fs.read_all(&desc.file.path).unwrap_or_default();
    if offset >= data.len() {
        return Ok(());
    }

    let bounded_end = core::cmp::min(end, data.len());
    data[offset..bounded_end].fill(0);
    let _ = fs.write_all(&desc.file.path, &data).map_err(map_fs_error)?;
    Ok(())
}

pub fn fallocate(fd: u32, mode: u32, offset: usize, len: usize) -> Result<(), PosixErrno> {
    let keep_size = crate::modules::posix_consts::fs::FALLOC_FL_KEEP_SIZE;
    let punch_hole = crate::modules::posix_consts::fs::FALLOC_FL_PUNCH_HOLE;
    let allowed = keep_size | punch_hole;
    if (mode & !allowed) != 0 {
        return Err(PosixErrno::Invalid);
    }

    if (mode & punch_hole) != 0 {
        if (mode & keep_size) == 0 {
            return Err(PosixErrno::Invalid);
        }
        return punch_hole_range(fd, offset, len);
    }

    if (mode & keep_size) != 0 {
        return Ok(());
    }

    posix_fallocate_range(fd, offset, len)
}

pub fn syncfs(fs_id: u32) -> Result<(), PosixErrno> {
    if FS_CONTEXTS.lock().contains_key(&fs_id) {
        Ok(())
    } else {
        Err(PosixErrno::BadFileDescriptor)
    }
}

pub fn fd_fs_context(fd: u32) -> Result<u32, PosixErrno> {
    let table = FILE_TABLE.lock();
    let desc = table.get(&fd).ok_or(PosixErrno::BadFileDescriptor)?;
    Ok(desc.file.fs_id)
}

pub fn fd_path(fd: u32) -> Result<String, PosixErrno> {
    let table = FILE_TABLE.lock();
    let desc = table.get(&fd).ok_or(PosixErrno::BadFileDescriptor)?;
    Ok(desc.file.path.clone())
}