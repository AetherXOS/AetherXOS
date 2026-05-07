pub mod perms;
pub mod stat;
pub mod statfs;
pub mod sync;
pub mod times;
pub mod trunc;

pub use perms::*;
pub use stat::*;
pub use statfs::*;
pub use sync::*;
pub use times::*;
pub use trunc::*;

#[cfg(not(feature = "linux_compat"))]
mod statx;

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_statx(
    dirfd: usize,
    pathname_ptr: usize,
    flags: usize,
    mask: usize,
    statxbuf_ptr: usize,
) -> usize {
    statx::sys_linux_statx(dirfd, pathname_ptr, flags, mask, statxbuf_ptr)
}

#[cfg(all(test, not(feature = "linux_compat")))]
#[path = "meta/tests.rs"]
mod tests;
