use super::SyscallFrame;

pub(super) fn dispatch_linux_syscall(
    syscall_id: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
    arg6: usize,
    _user_rip: usize,
    _user_rflags: usize,
    frame_ptr: *mut SyscallFrame,
) -> Option<usize> {
    #[cfg(feature = "linux_compat")]
    {
        return dispatch_linux_syscall_linux_compat(
            syscall_id,
            arg1,
            arg2,
            arg3,
            arg4,
            arg5,
            arg6,
            _user_rip,
            _user_rflags,
            frame_ptr,
        );
    }

    #[cfg(not(feature = "linux_compat"))]
    {
        return dispatch_linux_syscall_shim(
            syscall_id, arg1, arg2, arg3, arg4, arg5, arg6, frame_ptr,
        );
    }
}

#[cfg(feature = "linux_compat")]
fn dispatch_linux_syscall_linux_compat(
    syscall_id: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
    arg6: usize,
    user_rip: usize,
    user_rflags: usize,
    frame_ptr: *mut SyscallFrame,
) -> Option<usize> {
    let _ = (user_rip, user_rflags);

    // Safe because frame_ptr is provided by the assembly handler on the current stack.
    let frame = unsafe { &mut *frame_ptr };

    if let Some(ret) = crate::modules::linux_compat::sys_dispatcher::sys_linux_compat(
        syscall_id, arg1, arg2, arg3, arg4, arg5, arg6, frame,
    ) {
        return Some(ret);
    }
    Some(crate::modules::linux_compat::linux_nosys())
}

#[cfg(not(feature = "linux_compat"))]
fn dispatch_linux_syscall_shim(
    syscall_id: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
    arg6: usize,
    frame_ptr: *mut SyscallFrame,
) -> Option<usize> {
    super::linux_shim::sys_linux_shim(syscall_id, arg1, arg2, arg3, arg4, arg5, arg6, frame_ptr)
}
