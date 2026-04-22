pub(super) const VFS_FS_RAMFS: &str = "ramfs";
pub(super) const VFS_FS_TMPFS: &str = "tmpfs";
pub(super) const VFS_FS_DEVFS: &str = "devfs";
pub(super) const VFS_FS_PROCFS: &str = "procfs";
pub(super) const VFS_FS_SYSFS: &str = "sysfs";
pub(super) const VFS_FS_DISK_FS: &str = "disk_fs";
pub(super) const VFS_FS_WRITABLE_OVERLAY: &str = "writable_overlay";
pub(super) const VFS_FS_NAMES: [&str; 7] = [
    VFS_FS_RAMFS,
    VFS_FS_TMPFS,
    VFS_FS_DEVFS,
    VFS_FS_PROCFS,
    VFS_FS_SYSFS,
    VFS_FS_DISK_FS,
    VFS_FS_WRITABLE_OVERLAY,
];

pub(super) const VFS_STATUS_FULL: &str = "full";
pub(super) const VFS_STATUS_PARTIAL: &str = "partial";
pub(super) const VFS_STATUS_READ_MOSTLY: &str = "read-mostly";
pub(super) const VFS_STATUS_FEATURE_GATED: &str = "feature-gated";
pub(super) const VFS_STATUS_BACKEND_SPECIFIC: &str = "backend-specific";
pub(super) const VFS_STATUS_REMOVABLE_ONLY: &str = "removable-only";
pub(super) const VFS_STATUS_UNSUPPORTED: &str = "unsupported";

pub(super) const VFS_TRAIT_REQUIRED: &str = "required";
pub(super) const VFS_TRAIT_OPTIONAL: &str = "optional";
pub(super) const VFS_TRAIT_UNLOCK_ONLY: &str = "unlock-only";

pub(super) const VFS_READINESS_STRONG_THRESHOLD: u32 = 85;
pub(super) const VFS_READINESS_WATCH_THRESHOLD: u32 = 70;
pub(super) const VFS_MATRIX_WARN_THRESHOLD: u32 = 70;
pub(super) const VFS_OPERATION_HOTSPOT_THRESHOLD: u32 = 55;
pub(super) const VFS_REQUIRED_OPERATION_HOTSPOT_THRESHOLD: u32 = 70;

pub(super) const VFS_STATUS_WEIGHT_FULL: u32 = 100;
pub(super) const VFS_STATUS_WEIGHT_PARTIAL: u32 = 65;
pub(super) const VFS_STATUS_WEIGHT_READ_MOSTLY: u32 = 70;
pub(super) const VFS_STATUS_WEIGHT_FEATURE_GATED: u32 = 55;
pub(super) const VFS_STATUS_WEIGHT_BACKEND_SPECIFIC: u32 = 50;
pub(super) const VFS_STATUS_WEIGHT_REMOVABLE_ONLY: u32 = 70;
pub(super) const VFS_STATUS_WEIGHT_UNSUPPORTED: u32 = 0;
pub(super) const VFS_STATUS_WEIGHT_DEFAULT: u32 = 25;

pub(super) const VFS_OPERATION_WEIGHT_REQUIRED: u32 = 3;
pub(super) const VFS_OPERATION_WEIGHT_OPTIONAL: u32 = 2;
pub(super) const VFS_OPERATION_WEIGHT_UNLOCK_ONLY: u32 = 1;
pub(super) const VFS_OPERATION_WEIGHT_DEFAULT: u32 = 1;
