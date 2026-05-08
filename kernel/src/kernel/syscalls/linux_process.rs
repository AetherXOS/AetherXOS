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

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_brk(new_brk: usize) -> usize {
    if let Some(proc) = crate::kernel::launch::current_process_arc() {
        let current_brk = proc.heap_break.load(core::sync::atomic::Ordering::Relaxed);
        
        // PHASE 6 TASK 7: Enforce memory quotas via syscall integration
        // Use a simplified PID (0 for now as placeholder)
        let result = crate::kernel_runtime::syscall_integration::on_brk_syscall(
            0,  // pid placeholder
            new_brk as u64,
            current_brk,
        );
        
        // If quota check passed, update the brk
        if result == new_brk as u64 {
            match proc.set_brk(new_brk as u64) {
                Ok(v) => v as usize,
                Err(_) => {
                    // If it fails, Linux brk returns the CURRENT break
                    proc.heap_break.load(core::sync::atomic::Ordering::Relaxed) as usize
                }
            }
        } else {
            // Quota rejected or error - return current break
            result as usize
        }
    } else {
        new_brk // Should not happen in a valid process
    }
}

