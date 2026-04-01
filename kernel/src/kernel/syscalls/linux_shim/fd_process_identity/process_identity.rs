#[cfg(feature = "posix_process")]
use super::super::linux_errno;
#[cfg(not(feature = "posix_process"))]
use super::super::task_time::sys_linux_getpid;

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_getuid() -> usize {
    #[cfg(feature = "posix_process")]
    {
        crate::modules::posix::process::getuid() as usize
    }
    #[cfg(not(feature = "posix_process"))]
    {
        0
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_getgid() -> usize {
    #[cfg(feature = "posix_process")]
    {
        crate::modules::posix::process::getgid() as usize
    }
    #[cfg(not(feature = "posix_process"))]
    {
        0
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_geteuid() -> usize {
    #[cfg(feature = "posix_process")]
    {
        crate::modules::posix::process::geteuid() as usize
    }
    #[cfg(not(feature = "posix_process"))]
    {
        0
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_getegid() -> usize {
    #[cfg(feature = "posix_process")]
    {
        crate::modules::posix::process::getegid() as usize
    }
    #[cfg(not(feature = "posix_process"))]
    {
        0
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_getpgrp() -> usize {
    #[cfg(feature = "posix_process")]
    {
        crate::modules::posix::process::getpgrp()
    }
    #[cfg(not(feature = "posix_process"))]
    {
        sys_linux_getpid()
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_getpgid(pid: usize) -> usize {
    #[cfg(feature = "posix_process")]
    {
        match crate::modules::posix::process::getpgid(pid) {
            Ok(v) => v,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_process"))]
    {
        let _ = pid;
        sys_linux_getpid()
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_setpgid(pid: usize, pgid: usize) -> usize {
    #[cfg(feature = "posix_process")]
    {
        match crate::modules::posix::process::setpgid(pid, pgid) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_process"))]
    {
        let _ = (pid, pgid);
        0
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_setsid() -> usize {
    #[cfg(feature = "posix_process")]
    {
        match crate::modules::posix::process::setsid() {
            Ok(v) => v,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_process"))]
    {
        sys_linux_getpid()
    }
}
