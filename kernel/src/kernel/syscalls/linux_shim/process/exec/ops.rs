use core::sync::atomic::Ordering;
use super::super::super::util::{read_user_c_string, read_user_c_string_array};
use super::super::super::*;
use super::env::{append_exec_runtime_env, normalize_execve_argv, normalize_execve_envp};
use super::validation::{validate_exec_entry_point, validate_exec_handoff_contract};
use super::auxv::build_exec_auxv;
use super::path::resolve_execveat_path;
use super::super::exec_stack::prepare_execve_user_stack as prepare_execve_user_stack_impl;

#[cfg(not(feature = "linux_compat"))]
pub fn execve_with_path(path: alloc::string::String, argv_ptr: usize, envp_ptr: usize) -> usize {
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
                super::super::super::fd_process_identity::storage::linux_fd_close_on_exec();
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
                                            _image_base,
                                            phdr_addr,
                                            phent_size,
                                            phnum,
                                            vdso_base,
                                            _vvar_base,
                                            interp_base,
                                        ) = proc.auxv_state();
                                        if let Err(errno) = validate_exec_entry_point(entry_val as usize) {
                                            return errno;
                                        }
                                        if let Err(errno) = validate_exec_handoff_contract(
                                            entry_val as usize,
                                            _image_base as usize,
                                            phdr_addr as usize,
                                            phent_size as usize,
                                            phnum as usize,
                                        ) {
                                            return errno;
                                        }

                                        let execfn = proc.exec_path_snapshot();
                                        let tsc = crate::hal::cpu::rdtsc();
                                        let mut at_random = [0u8; 16];
                                        at_random[..8].copy_from_slice(&tsc.to_le_bytes());
                                        at_random[8..].copy_from_slice(
                                            &(tsc.rotate_left(17)
                                                ^ entry_val as u64
                                                ^ _image_base as u64)
                                                .to_le_bytes(),
                                        );

                                        let auxv_entries = build_exec_auxv(
                                            entry_val as usize,
                                            interp_base as usize,
                                            phdr_addr as usize,
                                            phent_size as usize,
                                            phnum as usize,
                                            vdso_base as usize,
                                            execfn.as_str(),
                                            &at_random,
                                        );


                                        let sp = match prepare_execve_user_stack_impl(
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
                                        super::super::super::EXECVE_NEW_ENTRY
                                            .store(entry as usize, Ordering::Relaxed);
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
pub fn sys_linux_execve(path_ptr: usize, argv_ptr: usize, envp_ptr: usize) -> usize {
    let path = match read_user_c_string(path_ptr, USER_CSTRING_MAX_LEN) {
        Ok(p) => p,
        Err(e) => return e,
    };
    execve_with_path(path, argv_ptr, envp_ptr)
}

#[cfg(not(feature = "linux_compat"))]
pub fn sys_linux_execveat(
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
