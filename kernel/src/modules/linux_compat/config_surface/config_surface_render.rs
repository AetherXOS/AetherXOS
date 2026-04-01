use super::*;
use alloc::format;
use core::fmt::Write;

pub fn render_compat_config_surface(kind: CompatConfigSurfaceKind) -> Result<String, &'static str> {
    let snapshot = compat_config_surface_snapshot(kind);
    if !snapshot.enabled {
        return Err("compat config surface disabled by boundary/runtime policy");
    }

    let kernel = crate::config::KernelConfig::snapshot();
    let mut lines = String::new();
    let _ = writeln!(&mut lines, "surface={}", kind.name());
    let _ = writeln!(&mut lines, "mount_path={}", kind.mount_path());
    let _ = writeln!(&mut lines, "runtime_gate={}", kind.runtime_gate_key());
    let _ = writeln!(
        &mut lines,
        "linux_compat_surface={}",
        kernel.compat_surface.expose_linux_compat_surface
    );
    let _ = writeln!(
        &mut lines,
        "attack_surface_budget={}",
        kernel.compat_surface.attack_surface_budget
    );
    let _ = writeln!(
        &mut lines,
        "boundary_mode={:?}",
        crate::config::KernelConfig::boundary_mode()
    );
    let _ = writeln!(
        &mut lines,
        "vfs_api={}",
        kernel.library_runtime.expose_vfs_api
    );
    let _ = writeln!(
        &mut lines,
        "network_api={}",
        kernel.library_runtime.expose_network_api
    );
    let _ = writeln!(
        &mut lines,
        "ipc_api={}",
        kernel.library_runtime.expose_ipc_api
    );
    let _ = writeln!(
        &mut lines,
        "proc_config_api={}",
        kernel.library_runtime.expose_proc_config_api
    );
    let _ = writeln!(
        &mut lines,
        "sysctl_api={}",
        kernel.library_runtime.expose_sysctl_api
    );
    let _ = writeln!(
        &mut lines,
        "security_enforcement={}",
        kernel.credentials.security_enforcement
    );
    let _ = writeln!(
        &mut lines,
        "capability_enforcement={}",
        kernel.credentials.capability_enforcement
    );
    let _ = writeln!(&mut lines, "multi_user={}", kernel.credentials.multi_user);
    let _ = writeln!(
        &mut lines,
        "credential_enforcement={}",
        kernel.credentials.credential_enforcement
    );
    let _ = writeln!(&mut lines, "feature_controls={}", snapshot.feature_count);

    for control in crate::config::KernelConfig::feature_controls() {
        let _ = writeln!(
            &mut lines,
            "feature.{}={}|compile_time={}|runtime_gate={}",
            control.name,
            control.effective_enabled,
            control.compile_enabled,
            control.runtime_gate_key.unwrap_or("-")
        );
    }

    Ok(lines)
}

pub fn render_proc_config_snapshot() -> Result<String, &'static str> {
    render_compat_config_surface(CompatConfigSurfaceKind::ProcConfig)
}

pub fn render_sysctl_snapshot() -> Result<String, &'static str> {
    render_compat_config_surface(CompatConfigSurfaceKind::Sysctl)
}

pub fn render_compat_config_key(key: &str) -> Result<String, &'static str> {
    let normalized = normalize_runtime_style_key(key);
    if normalized.is_empty() {
        return render_sysctl_snapshot();
    }

    match normalized.as_str() {
        "boundary_mode" => Ok(format!(
            "{:?}\n",
            crate::config::KernelConfig::boundary_mode()
        )),
        "vfs_library_api_exposed" => Ok(format!(
            "{}\n",
            crate::config::KernelConfig::is_vfs_library_api_exposed()
        )),
        "network_library_api_exposed" => Ok(format!(
            "{}\n",
            crate::config::KernelConfig::is_network_library_api_exposed()
        )),
        "ipc_library_api_exposed" => Ok(format!(
            "{}\n",
            crate::config::KernelConfig::is_ipc_library_api_exposed()
        )),
        "proc_config_api_exposed" => Ok(format!(
            "{}\n",
            crate::config::KernelConfig::is_proc_config_api_exposed()
        )),
        "sysctl_api_exposed" => Ok(format!(
            "{}\n",
            crate::config::KernelConfig::is_sysctl_api_exposed()
        )),
        "security_enforcement_enabled" => Ok(format!(
            "{}\n",
            crate::config::KernelConfig::security_enforcement_enabled()
        )),
        "capability_enforcement_enabled" => Ok(format!(
            "{}\n",
            crate::config::KernelConfig::capability_enforcement_enabled()
        )),
        "multi_user_enabled" => Ok(format!(
            "{}\n",
            crate::config::KernelConfig::multi_user_enabled()
        )),
        "credential_enforcement_enabled" => Ok(format!(
            "{}\n",
            crate::config::KernelConfig::credential_enforcement_enabled()
        )),
        "telemetry_enabled" => Ok(format!(
            "{}\n",
            crate::config::KernelConfig::is_telemetry_enabled()
        )),
        "compat_attack_surface_budget" => Ok(format!(
            "{}\n",
            crate::config::KernelConfig::compat_attack_surface_budget()
        )),
        "linux_ptrace_compat_enabled" | "ptrace_compat_enabled" => Ok(format!(
            "{}\n",
            crate::modules::linux_compat::config::ptrace_compat_enabled()
        )),
        "linux_seccomp_compat_enabled" | "seccomp_compat_enabled" => Ok(format!(
            "{}\n",
            crate::modules::linux_compat::config::seccomp_compat_enabled()
        )),
        "linux_mman_soft_fallback_enabled" | "mman_soft_fallback_enabled" => Ok(format!(
            "{}\n",
            crate::modules::linux_compat::config::mman_soft_fallback_enabled()
        )),
        "linux_wayland_compat_enabled" | "wayland_compat_enabled" => Ok(format!(
            "{}\n",
            crate::modules::linux_compat::config::wayland_compat_enabled()
        )),
        "linux_x11_compat_enabled" | "x11_compat_enabled" => Ok(format!(
            "{}\n",
            crate::modules::linux_compat::config::x11_compat_enabled()
        )),
        "syscall_abi_magic" => Ok(format!(
            "{}\n",
            crate::kernel::syscalls::SYSCALL_ABI_MAGIC
        )),
        "syscall_abi_version_major" => Ok(format!(
            "{}\n",
            crate::kernel::syscalls::SYSCALL_ABI_VERSION_MAJOR
        )),
        "syscall_abi_version_minor" => Ok(format!(
            "{}\n",
            crate::kernel::syscalls::SYSCALL_ABI_VERSION_MINOR
        )),
        "syscall_abi_version_patch" => Ok(format!(
            "{}\n",
            crate::kernel::syscalls::SYSCALL_ABI_VERSION_PATCH
        )),
        "syscall_abi_min_compat_major" => Ok(format!(
            "{}\n",
            crate::kernel::syscalls::SYSCALL_ABI_MIN_COMPAT_MAJOR
        )),
        "syscall_abi_stable" => Ok(format!(
            "{}\n",
            (crate::kernel::syscalls::SYSCALL_ABI_FLAG_STABLE != 0) as u8
        )),
        "syscall_page_size" => Ok(format!("{}\n", crate::kernel::syscalls::PAGE_SIZE)),
        "syscall_user_space_bottom" => Ok(format!(
            "{}\n",
            crate::kernel::syscalls::USER_SPACE_BOTTOM_INCLUSIVE
        )),
        "syscall_user_space_top" => Ok(format!(
            "{}\n",
            crate::kernel::syscalls::USER_SPACE_TOP_EXCLUSIVE
        )),
        "syscall_platform" => Ok(format!("{}\n", syscall_abi_platform())),
        "syscall_openat" => Ok(format!(
            "{}\n",
            crate::kernel::syscalls::linux_nr::OPENAT
        )),
        "syscall_close" => Ok(format!("{}\n", crate::kernel::syscalls::linux_nr::CLOSE)),
        "syscall_read" => Ok(format!("{}\n", crate::kernel::syscalls::linux_nr::READ)),
        "syscall_write" => Ok(format!("{}\n", crate::kernel::syscalls::linux_nr::WRITE)),
        "syscall_execve" => Ok(format!("{}\n", crate::kernel::syscalls::linux_nr::EXECVE)),
        "syscall_exit" => Ok(format!("{}\n", crate::kernel::syscalls::linux_nr::EXIT)),
        "syscall_arch_prctl" => Ok(format!(
            "{}\n",
            crate::kernel::syscalls::linux_nr::ARCH_PRCTL
        )),
        "syscall_getpid" => Ok(format!("{}\n", crate::kernel::syscalls::linux_nr::GETPID)),
        "syscall_getppid" => Ok(format!("{}\n", crate::kernel::syscalls::linux_nr::GETPPID)),
        "syscall_getuid" => Ok(format!("{}\n", crate::kernel::syscalls::linux_nr::GETUID)),
        "syscall_geteuid" => Ok(format!("{}\n", crate::kernel::syscalls::linux_nr::GETEUID)),
        "syscall_getgid" => Ok(format!("{}\n", crate::kernel::syscalls::linux_nr::GETGID)),
        "syscall_getegid" => Ok(format!("{}\n", crate::kernel::syscalls::linux_nr::GETEGID)),
        "syscall_clock_gettime" => Ok(format!(
            "{}\n",
            crate::kernel::syscalls::linux_nr::CLOCK_GETTIME
        )),
        "syscall_gettimeofday" => Ok(format!(
            "{}\n",
            crate::kernel::syscalls::linux_nr::GETTIMEOFDAY
        )),
        "syscall_time" => Ok(format!("{}\n", crate::kernel::syscalls::linux_nr::TIME)),
        "syscall_futex" => Ok(format!("{}\n", crate::kernel::syscalls::linux_nr::FUTEX)),
        "syscall_mmap" => Ok(format!("{}\n", crate::kernel::syscalls::linux_nr::MMAP)),
        "syscall_munmap" => Ok(format!("{}\n", crate::kernel::syscalls::linux_nr::MUNMAP)),
        "syscall_rt_sigaction" => Ok(format!(
            "{}\n",
            crate::kernel::syscalls::linux_nr::RT_SIGACTION
        )),
        "syscall_rt_sigreturn" => Ok(format!(
            "{}\n",
            crate::kernel::syscalls::linux_nr::RT_SIGRETURN
        )),
        "loader_gnu_hash_supported" => Ok("1\n".into()),
        "loader_runpath_supported" => Ok("1\n".into()),
        "loader_init_hooks_supported" => Ok("1\n".into()),
        "loader_fini_hooks_tracked" => Ok("1\n".into()),
        "loader_fini_trampoline_supported" => Ok("1\n".into()),
        "runtime_vdso_supported" => Ok("1\n".into()),
        "runtime_vvar_supported" => Ok("1\n".into()),
        "runtime_tls_supported" => Ok("1\n".into()),
        "runtime_signal_abi_supported" => Ok("1\n".into()),
        "runtime_exec_env_supported" => Ok("1\n".into()),
        "runtime_multi_user_supported" => Ok(format!(
            "{}\n",
            crate::config::KernelConfig::multi_user_enabled()
        )),
        "runtime_capability_model_supported" => Ok(format!(
            "{}\n",
            crate::config::KernelConfig::capability_enforcement_enabled()
        )),
        "startup_crt0_supported" => Ok("1\n".into()),
        "startup_argv_envp_supported" => Ok("1\n".into()),
        "startup_auxv_supported" => Ok("1\n".into()),
        "startup_runtime_contract_env_supported" => Ok("1\n".into()),
        "startup_stack_layout" => Ok("argc|argv|null|envp|null|auxv\n".into()),
        "runtime_core_helpers" => Ok(
            "strlen,print_hex_value,print_u64_decimal,argc_argv_envp_scan,auxv_scan,vdso_magic_check,runtime_init_state,runtime_status_word,runtime_probe_mask,runtime_probe_status_word,runtime_probe_summary\n"
                .into(),
        ),
        "runtime_core_memory_helpers" => Ok(
            "memcpy_contract,memset_contract,memmove_contract_planned\n".into(),
        ),
        "runtime_core_string_helpers" => Ok(
            "strlen_contract,argv_execfn_scan,auxv_string_slots\n".into(),
        ),
        "runtime_core_auxv_helpers" => Ok(
            "auxv_scan,auxv_execfn_lookup,auxv_sysinfo_ehdr_lookup,auxv_pagesz_lookup,auxv_random_lookup,auxv_presence_checks\n"
                .into(),
        ),
        "runtime_core_env_helpers" => Ok(
            "runtime_env_contract_keys,syscall_env_key_lookup,auxv_env_key_lookup,default_env_bootstrap,last_env_name_tracking,builtin_env_lookup,last_env_value_tracking\n"
                .into(),
        ),
        "runtime_core_errno_features" => Ok(
            "negative_errno_syscall_return,thread_local_errno_planned,errno_wrapper_planned,errno_state_storage,errno_query_api\n"
                .into(),
        ),
        "runtime_core_entrypoints" => Ok(
            "_start,__hypercore_crt0_start,__hypercore_auxv_init,__hypercore_env_init,__hypercore_syscall_init\n"
                .into(),
        ),
        "runtime_core_source_units" => Ok(
            "crt0.S,runtime_state.c,auxv_runtime.c,env_runtime.c,runtime_syscall.c,runtime_entry.c,runtime_probe.c,runtime_smoke.c\n".into(),
        ),
        "runtime_core_wrappers" => Ok(
            "read,write,openat,close,arch_prctl,getpid,getppid,getuid,geteuid,getgid,getegid,clock_gettime,gettimeofday,time,futex,mmap,munmap,rt_sigaction,rt_sigreturn,execve,exit\n"
                .into(),
        ),
        "runtime_core_startup_features" => Ok(
            "argv0_print,execfn_print,argc_print,pagesz_print,random_print,sysinfo_ehdr_print,vdso_elf_probe,startup_status_report,runtime_status_word_report,runtime_probe_mask_report,runtime_probe_summary_report\n"
                .into(),
        ),
        "elf_class" => Ok("ELF64\n".into()),
        "elf_endianness" => Ok("little\n".into()),
        "elf_machine_targets" => Ok("x86_64,aarch64\n".into()),
        "elf_loader_features" => Ok(
            "pt_interp,dt_needed,dt_rpath,dt_runpath,dt_gnu_hash,dt_hash,dt_init,dt_init_array,dt_fini,dt_fini_array_tracking,pt_tls,vdso\n"
                .into(),
        ),
        "elf_relocation_families" => Ok(
            "relative,glob_dat,jmp_slot,plt32,pc32,got32,gotpcrel,gotpcrelx,tls_local_exec,tls_dtpmod,tls_dtpoff,irelative_best_effort,copy_best_effort\n"
                .into(),
        ),
        "elf_dynamic_tags" => Ok(
            "DT_NEEDED,DT_RPATH,DT_RUNPATH,DT_SONAME,DT_STRTAB,DT_STRSZ,DT_SYMTAB,DT_SYMENT,DT_HASH,DT_GNU_HASH,DT_RELA,DT_RELASZ,DT_INIT,DT_INIT_ARRAY,DT_FINI,DT_FINI_ARRAY,DT_FLAGS_1,DT_VERSYM\n"
                .into(),
        ),
        "libc_startup_capabilities" => Ok(
            "argv_envp_stack_layout,auxv_delivery,exec_runtime_env,pt_interp,init_hooks,vdso_contract,tls_contract\n"
                .into(),
        ),
        "libc_thread_capabilities" => Ok(
            "arch_prctl_fs,set_tid_address,robust_list_tracking,futex_wait_wake,clear_child_tid_wakeup,pt_tls_template\n"
                .into(),
        ),
        "libc_signal_capabilities" => Ok(
            "rt_sigaction,rt_sigreturn,signal_frame_delivery,sa_restorer,signal_mask_tracking\n"
                .into(),
        ),
        "libc_time_capabilities" => Ok(
            "clock_gettime,clock_getres,gettimeofday,time,vdso_time_fastpath\n"
                .into(),
        ),
        "libc_fs_capabilities" => Ok(
            "read,write,openat,close,execve,proc_sys_hypercore_abi\n".into(),
        ),
        "libc_memory_capabilities" => Ok(
            "memcpy_contract,memset_contract,memmove_contract_planned,mmap_munmap_surface,page_size_env\n"
                .into(),
        ),
        "libc_string_capabilities" => Ok(
            "strlen_contract,argv_execfn_scan,runtime_env_lookup_planned,auxv_string_slots\n"
                .into(),
        ),
        "libc_errno_model" => Ok(
            "negative_errno_syscall_return,thread_local_errno_planned,libc_errno_wrapper_planned\n"
                .into(),
        ),
        "libc_planned_symbols" => Ok(
            "__errno_location,memcpy,memset,memmove,strlen,getenv,syscall,__libc_start_main\n"
                .into(),
        ),
        "libc_source_modules" => Ok(
            "libc_state.c,errno_runtime.c,memory_runtime.c,string_runtime.c,startup_runtime.c,libc_syscall.c\n"
                .into(),
        ),
        "libc_syscall_surface" => Ok(
            "read,write,openat,close,execve,exit,arch_prctl,getpid,getppid,getuid,geteuid,getgid,getegid,futex,rt_sigaction,rt_sigreturn,clock_gettime,clock_getres,gettimeofday,time,mmap,munmap\n"
                .into(),
        ),
        "startup_runtime_contract_env_keys" => Ok(
            "HYPERCORE_RUNTIME_ENTRY,HYPERCORE_RUNTIME_FINI_ENTRY,HYPERCORE_IMAGE_ENTRY,HYPERCORE_IMAGE_BASE,HYPERCORE_PHDR,HYPERCORE_VDSO_BASE,HYPERCORE_VVAR_BASE,HYPERCORE_INIT_HOOKS,HYPERCORE_FINI_HOOKS,HYPERCORE_EXEC_PATH,HYPERCORE_SECURITY_PROFILE,HYPERCORE_SYSCALL_ABI_VERSION,HYPERCORE_SYSCALL_PLATFORM,HYPERCORE_RUNTIME_VDSO,HYPERCORE_RUNTIME_TLS,HYPERCORE_RUNTIME_SIGNAL_ABI,HYPERCORE_RUNTIME_INIT_HOOKS,HYPERCORE_RUNTIME_FINI_HOOKS,HYPERCORE_RUNTIME_FINI_TRAMPOLINE,HYPERCORE_RUNTIME_MULTI_USER,HYPERCORE_RUNTIME_CAPABILITY_MODEL\n"
                .into(),
        ),
        "startup_syscall_env_keys" => Ok(
            "HYPERCORE_SYSCALL_READ,HYPERCORE_SYSCALL_WRITE,HYPERCORE_SYSCALL_OPENAT,HYPERCORE_SYSCALL_CLOSE,HYPERCORE_SYSCALL_EXECVE,HYPERCORE_SYSCALL_EXIT,HYPERCORE_SYSCALL_ARCH_PRCTL,HYPERCORE_SYSCALL_GETPID,HYPERCORE_SYSCALL_GETPPID,HYPERCORE_SYSCALL_GETUID,HYPERCORE_SYSCALL_GETEUID,HYPERCORE_SYSCALL_GETGID,HYPERCORE_SYSCALL_GETEGID,HYPERCORE_SYSCALL_CLOCK_GETTIME,HYPERCORE_SYSCALL_GETTIMEOFDAY,HYPERCORE_SYSCALL_TIME,HYPERCORE_SYSCALL_FUTEX,HYPERCORE_SYSCALL_MMAP,HYPERCORE_SYSCALL_MUNMAP,HYPERCORE_SYSCALL_RT_SIGACTION,HYPERCORE_SYSCALL_RT_SIGRETURN\n"
                .into(),
        ),
        "startup_auxv_env_keys" => Ok(
            "HYPERCORE_AUXV_AT_BASE,HYPERCORE_AUXV_AT_PHDR,HYPERCORE_AUXV_AT_PHENT,HYPERCORE_AUXV_AT_PHNUM,HYPERCORE_AUXV_AT_PAGESZ,HYPERCORE_AUXV_AT_ENTRY,HYPERCORE_AUXV_AT_UID,HYPERCORE_AUXV_AT_EUID,HYPERCORE_AUXV_AT_GID,HYPERCORE_AUXV_AT_EGID,HYPERCORE_AUXV_AT_PLATFORM,HYPERCORE_AUXV_AT_HWCAP,HYPERCORE_AUXV_AT_CLKTCK,HYPERCORE_AUXV_AT_SECURE,HYPERCORE_AUXV_AT_RANDOM,HYPERCORE_AUXV_AT_EXECFN,HYPERCORE_AUXV_AT_SYSINFO_EHDR\n"
                .into(),
        ),
        "auxv_at_base" => Ok("7\n".into()),
        "auxv_at_phdr" => Ok("3\n".into()),
        "auxv_at_phent" => Ok("4\n".into()),
        "auxv_at_phnum" => Ok("5\n".into()),
        "auxv_at_pagesz" => Ok("6\n".into()),
        "auxv_at_entry" => Ok("9\n".into()),
        "auxv_at_uid" => Ok("11\n".into()),
        "auxv_at_euid" => Ok("12\n".into()),
        "auxv_at_gid" => Ok("13\n".into()),
        "auxv_at_egid" => Ok("14\n".into()),
        "auxv_at_platform" => Ok("15\n".into()),
        "auxv_at_hwcap" => Ok("16\n".into()),
        "auxv_at_clktck" => Ok("17\n".into()),
        "auxv_at_secure" => Ok("23\n".into()),
        "auxv_at_random" => Ok("25\n".into()),
        "auxv_at_execfn" => Ok("31\n".into()),
        "auxv_at_sysinfo_ehdr" => Ok("33\n".into()),
        _ => {
            if let Some(feature_name) = normalized.strip_prefix("feature_") {
                let dotted = feature_name.replace('_', ".");
                let direct = crate::config::KernelConfig::feature_control(feature_name)
                    .or_else(|| crate::config::KernelConfig::feature_control(dotted.as_str()));
                if let Some(control) = direct {
                    return Ok(format!(
                        "name={}\ncategory={}\ncompile_enabled={}\nruntime_gate={}\neffective_enabled={}\n",
                        control.name,
                        control.category,
                        control.compile_enabled,
                        control.runtime_gate_key.unwrap_or("-"),
                        control.effective_enabled
                    ));
                }
            }
            Err("unsupported compat config key")
        }
    }
}

fn build_surface_summary() -> String {
    let mut out = String::new();
    let compat = crate::config::KernelConfig::compat_surface_profile();
    let _ = writeln!(
        &mut out,
        "linux_compat_surface={}",
        compat.expose_linux_compat_surface
    );
    let _ = writeln!(
        &mut out,
        "proc_config_surface={}",
        compat.expose_proc_config_api
    );
    let _ = writeln!(&mut out, "sysctl_surface={}", compat.expose_sysctl_api);
    let _ = writeln!(
        &mut out,
        "attack_surface_budget={}",
        compat.attack_surface_budget
    );
    let credentials = crate::config::KernelConfig::credential_runtime_profile();
    let _ = writeln!(
        &mut out,
        "security_enforcement={}",
        credentials.security_enforcement
    );
    let _ = writeln!(
        &mut out,
        "capability_enforcement={}",
        credentials.capability_enforcement
    );
    let _ = writeln!(&mut out, "multi_user={}", credentials.multi_user);
    let _ = writeln!(
        &mut out,
        "credential_enforcement={}",
        credentials.credential_enforcement
    );
    let _ = writeln!(
        &mut out,
        "syscall_abi_version={}.{}.{}",
        crate::kernel::syscalls::SYSCALL_ABI_VERSION_MAJOR,
        crate::kernel::syscalls::SYSCALL_ABI_VERSION_MINOR,
        crate::kernel::syscalls::SYSCALL_ABI_VERSION_PATCH
    );
    let _ = writeln!(&mut out, "syscall_platform={}", syscall_abi_platform());
    out
}

pub fn render_compat_virtual_file(path: &str) -> Result<String, &'static str> {
    match path.trim() {
        "/proc/hypercore/config" | "proc/hypercore/config" => render_proc_config_snapshot(),
        "/proc/sys/hypercore/config" | "proc/sys/hypercore/config" => render_sysctl_snapshot(),
        "/proc/compat_surface_summary.txt" | "proc/compat_surface_summary.txt" => {
            Ok(build_surface_summary())
        }
        _ => {
            let key = compat_path_to_config_key(path).ok_or("unsupported compat path")?;
            render_compat_config_key(&key)
        }
    }
}
