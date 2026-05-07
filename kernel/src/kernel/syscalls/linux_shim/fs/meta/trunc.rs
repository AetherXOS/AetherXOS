use super::super::super::*;

#[cfg(not(feature = "linux_compat"))]
pub fn sys_linux_ftruncate(fd: usize, length: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        match crate::modules::posix::fs::ftruncate(fd as u32, length) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (fd, length);
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}
