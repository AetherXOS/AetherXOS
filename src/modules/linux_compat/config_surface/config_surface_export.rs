use super::*;
use alloc::format;
use core::fmt::Write;
use alloc::vec::Vec;

pub fn exportable_compat_surfaces() -> Vec<CompatConfigSurfaceSnapshot> {
    [
        CompatConfigSurfaceKind::ProcConfig,
        CompatConfigSurfaceKind::Sysctl,
    ]
    .into_iter()
    .map(compat_config_surface_snapshot)
    .collect()
}

#[cfg(feature = "vfs")]
pub fn export_compat_surfaces_to_mount(
    mount_path: &str,
    export_dir: &str,
) -> Result<usize, &'static str> {
    let mount_id = crate::kernel::vfs_control::mount_id_by_path(mount_path.as_bytes())
        .ok_or("mount path not found")?;
    let tid = crate::interfaces::task::TaskId(0);
    let root = normalize_export_root(export_dir);
    ensure_dir(mount_id, &root, tid)?;

    let proc_dir = format!("{}/proc", root);
    let proc_hypercore_dir = format!("{}/hypercore", proc_dir);
    let proc_sys_dir = format!("{}/sys", proc_dir);
    let proc_sys_hypercore_dir = format!("{}/hypercore", proc_sys_dir);
    let proc_sys_abi_dir = format!("{}/abi", proc_sys_hypercore_dir);
    let proc_sys_features_dir = format!("{}/features", proc_sys_hypercore_dir);
    let proc_sys_runtime_dir = format!("{}/runtime", proc_sys_hypercore_dir);
    let proc_sys_library_dir = format!("{}/library", proc_sys_hypercore_dir);
    let proc_sys_compat_dir = format!("{}/compat", proc_sys_hypercore_dir);

    ensure_dir(mount_id, &proc_dir, tid)?;
    ensure_dir(mount_id, &proc_hypercore_dir, tid)?;
    ensure_dir(mount_id, &proc_sys_dir, tid)?;
    ensure_dir(mount_id, &proc_sys_hypercore_dir, tid)?;
    ensure_dir(mount_id, &proc_sys_abi_dir, tid)?;
    ensure_dir(mount_id, &proc_sys_features_dir, tid)?;
    ensure_dir(mount_id, &proc_sys_runtime_dir, tid)?;
    ensure_dir(mount_id, &proc_sys_library_dir, tid)?;
    ensure_dir(mount_id, &proc_sys_compat_dir, tid)?;

    let mut exported = 0usize;
    if let Ok(proc_snapshot) = render_proc_config_snapshot() {
        write_file(
            mount_id,
            &format!("{}/config", proc_hypercore_dir),
            proc_snapshot.as_bytes(),
            tid,
        )?;
        exported += 1;
    }

    if let Ok(sysctl_snapshot) = render_sysctl_snapshot() {
        write_file(
            mount_id,
            &format!("{}/config", proc_sys_hypercore_dir),
            sysctl_snapshot.as_bytes(),
            tid,
        )?;
        exported += 1;
    }

    let summary = build_surface_summary();
    write_file(
        mount_id,
        &format!("{}/compat_surface_summary.txt", root),
        summary.as_bytes(),
        tid,
    )?;
    exported += 1;

    exported += export_runtime_key_tree(mount_id, &proc_sys_runtime_dir, tid)?;
    exported += export_abi_key_tree(mount_id, &proc_sys_abi_dir, tid)?;
    exported += export_library_key_tree(mount_id, &proc_sys_library_dir, tid)?;
    exported += export_compat_key_tree(mount_id, &proc_sys_compat_dir, tid)?;
    exported += export_feature_tree(mount_id, &proc_sys_features_dir, tid)?;

    Ok(exported)
}

#[cfg(feature = "vfs")]
fn normalize_export_root(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return "/compat".into();
    }
    if trimmed == "/" {
        return String::new();
    }
    let mut out = String::with_capacity(trimmed.len() + 1);
    if !trimmed.starts_with('/') {
        out.push('/');
    }
    out.push_str(trimmed.trim_end_matches('/'));
    out
}

#[cfg(feature = "vfs")]

fn ensure_dir(
    mount_id: usize,
    path: &str,
    tid: crate::interfaces::TaskId,
) -> Result<(), &'static str> {
    match crate::kernel::vfs_control::ramfs_mkdir(mount_id, path, tid) {
        Ok(()) | Err("already exists") => Ok(()),
        Err(err) => Err(err),
    }
}

#[cfg(feature = "vfs")]
pub fn write_file(
    mount_id: usize,
    path: &str,
    data: &[u8],
    tid: crate::interfaces::TaskId,
) -> Result<(), &'static str> {
    let mut file = match crate::kernel::vfs_control::ramfs_create_file(mount_id, path, tid) {
        Ok(file) => file,
        Err(_) => crate::kernel::vfs_control::ramfs_open_file(mount_id, path, tid)?,
    };
    let _ = file.seek(crate::modules::vfs::SeekFrom::Start(0));
    file.truncate(0)?;
    let wrote = file.write(data)?;
    file.flush()?;
    if wrote != data.len() {
        return Err("short compat surface write");
    }
    Ok(())
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

fn render_compat_virtual_file(path: &str) -> Result<String, &'static str> {
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

fn refresh_visible_compat_surface_after_write() {
    #[cfg(feature = "vfs")]
    {
        let _ = ensure_runtime_compat_surface_state();
    }
}

#[cfg(feature = "vfs")]
fn export_abi_key_tree(
    mount_id: usize,
    base_dir: &str,
    tid: crate::interfaces::TaskId,
) -> Result<usize, &'static str> {
    let keys = [
        "syscall_abi_magic",
        "syscall_abi_version_major",
        "syscall_abi_version_minor",
        "syscall_abi_version_patch",
        "syscall_abi_min_compat_major",
        "syscall_abi_stable",
        "syscall_platform",
        "syscall_page_size",
        "syscall_user_space_bottom",
        "syscall_user_space_top",
        "loader_gnu_hash_supported",
        "loader_runpath_supported",
        "loader_init_hooks_supported",
        "loader_fini_hooks_tracked",
        "runtime_vdso_supported",
        "runtime_vvar_supported",
        "runtime_tls_supported",
        "runtime_signal_abi_supported",
        "runtime_exec_env_supported",
        "runtime_multi_user_supported",
        "runtime_capability_model_supported",
        "startup_crt0_supported",
        "startup_argv_envp_supported",
        "startup_auxv_supported",
        "startup_runtime_contract_env_supported",
        "startup_stack_layout",
        "runtime_core_helpers",
        "runtime_core_memory_helpers",
        "runtime_core_string_helpers",
        "runtime_core_auxv_helpers",
        "runtime_core_env_helpers",
        "runtime_core_errno_features",
        "runtime_core_entrypoints",
        "runtime_core_source_units",
        "runtime_core_wrappers",
        "runtime_core_startup_features",
        "elf_class",
        "elf_endianness",
        "elf_machine_targets",
        "elf_loader_features",
        "elf_relocation_families",
        "elf_dynamic_tags",
        "libc_startup_capabilities",
        "libc_thread_capabilities",
        "libc_signal_capabilities",
        "libc_time_capabilities",
        "libc_fs_capabilities",
        "libc_memory_capabilities",
        "libc_string_capabilities",
        "libc_errno_model",
        "libc_planned_symbols",
        "libc_source_modules",
        "libc_syscall_surface",
        "startup_runtime_contract_env_keys",
        "startup_syscall_env_keys",
        "startup_auxv_env_keys",
        "auxv_at_base",
        "auxv_at_phdr",
        "auxv_at_phent",
        "auxv_at_phnum",
        "auxv_at_pagesz",
        "auxv_at_entry",
        "auxv_at_uid",
        "auxv_at_euid",
        "auxv_at_gid",
        "auxv_at_egid",
        "auxv_at_platform",
        "auxv_at_hwcap",
        "auxv_at_clktck",
        "auxv_at_secure",
        "auxv_at_random",
        "auxv_at_execfn",
        "auxv_at_sysinfo_ehdr",
        "syscall_read",
        "syscall_write",
        "syscall_openat",
        "syscall_close",
        "syscall_execve",
        "syscall_exit",
        "syscall_arch_prctl",
        "syscall_getpid",
        "syscall_getppid",
        "syscall_getuid",
        "syscall_geteuid",
        "syscall_getgid",
        "syscall_getegid",
        "syscall_clock_gettime",
        "syscall_gettimeofday",
        "syscall_time",
        "syscall_futex",
        "syscall_mmap",
        "syscall_munmap",
        "syscall_rt_sigaction",
        "syscall_rt_sigreturn",
    ];
    let mut exported = 0usize;
    for key in keys {
        let rendered = render_compat_config_key(key)?;
        let file_name = key.strip_prefix("syscall_").unwrap_or(key);
        write_file(
            mount_id,
            &format!("{}/{}", base_dir, file_name),
            rendered.as_bytes(),
            tid,
        )?;
        exported += 1;
    }
    Ok(exported)
}

#[cfg(feature = "vfs")]
fn export_runtime_key_tree(
    mount_id: usize,
    base_dir: &str,
    tid: crate::interfaces::TaskId,
) -> Result<usize, &'static str> {
    let mut exported = 0usize;
    for key in crate::config::KernelConfig::runtime_override_template() {
        let stem = key.split(' ').next().unwrap_or("").trim();
        if stem.is_empty() {
            continue;
        }
        if let Ok(rendered) = render_compat_config_key(stem) {
            write_file(
                mount_id,
                &format!("{}/{}", base_dir, stem),
                rendered.as_bytes(),
                tid,
            )?;
            exported += 1;
        }
    }
    Ok(exported)
}

#[cfg(feature = "vfs")]
fn export_library_key_tree(
    mount_id: usize,
    base_dir: &str,
    tid: crate::interfaces::TaskId,
) -> Result<usize, &'static str> {
    let keys = [
        "boundary_mode",
        "vfs_library_api_exposed",
        "network_library_api_exposed",
        "ipc_library_api_exposed",
        "proc_config_api_exposed",
        "sysctl_api_exposed",
        "security_enforcement_enabled",
        "capability_enforcement_enabled",
        "multi_user_enabled",
        "credential_enforcement_enabled",
    ];
    let mut exported = 0usize;
    for key in keys {
        let rendered = render_compat_config_key(key)?;
        write_file(
            mount_id,
            &format!("{}/{}", base_dir, key),
            rendered.as_bytes(),
            tid,
        )?;
        exported += 1;
    }
    Ok(exported)
}

#[cfg(feature = "vfs")]
fn export_compat_key_tree(
    mount_id: usize,
    base_dir: &str,
    tid: crate::interfaces::TaskId,
) -> Result<usize, &'static str> {
    let keys = [
        "compat_attack_surface_budget",
        "telemetry_enabled",
        "security_enforcement_enabled",
        "capability_enforcement_enabled",
        "multi_user_enabled",
        "credential_enforcement_enabled",
    ];
    let mut exported = 0usize;
    for key in keys {
        let rendered = render_compat_config_key(key)?;
        write_file(
            mount_id,
            &format!("{}/{}", base_dir, key),
            rendered.as_bytes(),
            tid,
        )?;
        exported += 1;
    }
    Ok(exported)
}

#[cfg(feature = "vfs")]
fn export_feature_tree(
    mount_id: usize,
    base_dir: &str,
    tid: crate::interfaces::TaskId,
) -> Result<usize, &'static str> {
    let mut exported = 0usize;
    for control in crate::config::KernelConfig::feature_controls() {
        let rendered = format!(
            "name={}\ncategory={}\ncompile_enabled={}\nruntime_gate={}\neffective_enabled={}\n",
            control.name,
            control.category,
            control.compile_enabled,
            control.runtime_gate_key.unwrap_or("-"),
            control.effective_enabled
        );
        let file_name = sanitize_path_component(control.name);
        write_file(
            mount_id,
            &format!("{}/{}", base_dir, file_name),
            rendered.as_bytes(),
            tid,
        )?;
        exported += 1;
    }
    Ok(exported)
}
