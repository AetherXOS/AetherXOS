use super::*;
use core::sync::atomic::Ordering;

#[allow(dead_code)]
pub(crate) fn sys_vfs_mount_ramfs(_path_ptr: usize, _path_len: usize) -> usize {
    SYSCALL_VFS_MOUNT_RAMFS_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) = require_control_plane_access(crate::modules::security::RESOURCE_VFS_MOUNT) {
        return err;
    }

    #[cfg(feature = "vfs")]
    {
        with_user_vfs_path(_path_ptr, _path_len, |path| {
            match crate::kernel::vfs_control::mount_ramfs(path) {
                Ok(mount_id) => mount_id,
                Err(_) => invalid_arg(),
            }
        })
        .unwrap_or_else(|err| err)
    }

    #[cfg(not(feature = "vfs"))]
    {
        invalid_arg()
    }
}

#[allow(dead_code)]
pub(crate) fn sys_vfs_mount_diskfs(
    _path_ptr: usize,
    _path_len: usize,
    _fs_kind: usize,
    _flags: usize,
) -> usize {
    SYSCALL_VFS_MOUNT_DISKFS_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) = require_control_plane_access(crate::modules::security::RESOURCE_VFS_MOUNT) {
        return err;
    }

    #[cfg(feature = "vfs")]
    {
        with_user_vfs_path(_path_ptr, _path_len, |path| {
            let fs_kind = match _fs_kind {
                2 => crate::kernel::vfs_control::MountFsKind::Ext4,
                3 => crate::kernel::vfs_control::MountFsKind::Fat32,
                _ => return invalid_arg(),
            };
            let readonly = (_flags & 0x1) != 0;
            match crate::kernel::vfs_control::mount_diskfs(path, fs_kind, readonly) {
                Ok(mount_id) => mount_id,
                Err(_) => invalid_arg(),
            }
        })
        .unwrap_or_else(|err| err)
    }

    #[cfg(not(feature = "vfs"))]
    {
        invalid_arg()
    }
}

#[allow(dead_code)]
pub(crate) fn sys_vfs_mount_overlay(
    _path_ptr: usize,
    _path_len: usize,
    _lower_fs_kind: usize,
    _flags: usize,
) -> usize {
    SYSCALL_VFS_MOUNT_OVERLAY_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) = require_control_plane_access(crate::modules::security::RESOURCE_VFS_MOUNT) {
        return err;
    }

    #[cfg(feature = "vfs")]
    {
        with_user_vfs_path(_path_ptr, _path_len, |path| {
            let lower = match _lower_fs_kind {
                2 => crate::kernel::vfs_control::MountFsKind::Ext4,
                3 => crate::kernel::vfs_control::MountFsKind::Fat32,
                _ => return invalid_arg(),
            };
            let readonly_upper = (_flags & 0x1) != 0;
            match crate::kernel::vfs_control::mount_overlay(path, lower, readonly_upper) {
                Ok(mount_id) => mount_id,
                Err(_) => invalid_arg(),
            }
        })
        .unwrap_or_else(|err| err)
    }

    #[cfg(not(feature = "vfs"))]
    {
        invalid_arg()
    }
}

#[allow(dead_code)]
pub(crate) fn sys_vfs_list_mounts(_ptr: usize, len: usize) -> usize {
    SYSCALL_VFS_LIST_MOUNTS_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) = require_control_plane_access(crate::modules::security::RESOURCE_VFS_LIST) {
        return err;
    }

    if len == 0 {
        return invalid_arg();
    }

    let entry_bytes = MOUNT_RECORD_WORDS * core::mem::size_of::<usize>();
    let capacity = len / entry_bytes;
    if capacity == 0 {
        return invalid_arg();
    }

    #[cfg(feature = "vfs")]
    {
        let words_len = capacity * MOUNT_RECORD_WORDS;
        with_user_write_words_exact(_ptr, len, words_len, |out| {
            let mut records = alloc::vec![crate::kernel::vfs_control::MountRecord { id: 0, fs_kind: 0, path_len: 0 }; capacity];
            let written = crate::kernel::vfs_control::list_mounts(&mut records);

            for (idx, rec) in records.iter().take(written).enumerate() {
                let base = idx * MOUNT_RECORD_WORDS;
                write_mount_record_words(out, base, rec);
            }

            written
        })
        .unwrap_or_else(|err| err)
    }

    #[cfg(not(feature = "vfs"))]
    {
        invalid_arg()
    }
}

#[allow(dead_code)]
pub(crate) fn sys_vfs_get_mount_path(_mount_id: usize, _ptr: usize, _len: usize) -> usize {
    SYSCALL_VFS_MOUNT_PATH_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) = require_control_plane_access(crate::modules::security::RESOURCE_VFS_PATH) {
        return err;
    }

    #[cfg(feature = "vfs")]
    {
        with_user_write_bytes(
            _ptr,
            _len,
            |out| match crate::kernel::vfs_control::mount_path_by_id(_mount_id, out) {
                Some(path_len) => path_len,
                None => invalid_arg(),
            },
        )
        .unwrap_or_else(|err| err)
    }

    #[cfg(not(feature = "vfs"))]
    {
        invalid_arg()
    }
}

#[allow(dead_code)]
pub(crate) fn sys_vfs_unmount(_mount_id: usize) -> usize {
    SYSCALL_VFS_UNMOUNT_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) = require_control_plane_access(crate::modules::security::RESOURCE_VFS_UNMOUNT) {
        return err;
    }

    #[cfg(feature = "vfs")]
    {
        match crate::kernel::vfs_control::unmount(_mount_id) {
            Ok(()) => 0,
            Err(_) => invalid_arg(),
        }
    }

    #[cfg(not(feature = "vfs"))]
    {
        invalid_arg()
    }
}

#[allow(dead_code)]
pub(crate) fn sys_vfs_get_stats(_ptr: usize, _len: usize) -> usize {
    SYSCALL_VFS_STATS_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) = require_control_plane_access(crate::modules::security::RESOURCE_VFS_STATS) {
        return err;
    }

    #[cfg(feature = "vfs")]
    {
        let vfs = crate::kernel::vfs_control::stats();
        write_user_words(
            _ptr,
            _len,
            [
                vfs.mount_attempts as usize,
                vfs.mount_success as usize,
                vfs.mount_failures as usize,
                vfs.unmount_attempts as usize,
                vfs.unmount_success as usize,
                vfs.unmount_failures as usize,
                vfs.path_validation_failures as usize,
                vfs.total_mounts,
                vfs.last_mount_id,
                vfs.unmount_by_path_attempts as usize,
                vfs.unmount_by_path_success as usize,
                vfs.unmount_by_path_failures as usize,
            ],
        )
    }

    #[cfg(not(feature = "vfs"))]
    {
        invalid_arg()
    }
}

#[allow(dead_code)]
pub(crate) fn sys_vfs_unmount_path(_path_ptr: usize, _path_len: usize) -> usize {
    SYSCALL_VFS_UNMOUNT_PATH_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) = require_control_plane_access(crate::modules::security::RESOURCE_VFS_UNMOUNT) {
        return err;
    }

    #[cfg(feature = "vfs")]
    {
        with_user_vfs_path(_path_ptr, _path_len, |path| {
            match crate::kernel::vfs_control::unmount_by_path(path) {
                Ok(()) => 0,
                Err(_) => invalid_arg(),
            }
        })
        .unwrap_or_else(|err| err)
    }

    #[cfg(not(feature = "vfs"))]
    {
        invalid_arg()
    }
}
