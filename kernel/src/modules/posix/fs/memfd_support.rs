use super::*;
use crate::modules::vfs::ramfs::AnonymousRamFile;
use alloc::sync::Arc;
use spin::Mutex;

pub fn memfd_create(name: &str, flags: u32) -> Result<u32, PosixErrno> {
    let fs_id = *SHM_FS_ID;
    if fs_id == 0 {
        return Err(PosixErrno::BadFileDescriptor);
    }
    
    // Create an anonymous RamFile
    let ram_file = Arc::new(Mutex::new(alloc::vec::Vec::new()));
    let file = alloc::boxed::Box::new(AnonymousRamFile::new(ram_file));
    
    // In a real memfd, we would handle SEALING flags here.
    // For now, we register it as a regular file in the FD table.
    let fd = io_support::register_handle(
        fs_id,
        alloc::format!("memfd:{}", name),
        Arc::new(Mutex::new(file_types_support::BoxedFile { inner: file })),
        true,
    );
    
    // Apply flags like MFD_CLOEXEC
    if (flags & 0x01) != 0 {
        let _ = fd_support::fcntl_set_descriptor_flags(fd, 0x01);
    }
    
    Ok(fd)
}
