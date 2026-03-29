mod rlimit;
mod wait;
mod wait_support;

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_getrlimit(resource: usize, rlim_ptr: usize) -> usize {
    rlimit::sys_linux_getrlimit(resource, rlim_ptr)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_setrlimit(resource: usize, rlim_ptr: usize) -> usize {
    rlimit::sys_linux_setrlimit(resource, rlim_ptr)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_prlimit64(
    pid: usize,
    resource: usize,
    new_rlim_ptr: usize,
    old_rlim_ptr: usize,
) -> usize {
    rlimit::sys_linux_prlimit64(pid, resource, new_rlim_ptr, old_rlim_ptr)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_wait4(
    pid: isize,
    wstatus_ptr: usize,
    options: usize,
    _rusage_ptr: usize,
) -> usize {
    wait::sys_linux_wait4(pid, wstatus_ptr, options, _rusage_ptr)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_waitid(idtype: usize, id: usize, infop: usize, options: usize) -> usize {
    wait::sys_linux_waitid(idtype, id, infop, options)
}
