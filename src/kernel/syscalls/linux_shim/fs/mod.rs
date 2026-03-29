mod io;
mod meta;
mod support;

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_access(path_ptr: usize, mode: usize) -> usize {
    io::sys_linux_access(path_ptr, mode)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_close(fd: usize) -> usize {
    io::sys_linux_close(fd)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_faccessat(
    dirfd: isize,
    path_ptr: usize,
    mode: usize,
    flags: usize,
) -> usize {
    io::sys_linux_faccessat(dirfd, path_ptr, mode, flags)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_faccessat2(
    dirfd: isize,
    path_ptr: usize,
    mode: usize,
    flags: usize,
) -> usize {
    io::sys_linux_faccessat2(dirfd, path_ptr, mode, flags)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_lseek(fd: usize, offset: i64, whence_raw: usize) -> usize {
    io::sys_linux_lseek(fd, offset, whence_raw)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_openat(
    dirfd: isize,
    pathname_ptr: usize,
    flags: usize,
    mode: usize,
) -> usize {
    io::sys_linux_openat(dirfd, pathname_ptr, flags, mode)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_openat2(
    dirfd: isize,
    pathname_ptr: usize,
    how_ptr: usize,
    size: usize,
) -> usize {
    io::sys_linux_openat2(dirfd, pathname_ptr, how_ptr, size)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_mkdirat(dirfd: isize, pathname_ptr: usize, mode: usize) -> usize {
    io::sys_linux_mkdirat(dirfd, pathname_ptr, mode)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_unlinkat(dirfd: isize, pathname_ptr: usize, flags: usize) -> usize {
    io::sys_linux_unlinkat(dirfd, pathname_ptr, flags)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_linkat(
    olddirfd: isize,
    oldpath_ptr: usize,
    newdirfd: isize,
    newpath_ptr: usize,
    flags: usize,
) -> usize {
    io::sys_linux_linkat(olddirfd, oldpath_ptr, newdirfd, newpath_ptr, flags)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_symlinkat(target_ptr: usize, newdirfd: isize, linkpath_ptr: usize) -> usize {
    io::sys_linux_symlinkat(target_ptr, newdirfd, linkpath_ptr)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_renameat(
    olddirfd: isize,
    oldpath_ptr: usize,
    newdirfd: isize,
    newpath_ptr: usize,
) -> usize {
    io::sys_linux_renameat(olddirfd, oldpath_ptr, newdirfd, newpath_ptr)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_renameat2(
    olddirfd: isize,
    oldpath_ptr: usize,
    newdirfd: isize,
    newpath_ptr: usize,
    flags: usize,
) -> usize {
    io::sys_linux_renameat2(olddirfd, oldpath_ptr, newdirfd, newpath_ptr, flags)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_readlinkat(
    dirfd: isize,
    pathname_ptr: usize,
    buf_ptr: usize,
    buf_size: usize,
) -> usize {
    io::sys_linux_readlinkat(dirfd, pathname_ptr, buf_ptr, buf_size)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_read(fd: usize, ptr: usize, len: usize) -> usize {
    io::sys_linux_read(fd, ptr, len)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_write(fd: usize, ptr: usize, len: usize) -> usize {
    io::sys_linux_write(fd, ptr, len)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_fdatasync(fd: usize) -> usize {
    meta::sys_linux_fdatasync(fd)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_fstat(fd: usize, buf_ptr: usize) -> usize {
    meta::sys_linux_fstat(fd, buf_ptr)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_fsync(fd: usize) -> usize {
    meta::sys_linux_fsync(fd)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_ftruncate(fd: usize, length: usize) -> usize {
    meta::sys_linux_ftruncate(fd, length)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_newfstatat(
    dirfd: usize,
    pathname_ptr: usize,
    buf_ptr: usize,
    flags: usize,
) -> usize {
    meta::sys_linux_newfstatat(dirfd, pathname_ptr, buf_ptr, flags)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_statx(
    dirfd: usize,
    pathname_ptr: usize,
    flags: usize,
    mask: usize,
    statxbuf_ptr: usize,
) -> usize {
    meta::sys_linux_statx(dirfd, pathname_ptr, flags, mask, statxbuf_ptr)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_sync() -> usize {
    meta::sys_linux_sync()
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_syncfs(fd: usize) -> usize {
    meta::sys_linux_syncfs(fd)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_chmod(path_ptr: usize, mode: usize) -> usize {
    meta::sys_linux_chmod(path_ptr, mode)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_fchmod(fd: usize, mode: usize) -> usize {
    meta::sys_linux_fchmod(fd, mode)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_chown(path_ptr: usize, uid: usize, gid: usize) -> usize {
    meta::sys_linux_chown(path_ptr, uid, gid)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_fchown(fd: usize, uid: usize, gid: usize) -> usize {
    meta::sys_linux_fchown(fd, uid, gid)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_fchmodat(
    dirfd: usize,
    path_ptr: usize,
    mode: usize,
    flags: usize,
) -> usize {
    meta::sys_linux_fchmodat(dirfd, path_ptr, mode, flags)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_fchownat(
    dirfd: usize,
    path_ptr: usize,
    uid: usize,
    gid: usize,
    flags: usize,
) -> usize {
    meta::sys_linux_fchownat(dirfd, path_ptr, uid, gid, flags)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_statfs(path_ptr: usize, buf_ptr: usize) -> usize {
    meta::sys_linux_statfs(path_ptr, buf_ptr)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_fstatfs(fd: usize, buf_ptr: usize) -> usize {
    meta::sys_linux_fstatfs(fd, buf_ptr)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_utimensat(
    dirfd: usize,
    pathname_ptr: usize,
    times_ptr: usize,
    flags: usize,
) -> usize {
    meta::sys_linux_utimensat(dirfd, pathname_ptr, times_ptr, flags)
}
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_futimesat(dirfd: usize, pathname_ptr: usize, times_ptr: usize) -> usize {
    meta::sys_linux_futimesat(dirfd, pathname_ptr, times_ptr)
}

#[cfg(all(test, not(feature = "linux_compat")))]
#[path = "integration_tests.rs"]
mod integration_tests;
