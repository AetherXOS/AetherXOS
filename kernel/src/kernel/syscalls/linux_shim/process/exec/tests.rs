use super::*;

#[test_case]
fn sys_linux_execve_invalid_path_pointer_returns_efault_or_enosys() {
    let rc = sys_linux_execve(0, 0, 0);
    assert!(
        rc == linux_errno(crate::modules::posix_consts::errno::EFAULT)
            || rc == linux_errno(crate::modules::posix_consts::errno::ENOSYS)
    );
}

#[test_case]
fn sys_linux_execveat_rejects_unknown_flags() {
    let path = b"/bin/true\0";
    let rc = sys_linux_execveat(
        crate::kernel::syscalls::linux_shim::LINUX_AT_FDCWD,
        path.as_ptr() as usize,
        0,
        0,
        0x4000,
    );
    assert_eq!(rc, linux_errno(crate::modules::posix_consts::errno::EINVAL));
}

#[test_case]
fn sys_linux_execveat_empty_path_without_flag_fails() {
    let empty = b"\0";
    let rc = sys_linux_execveat(
        crate::kernel::syscalls::linux_shim::LINUX_AT_FDCWD,
        empty.as_ptr() as usize,
        0,
        0,
        0,
    );
    assert_eq!(rc, linux_errno(crate::modules::posix_consts::errno::ENOENT));
}

#[test_case]
fn sys_linux_execveat_relative_path_invalid_dirfd_returns_ebadf() {
    let rel = b"bin/true\0";
    let rc = sys_linux_execveat(-5, rel.as_ptr() as usize, 0, 0, 0);
    assert_eq!(rc, linux_errno(crate::modules::posix_consts::errno::EBADF));
}

#[cfg(not(feature = "posix_fs"))]
#[test_case]
fn sys_linux_execveat_relative_path_with_dirfd_without_fs_returns_enosys() {
    let rel = b"bin/true\0";
    let rc = sys_linux_execveat(3, rel.as_ptr() as usize, 0, 0, 0);
    assert_eq!(rc, linux_errno(crate::modules::posix_consts::errno::ENOSYS));
}

#[test_case]
fn resolve_execveat_path_relative_at_fdcwd_passes_through() {
    let p = resolve_execveat_path(
        crate::kernel::syscalls::linux_shim::LINUX_AT_FDCWD,
        "bin/true",
        0,
    )
    .expect("resolve");
    assert_eq!(p, "bin/true");
}

#[cfg(feature = "posix_process")]
#[test_case]
fn validate_exec_entry_point_rejects_zero_entry() {
    assert_eq!(
        validate_exec_entry_point(0),
        Err(linux_errno(crate::modules::posix_consts::errno::ENOEXEC))
    );
    assert_eq!(validate_exec_entry_point(0x1000), Ok(()));
}

#[cfg(feature = "posix_process")]
#[test_case]
fn sanitized_phdr_aux_values_accepts_only_sane_tuple() {
    assert_eq!(sanitized_phdr_aux_values(0, 56, 9), None);
    assert_eq!(sanitized_phdr_aux_values(0x4000, 0, 9), None);
    assert_eq!(sanitized_phdr_aux_values(0x4000, 8, 9), None);
    assert_eq!(sanitized_phdr_aux_values(0x4000, 8192, 9), None);
    assert_eq!(sanitized_phdr_aux_values(0x4000, 56, 0), None);
    assert_eq!(
        sanitized_phdr_aux_values(0x4000, 56, 9),
        Some((0x4000, 56, 9))
    );
}

#[cfg(feature = "posix_process")]
#[test_case]
fn validate_exec_handoff_contract_rejects_missing_required_phdr_triplet() {
    crate::config::KernelConfig::reset_runtime_overrides();
    crate::config::KernelConfig::set_exec_auxv_require_phdr_triplet(Some(true));

    assert_eq!(
        validate_exec_handoff_contract(0x401000, 0x400000, 0, 0, 0),
        Err(linux_errno(crate::modules::posix_consts::errno::ENOEXEC))
    );

    crate::config::KernelConfig::reset_runtime_overrides();
}

#[cfg(feature = "posix_process")]
#[test_case]
fn validate_exec_handoff_contract_allows_missing_phdr_triplet_when_relaxed() {
    crate::config::KernelConfig::reset_runtime_overrides();
    crate::config::KernelConfig::set_exec_auxv_require_phdr_triplet(Some(false));

    assert_eq!(
        validate_exec_handoff_contract(0x401000, 0x400000, 0, 0, 0),
        Ok(())
    );

    crate::config::KernelConfig::reset_runtime_overrides();
}
