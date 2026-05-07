#[cfg(feature = "vfs")]
use alloc::boxed::Box;
#[cfg(feature = "vfs")]
use alloc::sync::Arc;
#[cfg(feature = "vfs")]
use alloc::string::String;
#[cfg(feature = "vfs")]
use alloc::string::ToString;
#[cfg(feature = "vfs")]
use core::sync::atomic::Ordering;

#[cfg(feature = "vfs")]
use super::types::{MountEntry, MountError, MountFsKind, MountRecord};
#[cfg(feature = "vfs")]
use super::stats::{
    LAST_MOUNT_ID,
    MOUNT_ATTEMPTS, MOUNT_FAILURES, MOUNT_SUCCESS, NEXT_MOUNT_ID, PATH_VALIDATION_FAILURES,
    UNMOUNT_ATTEMPTS, UNMOUNT_BY_PATH_ATTEMPTS, UNMOUNT_BY_PATH_FAILURES,
    UNMOUNT_BY_PATH_SUCCESS, UNMOUNT_FAILURES, UNMOUNT_SUCCESS,
};
#[cfg(feature = "vfs")]
use super::registry::{MOUNT_REGISTRY, RAMFS_INSTANCES};
#[cfg(feature = "vfs")]
use super::support::{
    can_access_mount, current_task_id, normalize_mount_path, ROOT_TASK_ID,
};

#[cfg(feature = "vfs")]
pub fn mount_ramfs(path: &[u8]) -> Result<usize, MountError> {
    MOUNT_ATTEMPTS.fetch_add(1, Ordering::Relaxed);

    let Some(normalized) =
        normalize_mount_path(path, crate::config::KernelConfig::vfs_max_mount_path())
    else {
        PATH_VALIDATION_FAILURES.fetch_add(1, Ordering::Relaxed);
        MOUNT_FAILURES.fetch_add(1, Ordering::Relaxed);
        return Err(MountError::InvalidPath);
    };
    let path_len = normalized.len();

    let mut registry = MOUNT_REGISTRY.lock();
    if registry.len() >= crate::config::KernelConfig::vfs_max_mounts() {
        MOUNT_FAILURES.fetch_add(1, Ordering::Relaxed);
        return Err(MountError::RegistryFull);
    }

    if registry
        .iter()
        .any(|e| e.path_len == path_len && e.path == normalized)
    {
        MOUNT_FAILURES.fetch_add(1, Ordering::Relaxed);
        return Err(MountError::AlreadyMounted);
    }

    let mount_id = NEXT_MOUNT_ID.fetch_add(1, Ordering::Relaxed);
    let tid = current_task_id();

    registry.push(MountEntry {
        id: mount_id,
        fs_kind: MountFsKind::RamFs,
        path: normalized,
        path_len,
        owner: tid,
        readonly: false,
    });
    drop(registry);

    RAMFS_INSTANCES
        .lock()
        .push((mount_id, Box::new(crate::modules::vfs::RamFs::new())));

    LAST_MOUNT_ID.store(mount_id, Ordering::Relaxed);
    MOUNT_SUCCESS.fetch_add(1, Ordering::Relaxed);
    Ok(mount_id)
}

#[cfg(feature = "vfs")]
pub fn mount_diskfs(path: &[u8], fs_kind: MountFsKind, readonly: bool) -> Result<usize, MountError> {
    if !matches!(fs_kind, MountFsKind::Ext4 | MountFsKind::Fat32) {
        return Err(MountError::InvalidPath);
    }

    MOUNT_ATTEMPTS.fetch_add(1, Ordering::Relaxed);

    let Some(normalized) =
        normalize_mount_path(path, crate::config::KernelConfig::vfs_max_mount_path())
    else {
        PATH_VALIDATION_FAILURES.fetch_add(1, Ordering::Relaxed);
        MOUNT_FAILURES.fetch_add(1, Ordering::Relaxed);
        return Err(MountError::InvalidPath);
    };
    let path_len = normalized.len();

    let mut registry = MOUNT_REGISTRY.lock();
    if registry.len() >= crate::config::KernelConfig::vfs_max_mounts() {
        MOUNT_FAILURES.fetch_add(1, Ordering::Relaxed);
        return Err(MountError::RegistryFull);
    }

    if registry
        .iter()
        .any(|e| e.path_len == path_len && e.path == normalized)
    {
        MOUNT_FAILURES.fetch_add(1, Ordering::Relaxed);
        return Err(MountError::AlreadyMounted);
    }

    let mount_id = NEXT_MOUNT_ID.fetch_add(1, Ordering::Relaxed);
    let tid = current_task_id();

    registry.push(MountEntry {
        id: mount_id,
        fs_kind,
        path: normalized,
        path_len,
        owner: tid,
        readonly,
    });

    LAST_MOUNT_ID.store(mount_id, Ordering::Relaxed);
    MOUNT_SUCCESS.fetch_add(1, Ordering::Relaxed);
    Ok(mount_id)
}

#[cfg(feature = "vfs")]
pub fn mount_overlay(path: &[u8], lower_fs_kind: MountFsKind, readonly_upper: bool) -> Result<usize, MountError> {
    if !matches!(lower_fs_kind, MountFsKind::Ext4 | MountFsKind::Fat32) {
        return Err(MountError::InvalidPath);
    }

    MOUNT_ATTEMPTS.fetch_add(1, Ordering::Relaxed);

    let Some(normalized) =
        normalize_mount_path(path, crate::config::KernelConfig::vfs_max_mount_path())
    else {
        PATH_VALIDATION_FAILURES.fetch_add(1, Ordering::Relaxed);
        MOUNT_FAILURES.fetch_add(1, Ordering::Relaxed);
        return Err(MountError::InvalidPath);
    };
    let path_len = normalized.len();

    let mut registry = MOUNT_REGISTRY.lock();
    if registry.len() >= crate::config::KernelConfig::vfs_max_mounts() {
        MOUNT_FAILURES.fetch_add(1, Ordering::Relaxed);
        return Err(MountError::RegistryFull);
    }

    if registry
        .iter()
        .any(|e| e.path_len == path_len && e.path == normalized)
    {
        MOUNT_FAILURES.fetch_add(1, Ordering::Relaxed);
        return Err(MountError::AlreadyMounted);
    }

    let mount_id = NEXT_MOUNT_ID.fetch_add(1, Ordering::Relaxed);
    let tid = current_task_id();

    registry.push(MountEntry {
        id: mount_id,
        fs_kind: MountFsKind::Overlay,
        path: normalized,
        path_len,
        owner: tid,
        readonly: readonly_upper,
    });

    LAST_MOUNT_ID.store(mount_id, Ordering::Relaxed);
    MOUNT_SUCCESS.fetch_add(1, Ordering::Relaxed);

    // Overlay instantiation logic
    {
        let lower_path_opt: Option<String> = {
            let reg = MOUNT_REGISTRY.lock();
            reg.iter()
                .find(|e| e.fs_kind == lower_fs_kind)
                .and_then(|e| core::str::from_utf8(&e.path).ok().map(|s| s.to_string()))
        };

        let mut instantiated = false;

        if let Some(lower_path) = lower_path_opt {
            #[cfg(feature = "vfs_disk_fs")]
            {
                if let Ok(base) = crate::modules::vfs::disk_fs::DiskFsLibrary::attach_existing(&lower_path) {
                    #[cfg(feature = "drivers")]
                    {
                        let adapter = crate::modules::vfs::writable_fs::StorageManagerBlockAdapter::new();
                        let block_sink = crate::modules::vfs::BlockWritebackSink::new(Box::new(adapter), 4);
                        let sink: Arc<dyn crate::modules::vfs::writeback::WritebackSink> = Arc::new(block_sink);
                        let overlay = crate::modules::vfs::WritableOverlayFs::new(base, mount_id, sink.clone());
                        let upper = crate::modules::vfs::tmpfs::TmpFs::new();
                        crate::modules::vfs::overlay_registry::register_overlay_with_upper(
                            mount_id,
                            Box::new(overlay),
                            Some(Box::new(upper)),
                        );
                        instantiated = true;
                    }
                    #[cfg(not(feature = "drivers"))]
                    {
                        let sink: Arc<dyn crate::modules::vfs::writeback::WritebackSink> =
                            Arc::new(crate::modules::vfs::RamWritebackSink::new());
                        let overlay = crate::modules::vfs::WritableOverlayFs::new(base, mount_id, sink.clone());
                        let upper = crate::modules::vfs::tmpfs::TmpFs::new();
                        crate::modules::vfs::overlay_registry::register_overlay_with_upper(
                            mount_id,
                            Box::new(overlay),
                            Some(Box::new(upper)),
                        );
                        instantiated = true;
                    }
                }
            }
        }

        if !instantiated {
            let base = crate::modules::vfs::tmpfs::TmpFs::new();
            let sink: Arc<dyn crate::modules::vfs::writeback::WritebackSink> =
                Arc::new(crate::modules::vfs::RamWritebackSink::new());
            let overlay = crate::modules::vfs::WritableOverlayFs::new(base, mount_id, sink.clone());
            crate::modules::vfs::overlay_registry::register_overlay(mount_id, Box::new(overlay));
        }
    }

    Ok(mount_id)
}

#[cfg(feature = "vfs")]
pub fn unmount(mount_id: usize) -> Result<(), MountError> {
    UNMOUNT_ATTEMPTS.fetch_add(1, Ordering::Relaxed);

    let tid = current_task_id();

    let removed = {
        let mut registry = MOUNT_REGISTRY.lock();
        if let Some(index) = registry.iter().position(|entry| entry.id == mount_id) {
            let entry = &registry[index];
            if entry.owner != tid && tid != ROOT_TASK_ID {
                UNMOUNT_FAILURES.fetch_add(1, Ordering::Relaxed);
                return Err(MountError::MountNotFound);
            }
            registry.remove(index);
            true
        } else {
            false
        }
    };

    if !removed {
        UNMOUNT_FAILURES.fetch_add(1, Ordering::Relaxed);
        return Err(MountError::MountNotFound);
    }

    let mut instances = RAMFS_INSTANCES.lock();
    if let Some(index) = instances.iter().position(|(id, _)| *id == mount_id) {
        instances.remove(index);
    }

    let _ = crate::modules::vfs::overlay_registry::unregister_overlay(mount_id);

    UNMOUNT_SUCCESS.fetch_add(1, Ordering::Relaxed);
    Ok(())
}

#[cfg(feature = "vfs")]
pub fn unmount_by_path(path: &[u8]) -> Result<(), MountError> {
    UNMOUNT_BY_PATH_ATTEMPTS.fetch_add(1, Ordering::Relaxed);

    let Some(normalized) =
        normalize_mount_path(path, crate::config::KernelConfig::vfs_max_mount_path())
    else {
        PATH_VALIDATION_FAILURES.fetch_add(1, Ordering::Relaxed);
        UNMOUNT_BY_PATH_FAILURES.fetch_add(1, Ordering::Relaxed);
        return Err(MountError::InvalidPath);
    };
    let path_len = normalized.len();

    let mount_id = {
        let registry = MOUNT_REGISTRY.lock();
        let Some(entry) = registry
            .iter()
            .find(|entry| entry.path_len == path_len && entry.path == normalized)
        else {
            UNMOUNT_BY_PATH_FAILURES.fetch_add(1, Ordering::Relaxed);
            return Err(MountError::MountNotFound);
        };
        entry.id
    };

    match unmount(mount_id) {
        Ok(()) => {
            UNMOUNT_BY_PATH_SUCCESS.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
        Err(err) => {
            UNMOUNT_BY_PATH_FAILURES.fetch_add(1, Ordering::Relaxed);
            Err(err)
        }
    }
}

#[cfg(feature = "vfs")]
pub fn list_mounts(out: &mut [MountRecord]) -> usize {
    let registry = MOUNT_REGISTRY.lock();
    let tid = current_task_id();
    let mut written = 0usize;

    for entry in registry.iter() {
        if !can_access_mount(entry.owner, tid) {
            continue;
        }
        if written >= out.len() {
            break;
        }
        out[written] = MountRecord {
            id: entry.id,
            fs_kind: entry.fs_kind as usize,
            path_len: entry.path_len,
        };
        written += 1;
    }

    written
}

#[cfg(feature = "vfs")]
pub fn mount_path_by_id(mount_id: usize, out: &mut [u8]) -> Option<usize> {
    let registry = MOUNT_REGISTRY.lock();
    let tid = current_task_id();
    let entry = registry.iter().find(|entry| entry.id == mount_id)?;
    if !can_access_mount(entry.owner, tid) {
        return None;
    }
    if out.len() < entry.path_len {
        return None;
    }
    out[..entry.path_len].copy_from_slice(&entry.path);
    Some(entry.path_len)
}

#[cfg(feature = "vfs")]
pub fn mount_id_by_path(path: &[u8]) -> Option<usize> {
    let Some(normalized) =
        normalize_mount_path(path, crate::config::KernelConfig::vfs_max_mount_path())
    else {
        PATH_VALIDATION_FAILURES.fetch_add(1, Ordering::Relaxed);
        return None;
    };
    let path_len = normalized.len();

    let registry = MOUNT_REGISTRY.lock();
    let tid = current_task_id();
    registry
        .iter()
        .find(|entry| {
            can_access_mount(entry.owner, tid)
                && entry.path_len == path_len
                && entry.path == normalized
        })
        .map(|entry| entry.id)
}

#[cfg(feature = "vfs")]
pub fn relocate_mount(mount_id: usize, new_path: &[u8]) -> Result<(), MountError> {
    let Some(normalized) =
        normalize_mount_path(new_path, crate::config::KernelConfig::vfs_max_mount_path())
    else {
        PATH_VALIDATION_FAILURES.fetch_add(1, Ordering::Relaxed);
        return Err(MountError::InvalidPath);
    };

    let new_len = normalized.len();
    let tid = current_task_id();
    let mut registry = MOUNT_REGISTRY.lock();
    let Some(index) = registry.iter().position(|entry| entry.id == mount_id) else {
        return Err(MountError::MountNotFound);
    };
    if !can_access_mount(registry[index].owner, tid) {
        return Err(MountError::MountNotFound);
    }

    if registry
        .iter()
        .enumerate()
        .any(|(i, e)| i != index && e.path_len == new_len && e.path == normalized)
    {
        return Err(MountError::AlreadyMounted);
    }

    registry[index].path = normalized;
    registry[index].path_len = new_len;
    Ok(())
}

#[cfg(feature = "vfs")]
pub fn set_mount_readonly(mount_id: usize, readonly: bool) -> Result<(), MountError> {
    let tid = current_task_id();
    let mut registry = MOUNT_REGISTRY.lock();
    let Some(entry) = registry.iter_mut().find(|entry| entry.id == mount_id) else {
        return Err(MountError::MountNotFound);
    };
    if !can_access_mount(entry.owner, tid) {
        return Err(MountError::MountNotFound);
    }
    entry.readonly = readonly;
    Ok(())
}

#[cfg(feature = "vfs")]
pub fn mount_readonly_by_path(path: &[u8]) -> Option<bool> {
    let normalized = normalize_mount_path(path, crate::config::KernelConfig::vfs_max_mount_path())?;
    let path_len = normalized.len();
    let tid = current_task_id();
    let registry = MOUNT_REGISTRY.lock();
    registry
        .iter()
        .find(|entry| {
            can_access_mount(entry.owner, tid)
                && entry.path_len == path_len
                && entry.path == normalized
        })
        .map(|entry| entry.readonly)
}

#[cfg(feature = "vfs")]
pub fn mount_readonly_by_id(mount_id: usize) -> Option<bool> {
    let tid = current_task_id();
    let registry = MOUNT_REGISTRY.lock();
    registry
        .iter()
        .find(|entry| can_access_mount(entry.owner, tid) && entry.id == mount_id)
        .map(|entry| entry.readonly)
}
