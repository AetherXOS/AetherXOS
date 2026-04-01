use super::*;
use alloc::format;

fn parse_bool_like(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "on" | "yes" | "enabled" => Some(true),
        "0" | "false" | "off" | "no" | "disabled" => Some(false),
        _ => None,
    }
}

pub fn apply_compat_config_key(key: &str, value: &str) -> Result<(), &'static str> {
    let normalized = normalize_runtime_style_key(key);
    if normalized.is_empty() {
        return Err("empty compat config key");
    }

    match normalized.as_str() {
        "linux_ptrace_compat_enabled" | "ptrace_compat_enabled" => {
            let enabled = if value.trim().is_empty() {
                true
            } else {
                parse_bool_like(value).ok_or("invalid bool for ptrace compat")?
            };
            crate::modules::linux_compat::config::set_ptrace_compat_enabled(enabled);
            return Ok(());
        }
        "linux_seccomp_compat_enabled" | "seccomp_compat_enabled" => {
            let enabled = if value.trim().is_empty() {
                true
            } else {
                parse_bool_like(value).ok_or("invalid bool for seccomp compat")?
            };
            crate::modules::linux_compat::config::set_seccomp_compat_enabled(enabled);
            return Ok(());
        }
        "linux_mman_soft_fallback_enabled" | "mman_soft_fallback_enabled" => {
            let enabled = if value.trim().is_empty() {
                cfg!(feature = "linux_shim_noop_mlock")
            } else {
                parse_bool_like(value).ok_or("invalid bool for mman soft fallback")?
            };
            crate::modules::linux_compat::config::set_mman_soft_fallback_enabled(enabled);
            return Ok(());
        }
        "linux_wayland_compat_enabled" | "wayland_compat_enabled" => {
            let enabled = if value.trim().is_empty() {
                true
            } else {
                parse_bool_like(value).ok_or("invalid bool for wayland compat")?
            };
            crate::modules::linux_compat::config::set_wayland_compat_enabled(enabled);
            return Ok(());
        }
        "linux_x11_compat_enabled" | "x11_compat_enabled" => {
            let enabled = if value.trim().is_empty() {
                true
            } else {
                parse_bool_like(value).ok_or("invalid bool for x11 compat")?
            };
            crate::modules::linux_compat::config::set_x11_compat_enabled(enabled);
            return Ok(());
        }
        _ => {}
    }

    let batch = if value.trim().is_empty() {
        format!("reset.{}", normalized)
    } else {
        format!("{}={}", normalized, value.trim())
    };
    crate::config::KernelConfig::apply_override_batch_str(&batch)
        .map(|_| ())
        .map_err(|_| "compat config apply failed")
}

pub fn compat_path_to_config_key(path: &str) -> Option<String> {
    let trimmed = path.trim();
    if matches!(
        trimmed,
        "/proc/hypercore/config"
            | "proc/hypercore/config"
            | "/proc/sys/hypercore/config"
            | "proc/sys/hypercore/config"
    ) {
        return None;
    }
    let relative = trimmed
        .strip_prefix("/proc/sys/hypercore/")
        .or_else(|| trimmed.strip_prefix("proc/sys/hypercore/"))
        .or_else(|| trimmed.strip_prefix("/sys/hypercore/"))
        .or_else(|| trimmed.strip_prefix("sys/hypercore/"))?;

    let normalized = normalize_runtime_style_key(relative);
    if normalized.is_empty() {
        return None;
    }

    let mapped = if let Some(feature) = normalized.strip_prefix("features_") {
        format!("feature_{}", feature)
    } else if let Some(runtime) = normalized.strip_prefix("runtime_") {
        runtime.to_string()
    } else if let Some(library) = normalized.strip_prefix("library_") {
        library.to_string()
    } else if let Some(abi) = normalized.strip_prefix("abi_") {
        format!("syscall_{}", abi)
    } else if let Some(compat) = normalized.strip_prefix("compat_") {
        format!("compat_{}", compat)
    } else {
        normalized
    };

    Some(mapped)
}

pub fn read_compat_config_path(path: &str) -> Result<String, &'static str> {
    render_compat_virtual_file(path)
}

pub fn write_compat_config_path(path: &str, value: &str) -> Result<(), &'static str> {
    let key = compat_path_to_config_key(path).ok_or("unsupported compat path")?;
    apply_compat_config_key(&key, value)?;
    refresh_visible_compat_surface_after_write();
    Ok(())
}

fn refresh_visible_compat_surface_after_write() {
    #[cfg(feature = "vfs")]
    {
        let _ = ensure_runtime_compat_surface_state();
    }
}
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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

#[allow(dead_code)]
fn sanitize_path_component(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        let c = ch.to_ascii_lowercase();
        if matches!(c, '/' | '\\' | ':' | ' ' | '.') {
            out.push('_');
        } else {
            out.push(c);
        }
    }
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    out.trim_matches('_').to_string()
}

fn normalize_runtime_style_key(key: &str) -> String {
    let trimmed = key.trim();
    let mut out = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        let c = ch.to_ascii_lowercase();
        if matches!(c, '.' | '-' | ' ' | '/' | ':') {
            out.push('_');
        } else {
            out.push(c);
        }
    }
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    out.trim_matches('_').to_string()
}

#[allow(dead_code)]
fn syscall_abi_platform() -> &'static str {
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
