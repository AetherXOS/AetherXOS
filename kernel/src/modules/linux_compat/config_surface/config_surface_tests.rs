use super::*;

#[test_case]
fn proc_surface_respects_boundary_policy() {
    crate::config::KernelConfig::reset_runtime_overrides();
    crate::config::KernelConfig::set_vfs_library_api_exposed(Some(true));
    crate::config::KernelConfig::set_proc_config_api_exposed(Some(true));
    crate::config::KernelConfig::set_library_boundary_mode(Some(
        crate::config::BoundaryMode::Balanced,
    ));

    let rendered = render_proc_config_snapshot().expect("proc surface should render");
    assert!(rendered.contains("surface=proc_config"));
    assert!(rendered.contains("proc_config_api=true"));

    crate::config::KernelConfig::set_library_boundary_mode(Some(
        crate::config::BoundaryMode::Strict,
    ));
    assert!(render_proc_config_snapshot().is_err());

    crate::config::KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn exportable_surfaces_report_expected_kinds() {
    let surfaces = exportable_compat_surfaces();
    assert_eq!(surfaces.len(), 2);
    assert!(surfaces
        .iter()
        .any(|surface| surface.kind == CompatConfigSurfaceKind::ProcConfig));
    assert!(surfaces
        .iter()
        .any(|surface| surface.kind == CompatConfigSurfaceKind::Sysctl));
}

#[test_case]
fn abi_keys_render_through_compat_surface() {
    let rendered = render_compat_config_key("syscall_openat").expect("syscall key should render");
    assert_eq!(
        rendered.trim(),
        alloc::format!("{}", crate::kernel::syscalls::linux_nr::OPENAT)
    );
    assert_eq!(
        compat_path_to_config_key("/proc/sys/aethercore/abi/openat").as_deref(),
        Some("syscall_openat")
    );
    assert_eq!(
        render_compat_config_key("runtime_vdso_supported").as_deref(),
        Ok("1\n")
    );
    assert_eq!(
        render_compat_config_key("auxv_at_execfn").as_deref(),
        Ok("31\n")
    );
    assert_eq!(
        render_compat_config_key("startup_stack_layout").as_deref(),
        Ok("argc|argv|null|envp|null|auxv\n")
    );
    assert!(render_compat_config_key("runtime_core_helpers")
        .expect("runtime core helpers should render")
        .contains("print_u64_decimal"));
    assert!(render_compat_config_key("runtime_core_wrappers")
        .expect("runtime core wrappers should render")
        .contains("rt_sigreturn"));
    assert!(render_compat_config_key("runtime_core_memory_helpers")
        .expect("runtime core memory helpers should render")
        .contains("memcpy_contract"));
    assert!(render_compat_config_key("runtime_core_helpers")
        .expect("runtime core helpers should render")
        .contains("runtime_probe_mask"));
    assert!(render_compat_config_key("runtime_core_helpers")
        .expect("runtime core helpers should render")
        .contains("runtime_probe_status_word"));
    assert!(render_compat_config_key("runtime_core_helpers")
        .expect("runtime core helpers should render")
        .contains("runtime_probe_summary"));
    assert!(render_compat_config_key("runtime_core_auxv_helpers")
        .expect("runtime core auxv helpers should render")
        .contains("auxv_presence_checks"));
    assert!(render_compat_config_key("runtime_core_env_helpers")
        .expect("runtime core env helpers should render")
        .contains("last_env_name_tracking"));
    assert!(render_compat_config_key("runtime_core_startup_features")
        .expect("runtime core startup features should render")
        .contains("runtime_probe_mask_report"));
    assert!(render_compat_config_key("runtime_core_startup_features")
        .expect("runtime core startup features should render")
        .contains("runtime_probe_summary_report"));
    assert!(render_compat_config_key("runtime_core_entrypoints")
        .expect("runtime core entrypoints should render")
        .contains("__aethercore_crt0_start"));
    assert!(render_compat_config_key("runtime_core_source_units")
        .expect("runtime core source units should render")
        .contains("runtime_probe.c"));
    assert!(render_compat_config_key("runtime_core_source_units")
        .expect("runtime core source units should render")
        .contains("runtime_smoke.c"));
    assert!(render_compat_config_key("elf_loader_features")
        .expect("elf loader features should render")
        .contains("dt_init_array"));
    assert!(render_compat_config_key("libc_thread_capabilities")
        .expect("libc thread capabilities should render")
        .contains("robust_list_tracking"));
    assert!(render_compat_config_key("libc_errno_model")
        .expect("libc errno model should render")
        .contains("negative_errno_syscall_return"));
    assert!(render_compat_config_key("libc_planned_symbols")
        .expect("libc planned symbols should render")
        .contains("__libc_start_main"));
    assert!(render_compat_config_key("libc_source_modules")
        .expect("libc source modules should render")
        .contains("libc_state.c"));
    assert!(render_compat_config_key("startup_syscall_env_keys")
        .expect("startup syscall env keys should render")
        .contains("AETHERCORE_SYSCALL_FUTEX"));
}

#[test_case]
fn runtime_toggle_keys_round_trip_through_compat_surface() {
    assert!(apply_compat_config_key("linux_ptrace_compat_enabled", "0").is_ok());
    assert_eq!(
        render_compat_config_key("linux_ptrace_compat_enabled").as_deref(),
        Ok("false\n")
    );

    assert!(apply_compat_config_key("linux_seccomp_compat_enabled", "1").is_ok());
    assert_eq!(
        render_compat_config_key("linux_seccomp_compat_enabled").as_deref(),
        Ok("true\n")
    );

    assert!(apply_compat_config_key("linux_mman_soft_fallback_enabled", "yes").is_ok());
    assert_eq!(
        render_compat_config_key("linux_mman_soft_fallback_enabled").as_deref(),
        Ok("true\n")
    );

    assert!(apply_compat_config_key("linux_wayland_compat_enabled", "0").is_ok());
    assert_eq!(
        render_compat_config_key("linux_wayland_compat_enabled").as_deref(),
        Ok("false\n")
    );

    assert!(apply_compat_config_key("linux_x11_compat_enabled", "1").is_ok());
    assert_eq!(
        render_compat_config_key("linux_x11_compat_enabled").as_deref(),
        Ok("true\n")
    );

    // Restore defaults for follow-up tests.
    let _ = apply_compat_config_key("linux_ptrace_compat_enabled", "1");
    let _ = apply_compat_config_key("linux_seccomp_compat_enabled", "1");
    let _ = apply_compat_config_key("linux_mman_soft_fallback_enabled", "");
    let _ = apply_compat_config_key("linux_wayland_compat_enabled", "1");
    let _ = apply_compat_config_key("linux_x11_compat_enabled", "1");
}

#[test_case]
fn syscall_surface_renders_multiple_core_numbers() {
    let openat = render_compat_config_key("syscall_openat").expect("openat should render");
    let getpid = render_compat_config_key("syscall_getpid").expect("getpid should render");
    let mmap = render_compat_config_key("syscall_mmap").expect("mmap should render");
    let sigreturn =
        render_compat_config_key("syscall_rt_sigreturn").expect("rt_sigreturn should render");

    assert_eq!(
        openat.trim(),
        alloc::format!("{}", crate::kernel::syscalls::linux_nr::OPENAT)
    );
    assert_eq!(
        getpid.trim(),
        alloc::format!("{}", crate::kernel::syscalls::linux_nr::GETPID)
    );
    assert_eq!(
        mmap.trim(),
        alloc::format!("{}", crate::kernel::syscalls::linux_nr::MMAP)
    );
    assert_eq!(
        sigreturn.trim(),
        alloc::format!("{}", crate::kernel::syscalls::linux_nr::RT_SIGRETURN)
    );

    let env_keys =
        render_compat_config_key("startup_syscall_env_keys").expect("syscall env keys should render");
    assert!(env_keys.contains("AETHERCORE_SYSCALL_OPENAT"));
    assert!(env_keys.contains("AETHERCORE_SYSCALL_GETPID"));
    assert!(env_keys.contains("AETHERCORE_SYSCALL_RT_SIGRETURN"));
}

#[test_case]
fn wayland_x11_runtime_toggles_round_trip_through_paths() {
    assert!(write_compat_config_path(
        "/proc/sys/aethercore/compat/wayland_compat_enabled",
        "0"
    )
    .is_ok());
    assert!(!crate::modules::linux_compat::config::wayland_compat_enabled());
    assert_eq!(
        render_compat_config_key("linux_wayland_compat_enabled").as_deref(),
        Ok("false\n")
    );

    assert!(write_compat_config_path("/sys/aethercore/compat/x11_compat_enabled", "0").is_ok());
    assert!(!crate::modules::linux_compat::config::x11_compat_enabled());
    assert_eq!(
        render_compat_config_key("linux_x11_compat_enabled").as_deref(),
        Ok("false\n")
    );

    // Empty value resets compat toggles to their default-on behavior.
    assert!(write_compat_config_path(
        "/proc/sys/aethercore/compat/wayland_compat_enabled",
        ""
    )
    .is_ok());
    assert!(write_compat_config_path("/sys/aethercore/compat/x11_compat_enabled", "").is_ok());
    assert!(crate::modules::linux_compat::config::wayland_compat_enabled());
    assert!(crate::modules::linux_compat::config::x11_compat_enabled());
}

#[test_case]
fn wayland_x11_invalid_bool_values_are_rejected() {
    crate::modules::linux_compat::config::set_wayland_compat_enabled(true);
    crate::modules::linux_compat::config::set_x11_compat_enabled(true);

    assert!(apply_compat_config_key("linux_wayland_compat_enabled", "maybe").is_err());
    assert!(apply_compat_config_key("linux_x11_compat_enabled", "2").is_err());

    assert!(
        crate::modules::linux_compat::config::wayland_compat_enabled(),
        "failed writes must not mutate wayland runtime toggle"
    );
    assert!(
        crate::modules::linux_compat::config::x11_compat_enabled(),
        "failed writes must not mutate x11 runtime toggle"
    );
}

#[cfg(feature = "vfs")]
#[test_case]
fn compat_mount_path_classification_matches_expected_fs_type() {
    assert_eq!(
        classify_compat_surface_mount_path("/proc"),
        crate::modules::vfs::mount_table::FsType::Procfs
    );
    assert_eq!(
        classify_compat_surface_mount_path("/proc/sys/aethercore"),
        crate::modules::vfs::mount_table::FsType::Procfs
    );
    assert_eq!(
        classify_compat_surface_mount_path("/sys/aethercore"),
        crate::modules::vfs::mount_table::FsType::Sysfs
    );
}
