use super::vfs_constants::*;

#[derive(Debug, Clone, Copy)]
pub(super) struct VfsOperationSupportRow {
    pub(super) operation: &'static str,
    pub(super) default_trait: &'static str,
    pub(super) ramfs: &'static str,
    pub(super) tmpfs: &'static str,
    pub(super) devfs: &'static str,
    pub(super) procfs: &'static str,
    pub(super) sysfs: &'static str,
    pub(super) disk_fs: &'static str,
    pub(super) writable_overlay: &'static str,
}

impl VfsOperationSupportRow {
    pub(super) fn status_for_fs(&self, fs_name: &str) -> &'static str {
        match fs_name {
            VFS_FS_RAMFS => self.ramfs,
            VFS_FS_TMPFS => self.tmpfs,
            VFS_FS_DEVFS => self.devfs,
            VFS_FS_PROCFS => self.procfs,
            VFS_FS_SYSFS => self.sysfs,
            VFS_FS_DISK_FS => self.disk_fs,
            VFS_FS_WRITABLE_OVERLAY => self.writable_overlay,
            _ => VFS_STATUS_UNSUPPORTED,
        }
    }
}

pub(super) fn vfs_status_weight(status: &str) -> u32 {
    match status {
        VFS_STATUS_FULL => VFS_STATUS_WEIGHT_FULL,
        VFS_STATUS_PARTIAL => VFS_STATUS_WEIGHT_PARTIAL,
        VFS_STATUS_READ_MOSTLY => VFS_STATUS_WEIGHT_READ_MOSTLY,
        VFS_STATUS_FEATURE_GATED => VFS_STATUS_WEIGHT_FEATURE_GATED,
        VFS_STATUS_BACKEND_SPECIFIC => VFS_STATUS_WEIGHT_BACKEND_SPECIFIC,
        VFS_STATUS_REMOVABLE_ONLY => VFS_STATUS_WEIGHT_REMOVABLE_ONLY,
        VFS_STATUS_UNSUPPORTED => VFS_STATUS_WEIGHT_UNSUPPORTED,
        _ => VFS_STATUS_WEIGHT_DEFAULT,
    }
}

pub(super) fn vfs_operation_weight(default_trait: &str) -> u32 {
    match default_trait {
        VFS_TRAIT_REQUIRED => VFS_OPERATION_WEIGHT_REQUIRED,
        VFS_TRAIT_OPTIONAL => VFS_OPERATION_WEIGHT_OPTIONAL,
        VFS_TRAIT_UNLOCK_ONLY => VFS_OPERATION_WEIGHT_UNLOCK_ONLY,
        _ => VFS_OPERATION_WEIGHT_DEFAULT,
    }
}

pub(super) fn vfs_readiness_band(score: u32) -> &'static str {
    if score >= VFS_READINESS_STRONG_THRESHOLD {
        "strong"
    } else if score >= VFS_READINESS_WATCH_THRESHOLD {
        "watch"
    } else {
        "critical"
    }
}

pub(super) const VFS_OPERATION_SUPPORT_ROWS: &[VfsOperationSupportRow] = &[
    VfsOperationSupportRow {
        operation: "open",
        default_trait: VFS_TRAIT_REQUIRED,
        ramfs: VFS_STATUS_FULL,
        tmpfs: VFS_STATUS_FULL,
        devfs: VFS_STATUS_FULL,
        procfs: VFS_STATUS_READ_MOSTLY,
        sysfs: VFS_STATUS_READ_MOSTLY,
        disk_fs: VFS_STATUS_FEATURE_GATED,
        writable_overlay: VFS_STATUS_FULL,
    },
    VfsOperationSupportRow {
        operation: "create",
        default_trait: VFS_TRAIT_REQUIRED,
        ramfs: VFS_STATUS_FULL,
        tmpfs: VFS_STATUS_FULL,
        devfs: VFS_STATUS_UNSUPPORTED,
        procfs: VFS_STATUS_UNSUPPORTED,
        sysfs: VFS_STATUS_UNSUPPORTED,
        disk_fs: VFS_STATUS_FEATURE_GATED,
        writable_overlay: VFS_STATUS_FULL,
    },
    VfsOperationSupportRow {
        operation: "remove",
        default_trait: VFS_TRAIT_REQUIRED,
        ramfs: VFS_STATUS_FULL,
        tmpfs: VFS_STATUS_FULL,
        devfs: VFS_STATUS_REMOVABLE_ONLY,
        procfs: VFS_STATUS_UNSUPPORTED,
        sysfs: VFS_STATUS_UNSUPPORTED,
        disk_fs: VFS_STATUS_FEATURE_GATED,
        writable_overlay: VFS_STATUS_FULL,
    },
    VfsOperationSupportRow {
        operation: "mkdir",
        default_trait: VFS_TRAIT_REQUIRED,
        ramfs: VFS_STATUS_FULL,
        tmpfs: VFS_STATUS_FULL,
        devfs: VFS_STATUS_UNSUPPORTED,
        procfs: VFS_STATUS_UNSUPPORTED,
        sysfs: VFS_STATUS_UNSUPPORTED,
        disk_fs: VFS_STATUS_FEATURE_GATED,
        writable_overlay: VFS_STATUS_FULL,
    },
    VfsOperationSupportRow {
        operation: "rmdir",
        default_trait: VFS_TRAIT_REQUIRED,
        ramfs: VFS_STATUS_FULL,
        tmpfs: VFS_STATUS_FULL,
        devfs: VFS_STATUS_UNSUPPORTED,
        procfs: VFS_STATUS_UNSUPPORTED,
        sysfs: VFS_STATUS_UNSUPPORTED,
        disk_fs: VFS_STATUS_FEATURE_GATED,
        writable_overlay: VFS_STATUS_FULL,
    },
    VfsOperationSupportRow {
        operation: "readdir",
        default_trait: VFS_TRAIT_REQUIRED,
        ramfs: VFS_STATUS_FULL,
        tmpfs: VFS_STATUS_FULL,
        devfs: VFS_STATUS_FULL,
        procfs: VFS_STATUS_PARTIAL,
        sysfs: VFS_STATUS_PARTIAL,
        disk_fs: VFS_STATUS_FEATURE_GATED,
        writable_overlay: VFS_STATUS_FULL,
    },
    VfsOperationSupportRow {
        operation: "stat",
        default_trait: VFS_TRAIT_REQUIRED,
        ramfs: VFS_STATUS_FULL,
        tmpfs: VFS_STATUS_FULL,
        devfs: VFS_STATUS_FULL,
        procfs: VFS_STATUS_PARTIAL,
        sysfs: VFS_STATUS_PARTIAL,
        disk_fs: VFS_STATUS_FEATURE_GATED,
        writable_overlay: VFS_STATUS_FULL,
    },
    VfsOperationSupportRow {
        operation: "chmod",
        default_trait: VFS_TRAIT_OPTIONAL,
        ramfs: VFS_STATUS_FULL,
        tmpfs: VFS_STATUS_FULL,
        devfs: VFS_STATUS_FULL,
        procfs: VFS_STATUS_PARTIAL,
        sysfs: VFS_STATUS_PARTIAL,
        disk_fs: VFS_STATUS_FEATURE_GATED,
        writable_overlay: VFS_STATUS_FULL,
    },
    VfsOperationSupportRow {
        operation: "chown",
        default_trait: VFS_TRAIT_OPTIONAL,
        ramfs: VFS_STATUS_FULL,
        tmpfs: VFS_STATUS_FULL,
        devfs: VFS_STATUS_FULL,
        procfs: VFS_STATUS_PARTIAL,
        sysfs: VFS_STATUS_PARTIAL,
        disk_fs: VFS_STATUS_FEATURE_GATED,
        writable_overlay: VFS_STATUS_FULL,
    },
    VfsOperationSupportRow {
        operation: "rename",
        default_trait: VFS_TRAIT_OPTIONAL,
        ramfs: VFS_STATUS_FULL,
        tmpfs: VFS_STATUS_FULL,
        devfs: VFS_STATUS_UNSUPPORTED,
        procfs: VFS_STATUS_UNSUPPORTED,
        sysfs: VFS_STATUS_UNSUPPORTED,
        disk_fs: VFS_STATUS_FEATURE_GATED,
        writable_overlay: VFS_STATUS_FULL,
    },
    VfsOperationSupportRow {
        operation: "link",
        default_trait: VFS_TRAIT_OPTIONAL,
        ramfs: VFS_STATUS_FULL,
        tmpfs: VFS_STATUS_FULL,
        devfs: VFS_STATUS_UNSUPPORTED,
        procfs: VFS_STATUS_UNSUPPORTED,
        sysfs: VFS_STATUS_UNSUPPORTED,
        disk_fs: VFS_STATUS_FEATURE_GATED,
        writable_overlay: VFS_STATUS_FULL,
    },
    VfsOperationSupportRow {
        operation: "symlink",
        default_trait: VFS_TRAIT_OPTIONAL,
        ramfs: VFS_STATUS_FULL,
        tmpfs: VFS_STATUS_FULL,
        devfs: VFS_STATUS_UNSUPPORTED,
        procfs: VFS_STATUS_UNSUPPORTED,
        sysfs: VFS_STATUS_UNSUPPORTED,
        disk_fs: VFS_STATUS_FEATURE_GATED,
        writable_overlay: VFS_STATUS_FULL,
    },
    VfsOperationSupportRow {
        operation: "readlink",
        default_trait: VFS_TRAIT_OPTIONAL,
        ramfs: VFS_STATUS_FULL,
        tmpfs: VFS_STATUS_FULL,
        devfs: VFS_STATUS_UNSUPPORTED,
        procfs: VFS_STATUS_UNSUPPORTED,
        sysfs: VFS_STATUS_UNSUPPORTED,
        disk_fs: VFS_STATUS_FEATURE_GATED,
        writable_overlay: VFS_STATUS_FULL,
    },
    VfsOperationSupportRow {
        operation: "set_times",
        default_trait: VFS_TRAIT_OPTIONAL,
        ramfs: VFS_STATUS_FULL,
        tmpfs: VFS_STATUS_FULL,
        devfs: VFS_STATUS_UNSUPPORTED,
        procfs: VFS_STATUS_UNSUPPORTED,
        sysfs: VFS_STATUS_UNSUPPORTED,
        disk_fs: VFS_STATUS_FEATURE_GATED,
        writable_overlay: VFS_STATUS_PARTIAL,
    },
    VfsOperationSupportRow {
        operation: "sync_fs",
        default_trait: VFS_TRAIT_REQUIRED,
        ramfs: VFS_STATUS_FULL,
        tmpfs: VFS_STATUS_FULL,
        devfs: VFS_STATUS_FULL,
        procfs: VFS_STATUS_FULL,
        sysfs: VFS_STATUS_FULL,
        disk_fs: VFS_STATUS_FULL,
        writable_overlay: VFS_STATUS_FULL,
    },
    VfsOperationSupportRow {
        operation: "lock",
        default_trait: VFS_TRAIT_UNLOCK_ONLY,
        ramfs: VFS_STATUS_BACKEND_SPECIFIC,
        tmpfs: VFS_STATUS_BACKEND_SPECIFIC,
        devfs: VFS_STATUS_BACKEND_SPECIFIC,
        procfs: VFS_STATUS_BACKEND_SPECIFIC,
        sysfs: VFS_STATUS_BACKEND_SPECIFIC,
        disk_fs: VFS_STATUS_BACKEND_SPECIFIC,
        writable_overlay: VFS_STATUS_BACKEND_SPECIFIC,
    },
];
