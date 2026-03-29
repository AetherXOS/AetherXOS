use super::*;

pub fn fcntl_get_status_flags(fd: u32) -> Result<u32, PosixErrno> {
    let table = FILE_TABLE.lock();
    let desc = table.get(&fd).ok_or(PosixErrno::BadFileDescriptor)?;
    let flags = *desc.file.flags.lock();
    Ok(flags)
}

pub fn fcntl_get_descriptor_flags(fd: u32) -> Result<u32, PosixErrno> {
    let table = FILE_TABLE.lock();
    let desc = table.get(&fd).ok_or(PosixErrno::BadFileDescriptor)?;
    Ok(if desc.cloexec {
        POSIX_DESCRIPTOR_CLOEXEC
    } else {
        0
    })
}

pub fn fcntl_set_descriptor_flags(fd: u32, flags: u32) -> Result<(), PosixErrno> {
    let mut table = FILE_TABLE.lock();
    let desc = table.get_mut(&fd).ok_or(PosixErrno::BadFileDescriptor)?;
    desc.cloexec = (flags & POSIX_DESCRIPTOR_CLOEXEC) != 0;
    Ok(())
}

pub fn fcntl_set_status_flags(fd: u32, flags: u32) -> Result<(), PosixErrno> {
    let table = FILE_TABLE.lock();
    let desc = table.get(&fd).ok_or(PosixErrno::BadFileDescriptor)?;
    let masked = flags & POSIX_SUPPORTED_STATUS_FLAGS;
    *desc.file.flags.lock() = masked;
    #[cfg(feature = "posix_pipe")]
    {
        let nonblock = (masked & (crate::modules::posix_consts::net::O_NONBLOCK as u32)) != 0;
        let _ = crate::modules::posix::pipe::set_nonblock(fd, nonblock);
    }
    #[cfg(feature = "posix_io")]
    {
        let nonblock = (masked & (crate::modules::posix_consts::net::O_NONBLOCK as u32)) != 0;
        let _ = crate::modules::posix::io::eventfd_set_nonblock(fd, nonblock);
    }
    #[cfg(feature = "posix_signal")]
    {
        let nonblock = (masked & (crate::modules::posix_consts::net::O_NONBLOCK as u32)) != 0;
        let _ = crate::modules::posix::signal::signalfd_set_nonblock(fd, nonblock);
    }
    Ok(())
}

pub fn get_file_description(fd: u32) -> Result<Arc<SharedFile>, PosixErrno> {
    let table = FILE_TABLE.lock();
    Ok(table
        .get(&fd)
        .ok_or(PosixErrno::BadFileDescriptor)?
        .file
        .clone())
}

pub fn register_file_description(file: Arc<SharedFile>) -> u32 {
    let fd = NEXT_FD.fetch_add(1, Ordering::Relaxed);
    FILE_TABLE.lock().insert(
        fd,
        PosixFileDesc {
            file,
            cloexec: false,
        },
    );
    fd
}

pub fn ioctl(fd: u32, cmd: u32, arg: u64) -> Result<isize, PosixErrno> {
    let shared = {
        let table = FILE_TABLE.lock();
        table
            .get(&fd)
            .ok_or(PosixErrno::BadFileDescriptor)?
            .file
            .clone()
    };
    let res = shared.handle.lock().ioctl(cmd, arg).map_err(map_fs_error);
    res
}
