use super::*;
#[cfg(feature = "vfs")]
use super::basename_bytes;

#[cfg(all(feature = "vfs", feature = "posix_fs"))]
pub(super) fn set_exec_fs(fs_id: u32) {
    *EXEC_FS_ID.lock() = Some(fs_id);
}

#[cfg(all(feature = "vfs", feature = "posix_fs"))]
fn read_exec_image_from_fs(fs_id: u32, path: &str) -> Result<alloc::vec::Vec<u8>, PosixErrno> {
    let fd = crate::modules::posix::fs::open(fs_id, path, false)?;
    let mut out = alloc::vec::Vec::new();
    let mut chunk = [0u8; 512];

    loop {
        let n = crate::modules::posix::fs::read(fd, &mut chunk)?;
        if n == 0 {
            break;
        }
        out.extend_from_slice(&chunk[..n]);
    }

    let _ = crate::modules::posix::fs::close(fd);
    if out.is_empty() {
        return Err(PosixErrno::Invalid);
    }
    Ok(out)
}

#[cfg(all(feature = "vfs", feature = "posix_fs"))]
#[allow(dead_code)]
fn read_exec_image(path: &str) -> Result<alloc::vec::Vec<u8>, PosixErrno> {
    let fs_id = (*EXEC_FS_ID.lock()).ok_or(PosixErrno::BadFileDescriptor)?;
    read_exec_image_from_fs(fs_id, path)
}

#[cfg(all(feature = "vfs", feature = "posix_fs"))]
fn validate_interp_for_image(image: &[u8], fs_id: u32) -> Result<(), PosixErrno> {
    if let Some(interp_path) = resolve_interp_path(image)? {
        crate::klog_info!("exec: detected PT_INTERP '{}', validating interpreter", interp_path);
        let interp_image = read_exec_image_from_fs(fs_id, &interp_path)?;
        crate::kernel::module_loader::inspect_elf_image(&interp_image)
            .map_err(|_| PosixErrno::Invalid)?;
        crate::klog_info!("exec: PT_INTERP '{}' validated", interp_path);
    }
    Ok(())
}

#[cfg(all(feature = "vfs", feature = "posix_fs"))]
fn read_exec_image_with_validated_interp(path: &str) -> Result<alloc::vec::Vec<u8>, PosixErrno> {
    let fs_id = (*EXEC_FS_ID.lock()).ok_or(PosixErrno::BadFileDescriptor)?;
    let image = read_exec_image_from_fs(fs_id, path)?;
    // Keep the original executable image as exec target and validate PT_INTERP separately.
    validate_interp_for_image(&image, fs_id)?;
    Ok(image)
}

#[cfg(all(feature = "vfs", feature = "posix_fs"))]
fn read_exec_image_with_validated_interp_from_fs(
    fs_id: u32,
    path: &str,
) -> Result<alloc::vec::Vec<u8>, PosixErrno> {
    let image = read_exec_image_from_fs(fs_id, path)?;
    validate_interp_for_image(&image, fs_id)?;
    Ok(image)
}

pub(super) fn execve(path: &str, _argv: &[&str], _envp: &[&str]) -> Result<(), PosixErrno> {
    #[cfg(not(feature = "process_abstraction"))]
    {
        let _ = (path, _argv, _envp);
        return Err(PosixErrno::NoSys);
    }

    #[cfg(feature = "process_abstraction")]
    {
        #[cfg(not(all(feature = "vfs", feature = "posix_fs")))]
        {
            let _ = (path, _argv, _envp);
            return Err(PosixErrno::NoSys);
        }

        #[cfg(all(feature = "vfs", feature = "posix_fs"))]
        {
            let image = read_exec_image_with_validated_interp(path)?;

            let pid = getpid();
            if pid == 0 {
                return Err(PosixErrno::Invalid);
            }

            if let Some(proc_arc) =
                crate::kernel::launch::process_arc_by_id(crate::interfaces::task::ProcessId(pid))
            {
                #[cfg(feature = "paging_enable")]
                {
                    if let Some(hhdm) = crate::hal::hhdm_offset() {
                        unsafe {
                            let offset = x86_64::VirtAddr::new(hhdm);
                            let lvl4 = crate::kernel::memory::paging::active_level_4_table(offset);
                            let mut page_manager =
                                crate::kernel::memory::paging::PageManager::new(offset, lvl4);
                            let mut frame_allocator = crate::kernel::vmm::PageAllocWrapper;
                            crate::kernel::module_loader::materialize_process_image(
                                &proc_arc,
                                &image,
                                &mut page_manager,
                                &mut frame_allocator,
                            )
                            .map_err(|_| PosixErrno::Invalid)?;
                        }
                    } else {
                        crate::kernel::module_loader::prepare_process_image(&proc_arc, &image)
                            .map_err(|_| PosixErrno::Invalid)?;
                    }
                }

                #[cfg(not(feature = "paging_enable"))]
                {
                    crate::kernel::module_loader::prepare_process_image(&proc_arc, &image)
                        .map_err(|_| PosixErrno::Invalid)?;
                }
                if proc_arc.effective_entry() == 0 {
                    return Err(PosixErrno::Invalid);
                }
                proc_arc.set_exec_path(path);
                Ok(())
            } else {
                Err(PosixErrno::Invalid)
            }
        }
    }
}

#[cfg(all(feature = "vfs", feature = "posix_fs"))]
pub(super) fn resolve_interp_path(image: &[u8]) -> Result<Option<String>, PosixErrno> {
    crate::kernel::module_loader::inspect_elf_image(image).map_err(|_| PosixErrno::Invalid)?;

    use xmas_elf::program::Type;
    const MAX_INTERP_PATH_BYTES: usize = 4096;
    let require_absolute = crate::config::KernelConfig::exec_elf_require_absolute_interp_path();
    let enforce_path_sanitization =
        crate::config::KernelConfig::exec_elf_enforce_interp_path_sanitization();
    let enforce_system_loader_paths =
        crate::config::KernelConfig::exec_elf_enforce_system_loader_paths();

    fn has_disallowed_path_segments(path: &str) -> bool {
        path.contains("//")
            || path.contains("/./")
            || path.contains("/../")
            || path.ends_with("/.")
            || path.ends_with("/..")
            || path.contains('\\')
    }

    fn is_supported_dynamic_loader_path(path: &str) -> bool {
        let is_system_loader_prefix = path.starts_with("/lib/")
            || path.starts_with("/lib64/")
            || path.starts_with("/usr/lib/")
            || path.starts_with("/usr/lib64/");
        if !is_system_loader_prefix {
            return false;
        }

        let file_name = path.rsplit('/').next().unwrap_or("");
        file_name.starts_with("ld-linux") || file_name.starts_with("ld-musl")
    }

    fn is_supported_dynamic_loader_name(path: &str) -> bool {
        let file_name = path.rsplit('/').next().unwrap_or(path);
        file_name.starts_with("ld-linux") || file_name.starts_with("ld-musl")
    }

    let elf_file = xmas_elf::ElfFile::new(image).map_err(|_| PosixErrno::Invalid)?;
    for ph in elf_file.program_iter() {
        if !matches!(ph.get_type(), Ok(Type::Interp)) {
            continue;
        }

        let interp_off = ph.offset() as usize;
        let interp_len = ph.file_size() as usize;
        if interp_len == 0 || interp_len > MAX_INTERP_PATH_BYTES {
            return Err(PosixErrno::Invalid);
        }
        let interp_bytes = image
            .get(interp_off..interp_off.saturating_add(interp_len))
            .ok_or(PosixErrno::Invalid)?;
        if interp_bytes.last().copied() != Some(0) {
            return Err(PosixErrno::Invalid);
        }
        let nul_idx = interp_bytes
            .iter()
            .position(|&b| b == 0)
            .ok_or(PosixErrno::Invalid)?;
        if interp_bytes[nul_idx + 1..].iter().any(|&b| b != 0) {
            return Err(PosixErrno::Invalid);
        }
        let interp_path = core::str::from_utf8(&interp_bytes[..nul_idx])
            .map_err(|_| PosixErrno::Invalid)?;
        if interp_path.is_empty() {
            return Err(PosixErrno::Invalid);
        }
        if require_absolute && !interp_path.starts_with('/') {
            return Err(PosixErrno::Invalid);
        }
        if enforce_path_sanitization && has_disallowed_path_segments(interp_path) {
            return Err(PosixErrno::Invalid);
        }
        if enforce_system_loader_paths {
            let supported = if interp_path.starts_with('/') {
                is_supported_dynamic_loader_path(interp_path)
            } else {
                is_supported_dynamic_loader_name(interp_path)
            };
            if !supported {
                return Err(PosixErrno::Invalid);
            }
        }
        return Ok(Some(String::from(interp_path)));
    }

    Ok(None)
}

#[cfg(all(feature = "vfs", feature = "posix_fs"))]
pub(super) fn execveat(fs_id: u32, path: &str, _argv: &[&str], _envp: &[&str]) -> Result<(), PosixErrno> {
    #[cfg(not(feature = "process_abstraction"))]
    {
        let _ = (fs_id, path, _argv, _envp);
        return Err(PosixErrno::NoSys);
    }

    #[cfg(feature = "process_abstraction")]
    {
        let image = read_exec_image_with_validated_interp_from_fs(fs_id, path)?;
        let _spawned = posix_spawn_from_image(basename_bytes(path), &image, 128, 0, 0, 0)?;
        exit_with_status(0)
    }
}

#[cfg(all(feature = "vfs", feature = "posix_fs"))]
pub(super) fn posix_spawn_from_path(
    fs_id: u32,
    path: &str,
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
) -> Result<usize, PosixErrno> {
    let image = read_exec_image_with_validated_interp_from_fs(fs_id, path)?;

    posix_spawn_from_image(
        basename_bytes(path),
        &image,
        priority,
        deadline,
        burst_time,
        kernel_stack_top,
    )
}

pub(super) fn posix_spawn_from_image(
    process_name: &[u8],
    image: &[u8],
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
) -> Result<usize, PosixErrno> {
    #[cfg(feature = "process_abstraction")]
    {
        #[cfg(all(feature = "vfs", feature = "posix_fs"))]
        {
            if let Some(interp_path) = resolve_interp_path(image)? {
                if crate::config::KernelConfig::exec_elf_require_absolute_interp_path()
                    && !interp_path.starts_with('/')
                {
                    return Err(PosixErrno::Invalid);
                }
            }
        }

        crate::kernel::module_loader::preflight_module_image(image)
            .map_err(|_| PosixErrno::Invalid)?;

        let parent_pid = getpid();
        let (pid, _tid) = crate::kernel::launch::spawn_bootstrap_from_image(
            process_name,
            image,
            priority,
            deadline,
            burst_time,
            kernel_stack_top,
        )
        .map_err(|e| match e {
            crate::kernel::launch::LaunchError::LoaderFailed => PosixErrno::Invalid,
            crate::kernel::launch::LaunchError::SchedulerUnavailable => PosixErrno::Again,
            crate::kernel::launch::LaunchError::InvalidSpawnRequest => PosixErrno::Invalid,
        })?;
        ensure_process_metadata(parent_pid);
        register_spawned_process(parent_pid, pid);
        Ok(pid)
    }

    #[cfg(not(feature = "process_abstraction"))]
    {
        let _ = (
            process_name,
            image,
            priority,
            deadline,
            burst_time,
            kernel_stack_top,
        );
        Err(PosixErrno::NoSys)
    }
}
