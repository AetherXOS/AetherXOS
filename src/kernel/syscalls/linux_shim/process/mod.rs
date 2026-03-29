mod clone_ns;
mod exec;
mod exec_stack;

#[cfg(all(test, not(feature = "linux_compat")))]
pub(crate) use exec_stack::{execve_stack_required_bytes, prepare_execve_user_stack};

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_execve(path_ptr: usize, argv_ptr: usize, envp_ptr: usize) -> usize {
    exec::sys_linux_execve(path_ptr, argv_ptr, envp_ptr)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_execveat(
    dirfd: isize,
    path_ptr: usize,
    argv_ptr: usize,
    envp_ptr: usize,
    flags: usize,
) -> usize {
    exec::sys_linux_execveat(dirfd, path_ptr, argv_ptr, envp_ptr, flags)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_fork() -> usize {
    clone_ns::sys_linux_fork()
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_clone(
    flags: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
    arg6: usize,
) -> usize {
    clone_ns::sys_linux_clone(flags, arg2, arg3, arg4, arg5, arg6)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_clone3(args_ptr: usize, size: usize) -> usize {
    clone_ns::sys_linux_clone3(args_ptr, size)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_unshare(flags: usize) -> usize {
    clone_ns::sys_linux_unshare(flags)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_setns(fd: usize, nstype: usize) -> usize {
    clone_ns::sys_linux_setns(fd, nstype)
}
