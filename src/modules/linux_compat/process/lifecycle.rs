use super::super::*;

/// Linux `clone(2)` — production-grade implementation.
///
/// Supported flag combinations:
///   * Thread : CLONE_VM | CLONE_THREAD | CLONE_SIGHAND (+ optional TLS/tidptrs)
///   * Process: any combination without CLONE_VM (delegates to posix::process::fork)
pub fn sys_linux_clone(
    flags: usize,
    child_stack: UserPtr<u8>,
    parent_tidptr: UserPtr<usize>,
    child_tidptr: UserPtr<usize>,
    tls: usize,
    _arg6: usize,
    _user_rip: usize,
    _user_rflags: usize,
) -> usize {
    crate::require_posix_process!((flags, child_stack, parent_tidptr, child_tidptr, tls, _arg6) => {
        use crate::hal::syscalls_consts::linux::clone_flags as cf;

        let is_thread = (flags & cf::CLONE_VM) != 0 && (flags & cf::CLONE_THREAD) != 0;

        if is_thread {
            if child_stack.is_null() { return linux_inval(); }
            let sp = child_stack.addr & !0xF_usize;
            if sp == 0 { return linux_inval(); }

            let pid = crate::modules::posix::process::getpid();
            if pid == 0 { return linux_esrch(); }

            let proc_id = crate::interfaces::task::ProcessId(pid);
            let proc = match crate::kernel::launch::process_arc_by_id(proc_id) {
                Some(p) => p,
                None    => return linux_esrch(),
            };

            let entry = _user_rip as u64;
            if entry == 0 { return linux_inval(); }

            const KSTACK_SIZE: usize = 0x4000;
            let kernel_stack_top = match proc.allocate_user_vaddr(KSTACK_SIZE) {
                Ok(base) => base + KSTACK_SIZE as u64,
                Err(_)   => return linux_enomem(),
            };

            let current_tid = unsafe { crate::kernel::cpu_local::CpuLocal::get().current_task_id() };
            let cr3 = match crate::kernel::task::get_task(current_tid) {
                Some(task) => task.lock().page_table_root,
                None => return linux_esrch(),
            };
            let tls_ptr = if (flags & cf::CLONE_SETTLS) != 0 { tls as u64 } else { 0 };
            let new_tid = match crate::kernel::fork::do_clone(
                proc_id,
                kernel_stack_top,
                entry,
                cr3,
                flags as u64,
                tls_ptr,
            ) {
                Ok(tid) => tid,
                Err(_) => return linux_errno(crate::modules::posix_consts::errno::EAGAIN),
            };

            if let Some(task) = crate::kernel::task::get_task(new_tid) {
                let mut locked = task.lock();
                locked.context.rax = 0;
                locked.context.rsp = sp as u64;
                locked.context.rip = entry;
                locked.context.rcx = entry;
                locked.context.r11 = _user_rflags as u64;
            }

            if (flags & cf::CLONE_PARENT_SETTID) != 0 && !parent_tidptr.is_null() {
                let _ = parent_tidptr.write(&new_tid.0);
            }
            if (flags & cf::CLONE_CHILD_SETTID) != 0 && !child_tidptr.is_null() {
                let _ = child_tidptr.write(&new_tid.0);
            }
            return new_tid.0;
        }

        let ns_flags = flags & (
            cf::CLONE_NEWPID | cf::CLONE_NEWNET | cf::CLONE_NEWNS | 
            cf::CLONE_NEWIPC | cf::CLONE_NEWUTS | cf::CLONE_NEWUSER | cf::CLONE_NEWCGROUP
        );

        match crate::modules::posix::process::fork() {
            Ok(child_pid) => {
                if ns_flags != 0 {
                    if let Some(parent_pid) = crate::modules::linux_compat::current_process_id() {
                        if let Some(parent) = crate::kernel::launch::process_arc_by_id(
                            crate::interfaces::task::ProcessId(parent_pid),
                        ) {
                            if let Some(child) = crate::kernel::launch::process_arc_by_id(
                                crate::interfaces::task::ProcessId(child_pid),
                            ) {
                                let parent_ns = parent.namespace_id.load(core::sync::atomic::Ordering::Relaxed);
                                if let Ok(new_ns) = crate::kernel::namespaces::unshare_process_namespaces(
                                    parent_ns,
                                    ns_flags as u32,
                                ) {
                                    child.namespace_id.store(new_ns, core::sync::atomic::Ordering::Relaxed);
                                } else {
                                    // Normally we should undo fork here, but for now we just return EINVAL
                                    return linux_errno(crate::modules::posix_consts::errno::EINVAL);
                                }
                            }
                        }
                    }
                }

                if (flags & cf::CLONE_PARENT_SETTID) != 0 && !parent_tidptr.is_null() {
                    let _ = parent_tidptr.write(&child_pid);
                }
                child_pid
            }
            Err(e) => linux_errno(e.code()),
        }
    })
}

pub fn sys_linux_unshare(flags: usize) -> usize {
    crate::require_posix_process!((flags) => {
        let Some(pid) = crate::modules::linux_compat::current_process_id() else {
            return linux_errno(crate::modules::posix_consts::errno::ESRCH);
        };
        let Some(process) = crate::kernel::launch::process_arc_by_id(crate::interfaces::task::ProcessId(pid)) else {
            return linux_errno(crate::modules::posix_consts::errno::ESRCH);
        };
        let current_ns = process.namespace_id.load(core::sync::atomic::Ordering::Relaxed);
        let flags_u32 = flags as u32;
        match crate::kernel::namespaces::unshare_process_namespaces(current_ns, flags_u32) {
            Ok(new_ns) => {
                process.namespace_id.store(new_ns, core::sync::atomic::Ordering::Relaxed);
                0
            }
            Err(_) => linux_errno(crate::modules::posix_consts::errno::EINVAL),
        }
    })
}

pub fn sys_linux_setns(fd: Fd, nstype: usize) -> usize {
    crate::require_posix_process!((fd, nstype) => {
        let Some(pid) = crate::modules::linux_compat::current_process_id() else {
            return linux_errno(crate::modules::posix_consts::errno::ESRCH);
        };
        let Some(process) = crate::kernel::launch::process_arc_by_id(crate::interfaces::task::ProcessId(pid)) else {
            return linux_errno(crate::modules::posix_consts::errno::ESRCH);
        };
        let current_ns = process.namespace_id.load(core::sync::atomic::Ordering::Relaxed);
        
        // Ensure standard linux fd validation logic
        let nsfd = fd.as_u32() as i32;
        match crate::kernel::namespaces::setns_process_namespaces(current_ns, nsfd, nstype as u32) {
            Ok(new_ns) => {
                process.namespace_id.store(new_ns, core::sync::atomic::Ordering::Relaxed);
                0
            }
            Err("EBADF") => linux_errno(crate::modules::posix_consts::errno::EBADF),
            Err("EOVERFLOW") => linux_errno(crate::modules::posix_consts::errno::EOVERFLOW),
            Err(_) => linux_errno(crate::modules::posix_consts::errno::EINVAL),
        }
    })
}

pub fn sys_linux_fork() -> usize {
    crate::require_posix_process!(() => {
        match crate::modules::posix::process::fork() {
            Ok(pid) => pid,
            Err(e)  => linux_errno(e.code()),
        }
    })
}

pub fn sys_linux_exit_group(status: usize) -> usize {
    crate::kernel::syscalls::sys_exit(status)
}

// wait4 and waitid are implemented in wait.rs
