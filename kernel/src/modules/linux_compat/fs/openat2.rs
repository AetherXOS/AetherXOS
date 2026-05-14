use super::super::*;

/// `openat2(2)` — Modern and more secure file opening.
pub fn sys_linux_openat2(
    dirfd: Fd,
    pathname_ptr: UserPtr<u8>,
    how_ptr: UserPtr<LinuxOpenHow>,
    size: usize,
) -> usize {
    crate::require_posix_fs!((dirfd, pathname_ptr, how_ptr, size) => {
        if size < core::mem::size_of::<LinuxOpenHow>() { return linux_inval(); }
        let how = match how_ptr.read() { Ok(v) => v, Err(e) => return e };

        if let Err(e) = super::file::decode_linux_open_intent(how.flags as usize) {
            return e;
        }

        let (fs_id, dir_path, path) = resolve_at!(dirfd, pathname_ptr);
        
        // RESOLVE_* flags handling (Security checks)
        let rflags = crate::modules::vfs::types::ResolveFlags::from_bits_truncate(how.resolve as u32);
        
        match crate::modules::posix::fs::openat2(fs_id, &dir_path, &path, (how.flags as usize & 0o100) != 0, rflags) {
            Ok(fd) => {
                if (how.flags as usize & 0o1000) != 0 { // O_TRUNC
                    let _ = crate::modules::posix::fs::ftruncate(fd, 0);
                }
                super::file::apply_linux_open_post_flags(fd, how.flags as usize);
                fd as usize
            }
            Err(err) => linux_errno(err.code()),
        }
    })
}
