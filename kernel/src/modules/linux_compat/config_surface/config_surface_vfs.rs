use super::*;
#[cfg(feature = "vfs")]
use core::sync::atomic::Ordering;

pub fn mount_compat_surface_fs(mount_path: &str) -> Result<usize, &'static str> {
    let normalized_mount = normalize_mount_path(mount_path);
    crate::modules::vfs::mount_table::init_mount_table();
    match crate::kernel::vfs_control::mount_id_by_path(normalized_mount.as_bytes()) {
        Some(_) => refresh_compat_surface_fs(&normalized_mount),
        None => {
            crate::kernel::vfs_control::mount_ramfs(normalized_mount.as_bytes())
                .map_err(|_| "compat surface mount failed")?;
            register_mount_table_entry(&normalized_mount);
            refresh_compat_surface_fs(&normalized_mount)
        }
    }
}

#[cfg(feature = "vfs")]
pub fn refresh_compat_surface_fs(mount_path: &str) -> Result<usize, &'static str> {
    let normalized_mount = normalize_mount_path(mount_path);
    let exported = export_compat_surfaces_to_mount(&normalized_mount, "/")?;
    COMPAT_SURFACE_REFRESH_EPOCH.fetch_add(1, Ordering::Relaxed);
    Ok(exported)
}

#[cfg(feature = "vfs")]
pub fn ensure_runtime_compat_surface_state() -> Result<Option<usize>, &'static str> {
    let profile = crate::config::KernelConfig::compat_surface_profile();
    if !(profile.expose_linux_compat_surface
        || profile.expose_proc_config_api
        || profile.expose_sysctl_api)
    {
        hide_runtime_compat_surface(DEFAULT_COMPAT_SURFACE_MOUNT_PATH)?;
        return Ok(None);
    }
    mount_compat_surface_fs(DEFAULT_COMPAT_SURFACE_MOUNT_PATH).map(Some)
}

#[cfg(feature = "vfs")]
pub fn maybe_refresh_runtime_compat_surface(
    sample_tick: u64,
) -> Result<Option<usize>, &'static str> {
    if sample_tick % DEFAULT_COMPAT_SURFACE_REFRESH_INTERVAL_TICKS != 0 {
        return Ok(None);
    }

    let profile = crate::config::KernelConfig::compat_surface_profile();
    if !(profile.expose_linux_compat_surface
        || profile.expose_proc_config_api
        || profile.expose_sysctl_api)
    {
        return Ok(None);
    }

    refresh_compat_surface_fs(DEFAULT_COMPAT_SURFACE_MOUNT_PATH).map(Some)
}

#[cfg(feature = "vfs")]
pub fn compat_surface_refresh_epoch() -> u64 {
    COMPAT_SURFACE_REFRESH_EPOCH.load(Ordering::Relaxed)
}

#[cfg(feature = "vfs")]
pub fn hide_runtime_compat_surface(mount_path: &str) -> Result<(), &'static str> {
    let normalized_mount = normalize_mount_path(mount_path);
    if let Some(mount_id) =
        crate::kernel::vfs_control::mount_id_by_path(normalized_mount.as_bytes())
    {
        crate::kernel::vfs_control::unmount(mount_id)
            .map_err(|_| "compat surface unmount failed")?;
    }
    unregister_mount_table_entry(&normalized_mount);
    Ok(())
}

#[cfg(feature = "vfs")]
fn normalize_mount_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return "/compat".into();
    }
    let mut out = String::with_capacity(trimmed.len() + 1);
    if !trimmed.starts_with('/') {
        out.push('/');
    }
    out.push_str(trimmed.trim_end_matches('/'));
    if out.is_empty() {
        out.push('/');
    }
    out
}

#[cfg(feature = "vfs")]
fn register_mount_table_entry(mount_path: &str) {
    let fs_type = if mount_path == "/proc" || mount_path.starts_with("/proc/") {
        crate::modules::vfs::mount_table::FsType::Procfs
    } else {
        crate::modules::vfs::mount_table::FsType::Sysfs
    };
    let _ = crate::modules::vfs::mount_table::mount(
        mount_path,
        COMPAT_SURFACE_SOURCE_NAME,
        fs_type,
        crate::modules::vfs::mount_table::MountFlags::RDONLY
            | crate::modules::vfs::mount_table::MountFlags::NOEXEC
            | crate::modules::vfs::mount_table::MountFlags::NODEV,
    );
}

#[cfg(feature = "vfs")]
fn unregister_mount_table_entry(mount_path: &str) {
    let _ = crate::modules::vfs::mount_table::unmount(mount_path);
}

#[cfg(feature = "vfs")]

pub fn classify_compat_surface_mount_path(
    mount_path: &str,
) -> crate::modules::vfs::mount_table::FsType {
    if mount_path == "/proc" || mount_path.starts_with("/proc/") {
        crate::modules::vfs::mount_table::FsType::Procfs
    } else {
        crate::modules::vfs::mount_table::FsType::Sysfs
    }
}
