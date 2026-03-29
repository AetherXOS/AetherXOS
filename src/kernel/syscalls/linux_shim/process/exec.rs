use super::super::util::read_user_c_string;
#[cfg(feature = "posix_process")]
use super::super::util::read_user_c_string_array;
use super::super::*;
#[cfg(all(not(feature = "linux_compat"), feature = "posix_process"))]
mod env;
#[cfg(all(not(feature = "linux_compat"), feature = "posix_process"))]
use env::{
    append_exec_runtime_env, execve_aux_hwcap, execve_aux_platform, normalize_execve_argv,
    normalize_execve_envp,
};
#[cfg(any(
    all(
        not(feature = "linux_compat"),
        feature = "posix_process",
        feature = "process_abstraction",
        feature = "vfs",
        feature = "posix_mman"
    ),
    test
))]
use super::exec_stack::prepare_execve_user_stack as prepare_execve_user_stack_impl;
#[cfg(any(
    all(
        not(feature = "linux_compat"),
        feature = "posix_process",
        feature = "process_abstraction",
        feature = "vfs",
        feature = "posix_mman"
    ),
    test
))]
#[allow(unused_imports)]
use super::exec_stack::{ExecveAuxEntry, ExecveAuxValue};
#[cfg(all(not(feature = "linux_compat"), feature = "posix_process"))]
use core::sync::atomic::Ordering;

#[cfg(all(not(feature = "linux_compat"), feature = "posix_process"))]
fn validate_exec_entry_point(entry_val: usize) -> Result<(), usize> {
    if entry_val == 0 {
        return Err(linux_errno(crate::modules::posix_consts::errno::ENOEXEC));
    }
    Ok(())
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_process"))]
fn sanitized_phdr_aux_values(
    phdr_addr: usize,
    phent_size: usize,
    phnum: usize,
) -> Option<(usize, usize, usize)> {
    if phdr_addr == 0 || phent_size == 0 || phnum == 0 {
        return None;
    }
    if !(16..=4096).contains(&phent_size) {
        return None;
    }
    Some((phdr_addr, phent_size, phnum))
}

#[cfg(any(
    all(
        not(feature = "linux_compat"),
        feature = "posix_process",
        feature = "process_abstraction",
        feature = "vfs",
        feature = "posix_mman"
    ),
    test
))]
#[allow(dead_code)]
fn push_execve_user_word(sp: &mut u64, word_size: u64, value: usize) -> Result<(), usize> {
    let _efault = linux_errno(crate::modules::posix_consts::errno::EFAULT);
    super::exec_stack::push_execve_user_word(sp, word_size, value)
}

#[cfg(any(
    all(
        not(feature = "linux_compat"),
        feature = "posix_process",
        feature = "process_abstraction",
        feature = "vfs",
        feature = "posix_mman"
    ),
    test
))]
#[allow(dead_code)]
fn prepare_execve_user_stack(
    stack_start: u64,
    stack_size: u64,
    argv: &[alloc::string::String],
    envp: &[alloc::string::String],
    auxv_entries: &[ExecveAuxEntry<'_>],
) -> Result<u64, usize> {
    let _efault = linux_errno(crate::modules::posix_consts::errno::EFAULT);
    prepare_execve_user_stack_impl(stack_start, stack_size, argv, envp, auxv_entries)
}

#[cfg(not(feature = "linux_compat"))]
fn execve_with_path(path: alloc::string::String, argv_ptr: usize, envp_ptr: usize) -> usize {
    #[cfg(feature = "posix_process")]
    {
        let argv =
            match read_user_c_string_array(argv_ptr, EXECVE_MAX_VECTOR_ITEMS, USER_CSTRING_MAX_LEN)
            {
                Ok(v) => v,
                Err(e) => return e,
            };
        let envp =
            match read_user_c_string_array(envp_ptr, EXECVE_MAX_VECTOR_ITEMS, USER_CSTRING_MAX_LEN)
            {
                Ok(v) => v,
                Err(e) => return e,
            };
        let argv = normalize_execve_argv(&path, argv);
        let mut envp = normalize_execve_envp(&path, envp);

        let argv_refs: alloc::vec::Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
        let envp_refs: alloc::vec::Vec<&str> = envp.iter().map(|s| s.as_str()).collect();
        match crate::modules::posix::process::execve(&path, &argv_refs, &envp_refs) {
            Ok(()) => {
                #[cfg(feature = "process_abstraction")]
                {
                    if let Some(pid) = current_process_id() {
                        if let Some(proc) = crate::kernel::launch::process_arc_by_id(
                            crate::interfaces::task::ProcessId(pid),
                        ) {
                            append_exec_runtime_env(&mut envp, &proc);
                            #[cfg(all(feature = "vfs", feature = "posix_mman"))]
                            {
                                if let Ok(map_id) = crate::modules::posix::mman::mmap_anonymous(
                                    EXECVE_STACK_BYTES as usize,
                                    crate::modules::posix_consts::mman::PROT_READ
                                        | crate::modules::posix_consts::mman::PROT_WRITE,
                                    crate::modules::posix_consts::mman::MAP_PRIVATE,
                                ) {
                                    if let Ok(stack_start) =
                                        proc.allocate_user_vaddr(EXECVE_STACK_BYTES as usize)
                                    {
                                        let stack_end = stack_start + EXECVE_STACK_BYTES;
                                        let _ = proc.register_mapping(
                                            map_id,
                                            stack_start,
                                            stack_end,
                                            (crate::modules::posix_consts::mman::PROT_READ
                                                | crate::modules::posix_consts::mman::PROT_WRITE)
                                                as u32,
                                            crate::modules::posix_consts::mman::MAP_PRIVATE as u32,
                                        );
                                        let _ = proc.ensure_linux_runtime_mappings();
                                        let (
                                            entry_val,
                                            base_addr,
                                            phdr_addr,
                                            phent_size,
                                            phnum,
                                            vdso_base,
                                            _vvar_base,
                                        ) = proc.auxv_state();
                                        if let Err(errno) = validate_exec_entry_point(entry_val) {
                                            return errno;
                                        }
                                        let phdr_aux =
                                            sanitized_phdr_aux_values(phdr_addr, phent_size, phnum);
                                        let execfn = proc.exec_path_snapshot();
                                        let uid = crate::modules::posix::process::getuid() as usize;
                                        let euid =
                                            crate::modules::posix::process::geteuid() as usize;
                                        let gid = crate::modules::posix::process::getgid() as usize;
                                        let egid =
                                            crate::modules::posix::process::getegid() as usize;
                                        let secure = usize::from(uid != euid || gid != egid);
                                        let (hwcap, hwcap2) = execve_aux_hwcap();
                                        let platform = execve_aux_platform();
                                        let tsc = crate::hal::cpu::rdtsc();
                                        let mut at_random = [0u8; 16];
                                        at_random[..8].copy_from_slice(&tsc.to_le_bytes());
                                        at_random[8..].copy_from_slice(
                                            &(tsc.rotate_left(17)
                                                ^ entry_val as u64
                                                ^ base_addr as u64)
                                                .to_le_bytes(),
                                        );
                                        let mut auxv_entries = alloc::vec::Vec::with_capacity(19);
                                        auxv_entries.push(ExecveAuxEntry {
                                            key: EXECVE_AUXV_AT_ENTRY,
                                            value: ExecveAuxValue::Word(entry_val),
                                        });
                                        auxv_entries.push(ExecveAuxEntry {
                                            key: EXECVE_AUXV_AT_PAGESZ,
                                            value: ExecveAuxValue::Word(
                                                crate::interfaces::memory::PAGE_SIZE_4K as usize,
                                            ),
                                        });
                                        auxv_entries.push(ExecveAuxEntry {
                                            key: EXECVE_AUXV_AT_BASE,
                                            value: ExecveAuxValue::Word(base_addr),
                                        });
                                        auxv_entries.push(ExecveAuxEntry {
                                            key: EXECVE_AUXV_AT_FLAGS,
                                            value: ExecveAuxValue::Word(0),
                                        });
                                        auxv_entries.push(ExecveAuxEntry {
                                            key: EXECVE_AUXV_AT_UID,
                                            value: ExecveAuxValue::Word(uid),
                                        });
                                        auxv_entries.push(ExecveAuxEntry {
                                            key: EXECVE_AUXV_AT_EUID,
                                            value: ExecveAuxValue::Word(euid),
                                        });
                                        auxv_entries.push(ExecveAuxEntry {
                                            key: EXECVE_AUXV_AT_GID,
                                            value: ExecveAuxValue::Word(gid),
                                        });
                                        auxv_entries.push(ExecveAuxEntry {
                                            key: EXECVE_AUXV_AT_EGID,
                                            value: ExecveAuxValue::Word(egid),
                                        });
                                        auxv_entries.push(ExecveAuxEntry {
                                            key: EXECVE_AUXV_AT_SECURE,
                                            value: ExecveAuxValue::Word(secure),
                                        });
                                        auxv_entries.push(ExecveAuxEntry {
                                            key: EXECVE_AUXV_AT_HWCAP,
                                            value: ExecveAuxValue::Word(hwcap),
                                        });
                                        auxv_entries.push(ExecveAuxEntry {
                                            key: EXECVE_AUXV_AT_CLKTCK,
                                            value: ExecveAuxValue::Word(100),
                                        });
                                        auxv_entries.push(ExecveAuxEntry {
                                            key: EXECVE_AUXV_AT_PLATFORM,
                                            value: ExecveAuxValue::CString(platform),
                                        });
                                        auxv_entries.push(ExecveAuxEntry {
                                            key: EXECVE_AUXV_AT_RANDOM,
                                            value: ExecveAuxValue::Bytes(&at_random),
                                        });
                                        auxv_entries.push(ExecveAuxEntry {
                                            key: EXECVE_AUXV_AT_HWCAP2,
                                            value: ExecveAuxValue::Word(hwcap2),
                                        });
                                        if let Some((phdr_addr, phent_size, phnum)) = phdr_aux {
                                            auxv_entries.push(ExecveAuxEntry {
                                                key: EXECVE_AUXV_AT_PHDR,
                                                value: ExecveAuxValue::Word(phdr_addr),
                                            });
                                            auxv_entries.push(ExecveAuxEntry {
                                                key: EXECVE_AUXV_AT_PHENT,
                                                value: ExecveAuxValue::Word(phent_size),
                                            });
                                            auxv_entries.push(ExecveAuxEntry {
                                                key: EXECVE_AUXV_AT_PHNUM,
                                                value: ExecveAuxValue::Word(phnum),
                                            });
                                        }
                                        auxv_entries.push(ExecveAuxEntry {
                                            key: EXECVE_AUXV_AT_EXECFN,
                                            value: ExecveAuxValue::CString(execfn.as_str()),
                                        });
                                        auxv_entries.push(ExecveAuxEntry {
                                            key: EXECVE_AUXV_AT_SYSINFO_EHDR,
                                            value: ExecveAuxValue::Word(vdso_base),
                                        });
                                        let sp = match prepare_execve_user_stack(
                                            stack_start,
                                            EXECVE_STACK_BYTES,
                                            &argv,
                                            &envp,
                                            auxv_entries.as_slice(),
                                        ) {
                                            Ok(v) => v,
                                            Err(errno) => return errno,
                                        };
                                        let cpu_ptr = unsafe {
                                            crate::kernel::cpu_local::CpuLocal::get() as *const _
                                                as *mut crate::kernel::cpu_local::CpuLocal
                                        };
                                        unsafe {
                                            (*cpu_ptr).scratch = sp as u64;
                                        }
                                        if let Some(task_arc) = crate::kernel::task::get_task(
                                            crate::interfaces::task::TaskId(
                                                unsafe {
                                                    crate::kernel::cpu_local::CpuLocal::get()
                                                }
                                                .current_task
                                                .load(Ordering::Relaxed),
                                            ),
                                        ) {
                                            #[cfg(feature = "ring_protection")]
                                            {
                                                task_arc.lock().user_stack_pointer = sp;
                                            }
                                        }
                                    }
                                }
                            }
                            let entry = proc.effective_entry();
                            super::super::EXECVE_NEW_ENTRY.store(entry, Ordering::Relaxed);
                        }
                    }
                }
                0
            }
            Err(err) => linux_errno(err.code()),
        }
    }

    #[cfg(not(feature = "posix_process"))]
    {
        let _ = (path, argv_ptr, envp_ptr);
        linux_errno(crate::modules::posix_consts::errno::ENOENT)
    }
}

#[cfg(not(feature = "linux_compat"))]
fn resolve_execveat_path(dirfd: isize, path: &str, flags: usize) -> Result<alloc::string::String, usize> {
    const AT_EMPTY_PATH: usize = crate::kernel::syscalls::syscalls_consts::linux::AT_EMPTY_PATH;

    if path.is_empty() {
        if (flags & AT_EMPTY_PATH) == 0 {
            return Err(linux_errno(crate::modules::posix_consts::errno::ENOENT));
        }
        if dirfd < 0 {
            return Err(linux_errno(crate::modules::posix_consts::errno::EBADF));
        }

        #[cfg(feature = "posix_fs")]
        {
            return crate::modules::posix::fs::fd_path(dirfd as u32)
                .map_err(|err| linux_errno(err.code()));
        }

        #[cfg(not(feature = "posix_fs"))]
        {
            return Err(linux_errno(crate::modules::posix_consts::errno::ENOSYS));
        }
    }

    if path.starts_with('/') || dirfd == super::super::LINUX_AT_FDCWD {
        return Ok(path.into());
    }
    if dirfd < 0 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EBADF));
    }

    #[cfg(feature = "posix_fs")]
    {
        let fs_id = crate::modules::posix::fs::fd_fs_context(dirfd as u32)
            .map_err(|err| linux_errno(err.code()))?;
        let dir_path = crate::modules::posix::fs::fd_path(dirfd as u32)
            .map_err(|err| linux_errno(err.code()))?;
        return crate::modules::posix::fs::resolve_at_path(fs_id, &dir_path, path)
            .map_err(|err| linux_errno(err.code()));
    }

    #[cfg(not(feature = "posix_fs"))]
    {
        // Without fs context we cannot legally resolve dirfd-relative paths.
        Err(linux_errno(crate::modules::posix_consts::errno::ENOSYS))
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_execve(path_ptr: usize, argv_ptr: usize, envp_ptr: usize) -> usize {
    let path = match read_user_c_string(path_ptr, USER_CSTRING_MAX_LEN) {
        Ok(p) => p,
        Err(e) => return e,
    };
    execve_with_path(path, argv_ptr, envp_ptr)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_execveat(
    dirfd: isize,
    path_ptr: usize,
    argv_ptr: usize,
    envp_ptr: usize,
    flags: usize,
) -> usize {
    const AT_EMPTY_PATH: usize = crate::kernel::syscalls::syscalls_consts::linux::AT_EMPTY_PATH;
    const AT_SYMLINK_NOFOLLOW: usize =
        crate::kernel::syscalls::syscalls_consts::linux::AT_SYMLINK_NOFOLLOW;

    if (flags & !(AT_EMPTY_PATH | AT_SYMLINK_NOFOLLOW)) != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    let path = match read_user_c_string(path_ptr, USER_CSTRING_MAX_LEN) {
        Ok(p) => p,
        Err(e) => return e,
    };

    let resolved = match resolve_execveat_path(dirfd, &path, flags) {
        Ok(v) => v,
        Err(err) => return err,
    };

    execve_with_path(resolved, argv_ptr, envp_ptr)
}

#[cfg(all(test, not(feature = "linux_compat")))]
#[path = "exec/tests.rs"]
mod tests;
