use super::*;
use core::any::Any;
use core::sync::atomic::Ordering;

// The process-level file table (Process::files) uses IrqSafeMutex which does not implement Clone.
// The POSIX compat layer uses the global FILE_TABLE as its authoritative FD registry.
// This function always returns None so all paths use FILE_TABLE.
#[allow(dead_code)]
fn get_process_file_table() -> Option<Arc<IrqSafeMutex<alloc::collections::BTreeMap<u32, Arc<dyn Any + Send + Sync>>>>> {
    None
}

pub fn fcntl_get_status_flags(fd: u32) -> Result<u32, PosixErrno> {
    let desc = if let Some(table) = get_process_file_table() {
        let lock = table.lock();
        lock.get(&fd).and_then(|f| f.clone().downcast::<PosixFileDesc>().ok()).map(|arc| (*arc).clone())
    } else {
        FILE_TABLE.lock().get(&fd).cloned()
    }.ok_or(PosixErrno::BadFileDescriptor)?;
    
    let flags = desc.file.flags.load(Ordering::Acquire);
    Ok(flags)
}

pub fn fcntl_get_descriptor_flags(fd: u32) -> Result<u32, PosixErrno> {
    let desc = if let Some(table) = get_process_file_table() {
        let lock = table.lock();
        lock.get(&fd).and_then(|f| f.clone().downcast::<PosixFileDesc>().ok()).map(|arc| (*arc).clone())
    } else {
        FILE_TABLE.lock().get(&fd).cloned()
    }.ok_or(PosixErrno::BadFileDescriptor)?;
    
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
    desc.file.flags.store(masked, Ordering::Relaxed);
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
    let desc = if let Some(table) = get_process_file_table() {
        let lock = table.lock();
        lock.get(&fd).and_then(|f| f.clone().downcast::<PosixFileDesc>().ok()).map(|arc| (*arc).clone())
    } else {
        FILE_TABLE.lock().get(&fd).cloned()
    }.ok_or(PosixErrno::BadFileDescriptor)?;

    Ok(desc.file.clone())
}

pub fn register_file_description(file: Arc<SharedFile>) -> u32 {
    let fd = NEXT_FD.fetch_add(1, Ordering::Relaxed);
    let desc = PosixFileDesc {
        file,
        cloexec: false,
    };
    
    if let Some(table) = get_process_file_table() {
        table.lock().insert(fd, Arc::new(desc));
    } else {
        FILE_TABLE.lock().insert(fd, desc);
    }
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
