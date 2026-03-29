use super::super::*;
use core::sync::atomic::Ordering;

#[cfg(any(feature = "posix_process", test))]
fn namespace_clone_flags(flags: usize) -> usize {
    use crate::kernel::syscalls::syscalls_consts::linux::clone_flags as cf;

    flags
        & (cf::CLONE_NEWPID
            | cf::CLONE_NEWNET
            | cf::CLONE_NEWNS
            | cf::CLONE_NEWIPC
            | cf::CLONE_NEWUTS
            | cf::CLONE_NEWUSER
            | cf::CLONE_NEWCGROUP)
}

#[cfg(any(feature = "posix_process", test))]
fn validate_clone_flags(flags: usize) -> Result<usize, usize> {
    use crate::kernel::syscalls::syscalls_consts::linux::clone_flags as cf;

    if (flags & (cf::CLONE_THREAD | cf::CLONE_VM)) != 0 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }

    Ok(namespace_clone_flags(flags))
}

#[cfg(any(feature = "process_abstraction", test))]
fn decode_unshare_flags(flags: usize) -> Result<u32, usize> {
    u32::try_from(flags).map_err(|_| linux_errno(crate::modules::posix_consts::errno::EINVAL))
}

#[cfg(any(feature = "process_abstraction", test))]
fn decode_setns_fd(fd: usize) -> Result<i32, usize> {
    i32::try_from(fd).map_err(|_| linux_errno(crate::modules::posix_consts::errno::EBADF))
}

#[cfg(any(feature = "process_abstraction", test))]
fn decode_setns_type(nstype: usize) -> Result<u32, usize> {
    u32::try_from(nstype).map_err(|_| linux_errno(crate::modules::posix_consts::errno::EINVAL))
}

#[cfg(feature = "posix_process")]
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_fork() -> usize {
    match crate::modules::posix::process::fork() {
        Ok(pid) => pid,
        Err(e) => linux_errno(e.code()),
    }
}

#[cfg(not(feature = "posix_process"))]
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_fork() -> usize {
    linux_errno(crate::modules::posix_consts::errno::EAGAIN)
}

#[cfg(feature = "posix_process")]
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_clone(
    flags: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
    arg6: usize,
) -> usize {
    let _newsp = arg2;
    let parent_tidptr = arg3;
    let _child_tidptr = arg4;
    let _tls = arg5;
    let _ = arg6;

    let ns_flags = match validate_clone_flags(flags) {
        Ok(v) => v,
        Err(err) => return err,
    };

    match crate::modules::posix::process::fork() {
        Ok(child_pid) => {
            if ns_flags != 0 {
                if let Some(parent_pid) = current_process_id() {
                    if let Some(parent) = crate::kernel::launch::process_arc_by_id(
                        crate::interfaces::task::ProcessId(parent_pid),
                    ) {
                        if let Some(child) = crate::kernel::launch::process_arc_by_id(
                            crate::interfaces::task::ProcessId(child_pid),
                        ) {
                            let parent_ns = parent.namespace_id.load(Ordering::Relaxed);
                            match crate::kernel::namespaces::unshare_process_namespaces(
                                parent_ns,
                                ns_flags as u32,
                            ) {
                                Ok(new_ns) => child.namespace_id.store(new_ns, Ordering::Relaxed),
                                Err(_) => {
                                    return linux_errno(
                                        crate::modules::posix_consts::errno::EINVAL,
                                    );
                                }
                            }
                        }
                    }
                }
            }

            if parent_tidptr != 0 {
                let _ = with_user_write_words(
                    parent_tidptr,
                    core::mem::size_of::<usize>(),
                    1,
                    |words| {
                        words[0] = child_pid;
                    },
                );
            }
            child_pid
        }
        Err(e) => linux_errno(e.code()),
    }
}

#[cfg(not(feature = "posix_process"))]
#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_clone(
    flags: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
    arg6: usize,
) -> usize {
    let _ = (flags, arg2, arg3, arg4, arg5, arg6);
    linux_errno(crate::modules::posix_consts::errno::EAGAIN)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_unshare(flags: usize) -> usize {
    #[cfg(feature = "process_abstraction")]
    {
        let Some(pid) = current_process_id() else {
            return linux_errno(crate::modules::posix_consts::errno::ESRCH);
        };

        let Some(process) =
            crate::kernel::launch::process_arc_by_id(crate::interfaces::task::ProcessId(pid))
        else {
            return linux_errno(crate::modules::posix_consts::errno::ESRCH);
        };

        let current_ns = process.namespace_id.load(Ordering::Relaxed);
        let flags_u32 = match decode_unshare_flags(flags) {
            Ok(v) => v,
            Err(err) => return err,
        };

        match crate::kernel::namespaces::unshare_process_namespaces(current_ns, flags_u32) {
            Ok(new_ns) => {
                process.namespace_id.store(new_ns, Ordering::Relaxed);
                0
            }
            Err("EINVAL") => linux_errno(crate::modules::posix_consts::errno::EINVAL),
            Err("EOVERFLOW") => linux_errno(crate::modules::posix_consts::errno::EOVERFLOW),
            Err(_) => linux_errno(crate::modules::posix_consts::errno::EINVAL),
        }
    }

    #[cfg(not(feature = "process_abstraction"))]
    {
        let _ = flags;
        linux_errno(crate::modules::posix_consts::errno::EINVAL)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_setns(fd: usize, nstype: usize) -> usize {
    #[cfg(feature = "process_abstraction")]
    {
        let Some(pid) = current_process_id() else {
            return linux_errno(crate::modules::posix_consts::errno::ESRCH);
        };
        let Some(process) =
            crate::kernel::launch::process_arc_by_id(crate::interfaces::task::ProcessId(pid))
        else {
            return linux_errno(crate::modules::posix_consts::errno::ESRCH);
        };

        let nsfd = match decode_setns_fd(fd) {
            Ok(v) => v,
            Err(err) => return err,
        };

        let nstype = match decode_setns_type(nstype) {
            Ok(v) => v,
            Err(err) => return err,
        };

        let current_ns = process.namespace_id.load(Ordering::Relaxed);
        match crate::kernel::namespaces::setns_process_namespaces(current_ns, nsfd, nstype) {
            Ok(new_ns) => {
                process.namespace_id.store(new_ns, Ordering::Relaxed);
                0
            }
            Err("EBADF") => linux_errno(crate::modules::posix_consts::errno::EBADF),
            Err("EOVERFLOW") => linux_errno(crate::modules::posix_consts::errno::EOVERFLOW),
            Err(_) => linux_errno(crate::modules::posix_consts::errno::EINVAL),
        }
    }

    #[cfg(not(feature = "process_abstraction"))]
    {
        let _ = (fd, nstype);
        linux_errno(crate::modules::posix_consts::errno::EINVAL)
    }
}

#[cfg(all(test, not(feature = "linux_compat")))]
mod tests {
    use super::*;
    use crate::kernel::syscalls::syscalls_consts::linux::clone_flags as cf;

    #[test_case]
    fn clone_rejects_thread_or_vm_flags() {
        assert_eq!(
            validate_clone_flags(cf::CLONE_THREAD).unwrap_err(),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
        assert_eq!(
            validate_clone_flags(cf::CLONE_VM).unwrap_err(),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }

    #[test_case]
    fn clone_namespace_mask_keeps_supported_bits_only() {
        let flags = cf::CLONE_NEWPID | cf::CLONE_NEWUSER | cf::CLONE_FILES;
        assert_eq!(
            validate_clone_flags(flags).unwrap(),
            cf::CLONE_NEWPID | cf::CLONE_NEWUSER
        );
    }

    #[test_case]
    fn clone_namespace_mask_returns_zero_for_non_namespace_flags() {
        assert_eq!(namespace_clone_flags(cf::CLONE_FILES | cf::CLONE_FS), 0);
        assert_eq!(validate_clone_flags(cf::CLONE_FILES).unwrap(), 0);
    }

    #[test_case]
    fn unshare_rejects_flags_that_overflow_u32() {
        assert_eq!(
            decode_unshare_flags(usize::MAX).unwrap_err(),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }

    #[test_case]
    fn setns_argument_decoders_reject_overflow() {
        assert_eq!(
            decode_setns_fd(usize::MAX).unwrap_err(),
            linux_errno(crate::modules::posix_consts::errno::EBADF)
        );
        assert_eq!(
            decode_setns_type(usize::MAX).unwrap_err(),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }

    #[test_case]
    fn setns_argument_decoders_accept_i32_u32_boundaries() {
        assert_eq!(decode_setns_fd(i32::MAX as usize).unwrap(), i32::MAX);
        assert_eq!(decode_setns_type(u32::MAX as usize).unwrap(), u32::MAX);
    }

    #[test_case]
    fn namespace_clone_flags_ignore_non_namespace_bits() {
        let flags = cf::CLONE_NEWNET | cf::CLONE_NEWUTS | cf::CLONE_FS | cf::CLONE_FILES;
        assert_eq!(
            namespace_clone_flags(flags),
            cf::CLONE_NEWNET | cf::CLONE_NEWUTS
        );
    }

    #[test_case]
    fn decode_unshare_flags_accepts_valid_u32_boundary() {
        assert_eq!(decode_unshare_flags(u32::MAX as usize).unwrap(), u32::MAX);
    }

    #[test_case]
    fn validate_clone_flags_accepts_namespace_only_requests() {
        let flags = cf::CLONE_NEWPID | cf::CLONE_NEWNS | cf::CLONE_NEWCGROUP;
        assert_eq!(
            validate_clone_flags(flags).unwrap(),
            cf::CLONE_NEWPID | cf::CLONE_NEWNS | cf::CLONE_NEWCGROUP
        );
    }

    #[test_case]
    fn validate_clone_flags_rejects_mixed_thread_and_namespace_requests() {
        assert_eq!(
            validate_clone_flags(cf::CLONE_THREAD | cf::CLONE_NEWNET).unwrap_err(),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }
}
