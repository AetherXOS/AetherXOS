use super::*;

/// `F_DUPFD` semantics: duplicate `fd` to the smallest available FD >= `min_fd`.
pub fn dup_at_least(fd: u32, min_fd: u32) -> Result<u32, PosixErrno> {
    let file = {
        let table = FILE_TABLE.lock();
        let desc = table.get(&fd).ok_or(PosixErrno::BadFileDescriptor)?;
        desc.file.clone()
    };
    // Find the smallest fd >= min_fd not already in FILE_TABLE.
    let new_fd = {
        let table = FILE_TABLE.lock();
        let mut candidate = min_fd.max(1000); // keep below system reserved range
        while table.contains_key(&candidate) {
            candidate = candidate.saturating_add(1);
            if candidate == u32::MAX {
                return Err(PosixErrno::TooManyFiles);
            }
        }
        candidate
    };
    FILE_TABLE.lock().insert(
        new_fd,
        PosixFileDesc {
            file,
            cloexec: false,
        },
    );
    Ok(new_fd)
}

pub fn umask(new_mask: u16) -> u16 {
    UMASK_BITS.swap((new_mask & 0o777) as u32, Ordering::Relaxed) as u16
}