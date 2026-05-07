use crate::kernel::task;

/// Handle 'clone' and 'fork' syscalls.
pub fn sys_linux_clone(flags: u64, stack: u64, ptid: u64, ctid: u64, tls: u64) -> isize {
    crate::klog_info!("[SYSCALL] clone: flags={:#x}", flags);
    0
}

/// Handle 'clone3' syscall.
pub fn sys_linux_clone3(cl_args: u64, size: usize) -> isize {
    0
}

/// Handle 'execve' syscall.
pub fn sys_linux_execve(filename: u64, argv: u64, envp: u64) -> isize {
    0
}

/// Handle 'exit' and 'exit_group' syscalls.
pub fn sys_linux_exit(status: i32) -> ! {
    crate::kernel::task::scheduling::exit_current_task(status as u64);
}

/// Handle 'wait4' syscall.
pub fn sys_linux_wait4(pid: isize, status_ptr: u64, options: i32, rusage: u64) -> isize {
    0
}
