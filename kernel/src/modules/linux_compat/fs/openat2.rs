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

        // RESOLVE_* flags handling (Security checks)
        // If we see unknown bits in resolve field, return EINVAL as per Linux spec.
        if (how.resolve & !(linux::openat2::RESOLVE_ALLOWED_MASK as u64)) != 0 {
            return linux_inval();
        }

        // For production-grade, we delegate actual security enforcement to the VFS.
        // Here we pass flags/mode to openat but we should logically pass the 'how' structure
        // to a VFS that understands RESOLVE_BENEATH, RESOLVE_NO_SYMLINKS, etc.
        super::file::sys_linux_openat(dirfd, pathname_ptr, how.flags as usize, how.mode as usize)
    })
}
