/// Virtual File System (VFS) module.
///
/// Exokernel boundary stays in kernel mount control and raw block/device access,
/// while filesystem policies and adapters remain in Library-facing submodules.
pub mod backends;
pub mod cache;
pub mod devfs;
#[cfg(feature = "vfs_disk_fs")]
pub mod disk_fs;
#[cfg(feature = "vfs_disk_fs")]
mod disk_fs_support;
pub mod file_lock;
pub mod health;
pub mod journal;
mod journal_support;
#[cfg(feature = "vfs_library_backends")]
pub mod library_backends;
pub mod mount_table;
#[cfg(feature = "vfs_network_fs")]
pub mod network_fs;
pub mod path;
#[cfg(feature = "vfs_ramfs")]
pub mod ramfs;
mod ramfs_support;
pub mod service_templates;
#[cfg(feature = "vfs_telemetry")]
pub mod telemetry;
pub mod types;
pub mod writable_fs;
mod writable_fs_support;
pub mod writeback;
mod writeback_support;
pub mod xattr;

// ── Linux compatibility virtual filesystems ─────────────────────────────────
/// Core device files (/dev/null, /dev/zero, etc.) — always available.
pub mod dev_special;
/// In-memory temporary filesystem (/tmp, /run, /dev/shm) — always available.
pub mod tmpfs;
/// /proc virtual filesystem — requires linux_compat feature.
#[cfg(feature = "linux_compat")]
pub mod procfs;
/// /sys virtual filesystem — requires linux_compat feature.
#[cfg(feature = "linux_compat")]
pub mod sysfs;
/// Pseudo-terminal subsystem (/dev/ptmx, /dev/pts/*) — requires linux_compat.
#[cfg(feature = "linux_compat")]
pub mod pty;
/// Linux VFS mount orchestrator — sets up /proc, /sys, /tmp, /dev/pts, etc.
#[cfg(feature = "linux_compat")]
pub mod linux_mount_setup;
/// Linux feature inspection and capability reporting.
pub mod linux_features;

pub use cache::{
    alloc_ino, CachedFileSystem, Dentry, Inode, InodeCache, NegativeDentryCache,
    GLOBAL_INODE_CACHE, NEGATIVE_DENTRY_CACHE,
};
pub use file_lock::{FlockLock, LockManager, LockType, PosixLock};
pub use health::{
    current_mount_health_summary, evaluate_mount_health_slo, mount_slo_thresholds,
    recommended_mount_health_action, summarize_mount_health, VfsHealthSummary, VfsHealthTier,
    VfsMountHealthAction, VfsMountSloReport, VfsMountSloThresholds,
};
pub use mount_table::{FsType, MountEntry, MountFlags, MountId, MountTable};
#[cfg(feature = "vfs_network_fs")]
pub use network_fs::{
    force_nfs_disconnect, force_p9_disconnect, network_fs_stats, nfs_mount, nfs_read, p9_attach,
    p9_read, NetworkFsStats,
};
#[cfg(feature = "vfs_ramfs")]
pub use ramfs::{RamFile, RamFs};
#[cfg(feature = "vfs_disk_fs")]
pub use service_templates::apply_recommended_io_policy;
pub use service_templates::{
    io_policy_for_preset, recommended_io_policy, recommended_storage_preset, StorageServicePreset,
};
#[cfg(feature = "vfs_telemetry")]
pub use telemetry::{
    bridge_stats, disk_io_latency_stats, probe_fatfs_bridge, VfsBridgeStats, VfsDiskIoLatencyStats,
};
pub use types::{DirEntry, File, FileStats, FileSystem, IoPolicy, PollEvents, SeekFrom};
pub use writable_fs::{
    BlockDeviceAdapter, BlockWritebackSink, RamWritebackSink, WritableOverlayFs,
};
pub use writeback::{JournalEntry, JournalOp, JournalTransaction, WritebackSink, WritebackStats};
pub use xattr::{
    InodeXattrs, XattrError, XattrNamespace, XattrRegistry, XattrSetFlags, XATTR_REGISTRY,
};

#[cfg(feature = "vfs_library_backends")]
pub fn library_backend_inventory() -> alloc::vec::Vec<library_backends::LibraryBackendDescriptor> {
    library_backends::library_backend_inventory()
}
