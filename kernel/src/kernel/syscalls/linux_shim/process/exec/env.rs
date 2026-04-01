use alloc::string::{String, ToString};

#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
pub(super) fn normalize_execve_argv(
    path: &str,
    mut argv: alloc::vec::Vec<alloc::string::String>,
) -> alloc::vec::Vec<alloc::string::String> {
    if argv.is_empty() {
        argv.push(String::from(path));
    } else if argv[0].is_empty() {
        argv[0] = String::from(path);
    }
    argv
}

#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
pub(super) fn normalize_execve_envp(
    path: &str,
    mut envp: alloc::vec::Vec<alloc::string::String>,
) -> alloc::vec::Vec<alloc::string::String> {
    ensure_env_var(&mut envp, "PATH", "/usr/bin:/bin:/usr/sbin:/sbin");
    ensure_env_var(&mut envp, "HOME", "/");
    ensure_env_var(&mut envp, "LANG", "C");
    ensure_env_var(&mut envp, "TERM", "hypercore");
    ensure_env_var(&mut envp, "LD_LIBRARY_PATH", "/lib:/usr/lib");
    ensure_env_var(&mut envp, "HYPERCORE_EXEC_PATH", path);
    ensure_env_var(
        &mut envp,
        "HYPERCORE_LINUX_RELEASE",
        crate::config::KernelConfig::linux_release(),
    );
    envp
}

#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
pub(super) fn append_exec_runtime_env(
    envp: &mut alloc::vec::Vec<alloc::string::String>,
    proc: &crate::kernel::process::Process,
) {
    let snapshot = proc.runtime_contract_snapshot();
    let init_hook_count = snapshot.init_calls.len();
    let fini_hook_count = snapshot.fini_calls.len();
    ensure_env_var(
        envp,
        "HYPERCORE_RUNTIME_ENTRY",
        &snapshot.runtime_entry.to_string(),
    );
    ensure_env_var(
        envp,
        "HYPERCORE_RUNTIME_FINI_ENTRY",
        &snapshot.runtime_fini_entry.to_string(),
    );
    ensure_env_var(
        envp,
        "HYPERCORE_IMAGE_ENTRY",
        &snapshot.image_entry.to_string(),
    );
    ensure_env_var(
        envp,
        "HYPERCORE_IMAGE_BASE",
        &snapshot.image_base.to_string(),
    );
    ensure_env_var(envp, "HYPERCORE_PHDR", &snapshot.phdr_addr.to_string());
    ensure_env_var(envp, "HYPERCORE_VDSO_BASE", &snapshot.vdso_base.to_string());
    ensure_env_var(envp, "HYPERCORE_VVAR_BASE", &snapshot.vvar_base.to_string());
    ensure_env_var(envp, "HYPERCORE_INIT_HOOKS", &init_hook_count.to_string());
    ensure_env_var(envp, "HYPERCORE_FINI_HOOKS", &fini_hook_count.to_string());
    ensure_env_var(envp, "HYPERCORE_EXEC_PATH", snapshot.exec_path.as_str());
    ensure_env_var(
        envp,
        "HYPERCORE_SYSCALL_ABI_VERSION",
        &alloc::format!(
            "{}.{}.{}",
            crate::kernel::syscalls::SYSCALL_ABI_VERSION_MAJOR,
            crate::kernel::syscalls::SYSCALL_ABI_VERSION_MINOR,
            crate::kernel::syscalls::SYSCALL_ABI_VERSION_PATCH
        ),
    );
    ensure_env_var(
        envp,
        "HYPERCORE_SYSCALL_ABI_MAGIC",
        &crate::kernel::syscalls::SYSCALL_ABI_MAGIC.to_string(),
    );
    ensure_env_var(envp, "HYPERCORE_SYSCALL_PLATFORM", execve_aux_platform());
    ensure_env_var(
        envp,
        "HYPERCORE_RUNTIME_VDSO",
        bool_env_value(snapshot.vdso_base != 0),
    );
    ensure_env_var(
        envp,
        "HYPERCORE_RUNTIME_VVAR",
        bool_env_value(snapshot.vvar_base != 0),
    );
    ensure_env_var(envp, "HYPERCORE_RUNTIME_TLS", bool_env_value(true));
    ensure_env_var(envp, "HYPERCORE_RUNTIME_SIGNAL_ABI", bool_env_value(true));
    ensure_env_var(
        envp,
        "HYPERCORE_RUNTIME_INIT_HOOKS",
        bool_env_value(init_hook_count != 0),
    );
    ensure_env_var(
        envp,
        "HYPERCORE_RUNTIME_FINI_HOOKS",
        bool_env_value(fini_hook_count != 0),
    );
    ensure_env_var(
        envp,
        "HYPERCORE_RUNTIME_FINI_TRAMPOLINE",
        bool_env_value(snapshot.runtime_fini_entry != 0),
    );
    ensure_env_var(
        envp,
        "HYPERCORE_RUNTIME_MULTI_USER",
        bool_env_value(crate::config::KernelConfig::multi_user_enabled()),
    );
    ensure_env_var(
        envp,
        "HYPERCORE_RUNTIME_CAPABILITY_MODEL",
        bool_env_value(crate::config::KernelConfig::capability_enforcement_enabled()),
    );
    ensure_env_var(envp, "HYPERCORE_STARTUP_CRT0", "1");
    ensure_env_var(envp, "HYPERCORE_STARTUP_ARGV_ENVP", "1");
    ensure_env_var(envp, "HYPERCORE_STARTUP_AUXV", "1");
    ensure_env_var(envp, "HYPERCORE_STARTUP_RUNTIME_CONTRACT_ENV", "1");
    ensure_env_var(envp, "HYPERCORE_ELF_CLASS", "ELF64");
    ensure_env_var(envp, "HYPERCORE_ELF_ENDIANNESS", "little");
    ensure_env_var(envp, "HYPERCORE_ELF_MACHINE_TARGETS", "x86_64,aarch64");
    ensure_env_var(
        envp,
        "HYPERCORE_ELF_LOADER_FEATURES",
        "pt_interp,dt_needed,dt_rpath,dt_runpath,dt_gnu_hash,dt_hash,dt_init,dt_init_array,dt_fini,dt_fini_array_tracking,pt_tls,vdso",
    );
    ensure_env_var(
        envp,
        "HYPERCORE_ELF_RELOCATION_FAMILIES",
        "relative,glob_dat,jmp_slot,plt32,pc32,got32,gotpcrel,gotpcrelx,tls_local_exec,tls_dtpmod,tls_dtpoff,irelative_best_effort,copy_best_effort",
    );
    ensure_env_var(
        envp,
        "HYPERCORE_LIBC_STARTUP_CAPABILITIES",
        "argv_envp_stack_layout,auxv_delivery,exec_runtime_env,pt_interp,init_hooks,vdso_contract,tls_contract",
    );
    ensure_env_var(
        envp,
        "HYPERCORE_LIBC_THREAD_CAPABILITIES",
        "arch_prctl_fs,set_tid_address,robust_list_tracking,futex_wait_wake,clear_child_tid_wakeup,pt_tls_template",
    );
    ensure_env_var(
        envp,
        "HYPERCORE_LIBC_SIGNAL_CAPABILITIES",
        "rt_sigaction,rt_sigreturn,signal_frame_delivery,sa_restorer,signal_mask_tracking",
    );
    ensure_env_var(
        envp,
        "HYPERCORE_LIBC_TIME_CAPABILITIES",
        "clock_gettime,clock_getres,gettimeofday,time,vdso_time_fastpath",
    );
    ensure_env_var(
        envp,
        "HYPERCORE_LIBC_FS_CAPABILITIES",
        "read,write,openat,close,execve,proc_sys_hypercore_abi",
    );
    ensure_env_var(
        envp,
        "HYPERCORE_LIBC_MEMORY_CAPABILITIES",
        "memcpy_contract,memset_contract,memmove_contract_planned,mmap_munmap_surface,page_size_env",
    );
    ensure_env_var(
        envp,
        "HYPERCORE_LIBC_STRING_CAPABILITIES",
        "strlen_contract,argv_execfn_scan,runtime_env_lookup_planned,auxv_string_slots",
    );
    ensure_env_var(
        envp,
        "HYPERCORE_LIBC_ERRNO_MODEL",
        "negative_errno_syscall_return,thread_local_errno_planned,libc_errno_wrapper_planned",
    );
    ensure_env_var(
        envp,
        "HYPERCORE_LIBC_SYSCALL_SURFACE",
        "read,write,openat,close,execve,exit,arch_prctl,getpid,getppid,getuid,geteuid,getgid,getegid,futex,rt_sigaction,rt_sigreturn,clock_gettime,clock_getres,gettimeofday,time,mmap,munmap",
    );
    ensure_env_var(envp, "HYPERCORE_AUXV_AT_BASE", "7");
    ensure_env_var(envp, "HYPERCORE_AUXV_AT_PHDR", "3");
    ensure_env_var(envp, "HYPERCORE_AUXV_AT_PHENT", "4");
    ensure_env_var(envp, "HYPERCORE_AUXV_AT_PHNUM", "5");
    ensure_env_var(envp, "HYPERCORE_AUXV_AT_PAGESZ", "6");
    ensure_env_var(envp, "HYPERCORE_AUXV_AT_ENTRY", "9");
    ensure_env_var(envp, "HYPERCORE_AUXV_AT_UID", "11");
    ensure_env_var(envp, "HYPERCORE_AUXV_AT_EUID", "12");
    ensure_env_var(envp, "HYPERCORE_AUXV_AT_GID", "13");
    ensure_env_var(envp, "HYPERCORE_AUXV_AT_EGID", "14");
    ensure_env_var(envp, "HYPERCORE_AUXV_AT_PLATFORM", "15");
    ensure_env_var(envp, "HYPERCORE_AUXV_AT_HWCAP", "16");
    ensure_env_var(envp, "HYPERCORE_AUXV_AT_CLKTCK", "17");
    ensure_env_var(envp, "HYPERCORE_AUXV_AT_SECURE", "23");
    ensure_env_var(envp, "HYPERCORE_AUXV_AT_RANDOM", "25");
    ensure_env_var(envp, "HYPERCORE_AUXV_AT_EXECFN", "31");
    ensure_env_var(envp, "HYPERCORE_AUXV_AT_SYSINFO_EHDR", "33");
    ensure_env_var(
        envp,
        "HYPERCORE_SYSCALL_READ",
        &crate::kernel::syscalls::linux_nr::READ.to_string(),
    );
    ensure_env_var(
        envp,
        "HYPERCORE_SYSCALL_WRITE",
        &crate::kernel::syscalls::linux_nr::WRITE.to_string(),
    );
    ensure_env_var(
        envp,
        "HYPERCORE_SYSCALL_OPENAT",
        &crate::kernel::syscalls::linux_nr::OPENAT.to_string(),
    );
    ensure_env_var(
        envp,
        "HYPERCORE_SYSCALL_CLOSE",
        &crate::kernel::syscalls::linux_nr::CLOSE.to_string(),
    );
    ensure_env_var(
        envp,
        "HYPERCORE_SYSCALL_EXECVE",
        &crate::kernel::syscalls::linux_nr::EXECVE.to_string(),
    );
    ensure_env_var(
        envp,
        "HYPERCORE_SYSCALL_EXIT",
        &crate::kernel::syscalls::linux_nr::EXIT.to_string(),
    );
    ensure_env_var(
        envp,
        "HYPERCORE_SYSCALL_ARCH_PRCTL",
        &crate::kernel::syscalls::linux_nr::ARCH_PRCTL.to_string(),
    );
    ensure_env_var(
        envp,
        "HYPERCORE_SYSCALL_GETPID",
        &crate::kernel::syscalls::linux_nr::GETPID.to_string(),
    );
    ensure_env_var(
        envp,
        "HYPERCORE_SYSCALL_GETPPID",
        &crate::kernel::syscalls::linux_nr::GETPPID.to_string(),
    );
    ensure_env_var(
        envp,
        "HYPERCORE_SYSCALL_GETUID",
        &crate::kernel::syscalls::linux_nr::GETUID.to_string(),
    );
    ensure_env_var(
        envp,
        "HYPERCORE_SYSCALL_GETEUID",
        &crate::kernel::syscalls::linux_nr::GETEUID.to_string(),
    );
    ensure_env_var(
        envp,
        "HYPERCORE_SYSCALL_GETGID",
        &crate::kernel::syscalls::linux_nr::GETGID.to_string(),
    );
    ensure_env_var(
        envp,
        "HYPERCORE_SYSCALL_GETEGID",
        &crate::kernel::syscalls::linux_nr::GETEGID.to_string(),
    );
    ensure_env_var(
        envp,
        "HYPERCORE_SYSCALL_CLOCK_GETTIME",
        &crate::kernel::syscalls::linux_nr::CLOCK_GETTIME.to_string(),
    );
    ensure_env_var(
        envp,
        "HYPERCORE_SYSCALL_GETTIMEOFDAY",
        &crate::kernel::syscalls::linux_nr::GETTIMEOFDAY.to_string(),
    );
    ensure_env_var(
        envp,
        "HYPERCORE_SYSCALL_TIME",
        &crate::kernel::syscalls::linux_nr::TIME.to_string(),
    );
    ensure_env_var(
        envp,
        "HYPERCORE_SYSCALL_FUTEX",
        &crate::kernel::syscalls::linux_nr::FUTEX.to_string(),
    );
    ensure_env_var(
        envp,
        "HYPERCORE_SYSCALL_MMAP",
        &crate::kernel::syscalls::linux_nr::MMAP.to_string(),
    );
    ensure_env_var(
        envp,
        "HYPERCORE_SYSCALL_MUNMAP",
        &crate::kernel::syscalls::linux_nr::MUNMAP.to_string(),
    );
    ensure_env_var(
        envp,
        "HYPERCORE_SYSCALL_RT_SIGACTION",
        &crate::kernel::syscalls::linux_nr::RT_SIGACTION.to_string(),
    );
    ensure_env_var(
        envp,
        "HYPERCORE_SYSCALL_RT_SIGRETURN",
        &crate::kernel::syscalls::linux_nr::RT_SIGRETURN.to_string(),
    );
    ensure_env_var(
        envp,
        "HYPERCORE_SECURITY_PROFILE",
        match crate::modules::security::active_profile() {
            crate::modules::security::SecurityProfile::Null => "null",
            crate::modules::security::SecurityProfile::Acl => "acl",
            crate::modules::security::SecurityProfile::Capabilities => "capabilities",
            crate::modules::security::SecurityProfile::Sel4 => "sel4",
            crate::modules::security::SecurityProfile::ZeroTrust => "zero-trust",
        },
    );
}

#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
fn ensure_env_var(envp: &mut alloc::vec::Vec<alloc::string::String>, key: &str, value: &str) {
    let prefix = alloc::format!("{key}=");
    if envp.iter().any(|item| item.starts_with(prefix.as_str())) {
        return;
    }
    envp.push(alloc::format!("{key}={value}"));
}

#[cfg(not(feature = "linux_compat"))]
#[inline(always)]
#[allow(dead_code)]
fn bool_env_value(value: bool) -> &'static str {
    if value {
        "1"
    } else {
        "0"
    }
}

#[cfg(not(feature = "linux_compat"))]
#[inline(always)]
#[allow(dead_code)]
pub(super) fn execve_aux_platform() -> &'static str {
    #[cfg(target_arch = "x86_64")]
    {
        "x86_64"
    }
    #[cfg(target_arch = "aarch64")]
    {
        "aarch64"
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        "unknown"
    }
}

#[cfg(not(feature = "linux_compat"))]
#[inline(always)]
#[allow(dead_code)]
pub(super) fn execve_aux_hwcap() -> (usize, usize) {
    #[cfg(target_arch = "x86_64")]
    {
        // Conservative baseline until CPUID feature projection is plumbed into auxv.
        (0, 0)
    }
    #[cfg(target_arch = "aarch64")]
    {
        (0, 0)
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        (0, 0)
    }
}
