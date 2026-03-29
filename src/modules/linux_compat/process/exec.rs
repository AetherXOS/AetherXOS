use super::super::*;
use crate::kernel::syscalls::{current_process_id, SyscallFrame};

pub(crate) fn execve_with_path(
    frame: &mut SyscallFrame,
    path: alloc::string::String,
    argv_ptr: usize,
    envp_ptr: usize,
) -> usize {
    let _ = frame;
    // read argv/envp arrays
    let max_path = crate::config::KernelConfig::vfs_max_mount_path();
    let argv = match read_user_string_vec(argv_ptr, max_path) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let envp = match read_user_string_vec(envp_ptr, max_path) {
        Ok(v) => v,
        Err(e) => return e,
    };

    crate::require_posix_process!((path, argv, envp) => {
        if let Some(pid) = current_process_id() {
                    if let Some(proc) = crate::kernel::launch::process_arc_by_id(crate::interfaces::task::ProcessId(pid)) {
                        // perform the posix execve call
                        let argv_refs: alloc::vec::Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
                        let envp_refs: alloc::vec::Vec<&str> = envp.iter().map(|s| s.as_str()).collect();
                        match crate::modules::posix::process::execve(&path, &argv_refs, &envp_refs) {
                            Ok(()) => {
                                let closed = crate::modules::linux_compat::fs::close_cloexec_descriptors();
                                if closed != 0 {
                                    crate::klog_info!("execve: closed {} CLOEXEC descriptors", closed);
                                }
                                let entry = proc.image_entry.load(core::sync::atomic::Ordering::Relaxed);
                                frame.rip = entry as u64;
                                0
                            }
                            Err(err) => linux_errno(err.code()),
                        }
                    } else {
                        linux_esrch()
                    }
                } else {
                    linux_esrch()
                }
    })
}

pub(crate) fn resolve_linux_execveat_path(
    dirfd: Fd,
    pathname_ptr: usize,
    flags: usize,
) -> Result<alloc::string::String, usize> {
    let allowed_flags = linux::AT_EMPTY_PATH | linux::AT_SYMLINK_NOFOLLOW;
    if (flags & !allowed_flags) != 0 {
        return Err(linux_inval());
    }
    if pathname_ptr == 0 {
        return Err(linux_fault());
    }

    let max_path = crate::config::KernelConfig::vfs_max_mount_path();
    let path = read_user_c_string(pathname_ptr, max_path)?;

    if path.is_empty() {
        if (flags & linux::AT_EMPTY_PATH) == 0 {
            return Err(linux_errno(crate::modules::posix_consts::errno::ENOENT));
        }
        if dirfd.0 < 0 {
            return Err(linux_errno(crate::modules::posix_consts::errno::EBADF));
        }
        #[cfg(feature = "posix_fs")]
        {
            return crate::modules::posix::fs::fd_path(dirfd.as_u32())
                .map_err(|e| linux_errno(e.code()));
        }
        #[cfg(not(feature = "posix_fs"))]
        {
            return Err(linux_errno(crate::modules::posix_consts::errno::ENOENT));
        }
    }

    if path.starts_with('/') || dirfd.0 == linux::AT_FDCWD as i32 {
        return Ok(path);
    }
    if dirfd.0 < 0 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EBADF));
    }

    #[cfg(feature = "posix_fs")]
    {
        let fs_id = crate::modules::posix::fs::fd_fs_context(dirfd.as_u32())
            .map_err(|e| linux_errno(e.code()))?;
        let dir_path = crate::modules::posix::fs::fd_path(dirfd.as_u32())
            .map_err(|e| linux_errno(e.code()))?;
        crate::modules::posix::fs::resolve_at_path(fs_id, &dir_path, &path)
            .map_err(|e| linux_errno(e.code()))
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        Err(linux_errno(crate::modules::posix_consts::errno::ENOENT))
    }
}

pub fn sys_linux_execve(
    frame: &mut SyscallFrame,
    path_ptr: usize,
    argv_ptr: usize,
    envp_ptr: usize,
) -> usize {
    let path = match read_user_c_string(path_ptr, crate::config::KernelConfig::vfs_max_mount_path())
    {
        Ok(p) => p,
        Err(e) => return e,
    };
    execve_with_path(frame, path, argv_ptr, envp_ptr)
}
