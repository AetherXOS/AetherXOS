use super::helpers::linux_errno;

/// Semantic type alias for Linux syscall results.
/// Returns Ok(usize) for success or Err(usize) for a mapped Linux errno.
pub type LinuxResult = Result<usize, usize>;

/// Common Linux error mappings.
pub mod err {
    use super::*;

    pub fn inval() -> usize {
        linux_errno(crate::modules::posix_consts::errno::EINVAL)
    }
    pub fn fault() -> usize {
        linux_errno(crate::modules::posix_consts::errno::EFAULT)
    }
    pub fn perm() -> usize {
        linux_errno(crate::modules::posix_consts::errno::EPERM)
    }
    pub fn no_ent() -> usize {
        linux_errno(crate::modules::posix_consts::errno::ENOENT)
    }
    pub fn no_sys() -> usize {
        linux_errno(crate::modules::posix_consts::errno::ENOSYS)
    }
    pub fn exists() -> usize {
        linux_errno(crate::modules::posix_consts::errno::EEXIST)
    }
    pub fn is_dir() -> usize {
        linux_errno(crate::modules::posix_consts::errno::EISDIR)
    }
    pub fn not_dir() -> usize {
        linux_errno(crate::modules::posix_consts::errno::ENOTDIR)
    }
    pub fn bad_f() -> usize {
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
    pub fn rofs() -> usize {
        linux_errno(crate::modules::posix_consts::errno::EROFS)
    }
    pub fn timeout() -> usize {
        linux_errno(crate::modules::posix_consts::errno::ETIMEDOUT)
    }
    pub fn busy() -> usize {
        linux_errno(crate::modules::posix_consts::errno::EBUSY)
    }
    pub fn too_many_links() -> usize {
        linux_errno(crate::modules::posix_consts::errno::EMLINK)
    }
}
