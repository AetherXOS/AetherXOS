use super::super::*;
use crate::kernel::syscalls::{
    current_process_id, execve_stack_required_bytes, prepare_execve_user_stack,
    set_execve_new_entry, set_execve_new_stack, ExecveAuxEntry, ExecveAuxValue, SyscallFrame,
};
use core::sync::atomic::Ordering;

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

                                // --- Linux Stack Preparation (Required for glibc/Ubuntu) ---
                                let stack_bytes = 0x800000; // 8MB stack for Linux
                                if let Ok(map_id) = crate::modules::posix::mman::mmap_anonymous(
                                    stack_bytes,
                                    crate::modules::posix_consts::mman::PROT_READ
                                        | crate::modules::posix_consts::mman::PROT_WRITE,
                                    crate::modules::posix_consts::mman::MAP_PRIVATE,
                                ) {
                                    if let Ok(stack_start) = proc.allocate_user_vaddr(stack_bytes) {
                                        let stack_end = stack_start + stack_bytes as u64;
                                        let _ = proc.register_mapping(
                                            map_id,
                                            stack_start,
                                            stack_end,
                                            (crate::modules::posix_consts::mman::PROT_READ
                                                | crate::modules::posix_consts::mman::PROT_WRITE)
                                                as u32,
                                            crate::modules::posix_consts::mman::MAP_PRIVATE as u32,
                                        );

                                        // Populate Linux auxiliary vectors
                                        let _ = proc.ensure_linux_runtime_mappings();
                                        let (
                                            entry_val,
                                            base_addr,
                                            phdr_addr,
                                            phent_size,
                                            phnum,
                                            vdso_base,
                                            _vvar_base,
                                            _interpreter_base,
                                        ) = proc.auxv_state();

                                        let mut random_bytes = [0u8; 16];
                                        // Simple entropy for now, should use hal::get_random_bytes if available
                                        let tsc = crate::hal::cpu::rdtsc();
                                        let _hhdm = crate::hal::hhdm_offset().unwrap_or(0);
                                        random_bytes[..8].copy_from_slice(&tsc.to_le_bytes());
                                        random_bytes[8..].copy_from_slice(
                                            &(tsc.rotate_left(13) ^ (entry_val as u64)).to_le_bytes(),
                                        );
                                        let random_ptr = stack_end - 16;

                                        use crate::kernel::syscalls::syscalls_consts::linux::*;
                                        let auxv_entries = alloc::vec![
                                            ExecveAuxEntry {
                                                key: linux::AT_PHDR as usize,
                                                value: ExecveAuxValue::Word(phdr_addr as usize),
                                            },
                                            ExecveAuxEntry {
                                                key: linux::AT_PHENT as usize,
                                                value: ExecveAuxValue::Word(phent_size as usize),
                                            },
                                            ExecveAuxEntry {
                                                key: linux::AT_PHNUM as usize,
                                                value: ExecveAuxValue::Word(phnum as usize),
                                            },
                                            ExecveAuxEntry {
                                                key: linux::AT_PAGESZ as usize,
                                                value: ExecveAuxValue::Word(4096),
                                            },
                                            ExecveAuxEntry {
                                                key: linux::AT_BASE as usize,
                                                value: ExecveAuxValue::Word(base_addr as usize),
                                            },
                                            ExecveAuxEntry {
                                                key: linux::AT_ENTRY as usize,
                                                value: ExecveAuxValue::Word(entry_val as usize),
                                            },
                                            ExecveAuxEntry {
                                                key: linux::AT_UID as usize,
                                                value: ExecveAuxValue::Word(0),
                                            },
                                            ExecveAuxEntry {
                                                key: linux::AT_EUID as usize,
                                                value: ExecveAuxValue::Word(0),
                                            },
                                            ExecveAuxEntry {
                                                key: linux::AT_GID as usize,
                                                value: ExecveAuxValue::Word(0),
                                            },
                                            ExecveAuxEntry {
                                                key: linux::AT_EGID as usize,
                                                value: ExecveAuxValue::Word(0),
                                            },
                                            ExecveAuxEntry {
                                                key: linux::AT_SECURE as usize,
                                                value: ExecveAuxValue::Word(0),
                                            },
                                            ExecveAuxEntry {
                                                key: linux::AT_RANDOM as usize,
                                                value: ExecveAuxValue::Word(random_ptr as usize),
                                            },
                                            ExecveAuxEntry {
                                                key: linux::AT_HWCAP as usize,
                                                value: ExecveAuxValue::Word(0),
                                            },
                                            ExecveAuxEntry {
                                                key: linux::AT_CLKTCK as usize,
                                                value: ExecveAuxValue::Word(100),
                                            },
                                            ExecveAuxEntry {
                                                key: linux::AT_SYSINFO_EHDR as usize,
                                                value: ExecveAuxValue::Word(vdso_base as usize),
                                            },
                                        ];

                                        if let Ok(new_rsp) = prepare_execve_user_stack(
                                            stack_start,
                                            stack_bytes as u64,
                                            &argv,
                                            &envp,
                                            &auxv_entries,
                                        ) {
                                            set_execve_new_stack(new_rsp as usize);
                                            
                                            // Update task user stack pointer for scheduling
                                            if let Some(cpu) = unsafe { crate::kernel::cpu_local::CpuLocal::try_get() } {
                                                let tid = cpu.current_task.load(Ordering::Relaxed);
                                                if let Some(task_arc) = crate::kernel::task::get_task(crate::interfaces::task::TaskId(tid)) {
                                                    #[cfg(feature = "ring_protection")]
                                                    {
                                                        task_arc.lock().user_stack_pointer = new_rsp;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                let entry = proc.image_entry.load(Ordering::Relaxed);
                                frame.rip = entry as u64;
                                set_execve_new_entry(entry);
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
            use crate::modules::vfs::types::{FileStats, VfsTimespec};
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::linux_compat::helpers::{linux_fault, linux_inval};

    #[test_case]
    fn resolve_execveat_null_ptr_returns_fault() {
        // pathname_ptr == 0 should return EFAULT
        let res = resolve_linux_execveat_path(Fd(0), 0, 0);
        assert_eq!(res, Err(linux_fault()));
    }

    #[test_case]
    fn resolve_execveat_invalid_flags_returns_inval() {
        // invalid flags (bits outside allowed mask) should return EINVAL
        let invalid_flags = !0usize; // set high bits to trigger invalid flag path
        let res = resolve_linux_execveat_path(Fd(0), 0, invalid_flags);
        assert_eq!(res, Err(linux_inval()));
    }
}
