#[cfg(feature = "vfs")]
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
#[cfg(feature = "vfs")]
use super::types::MountStats;
#[cfg(feature = "vfs")]
use super::registry::MOUNT_REGISTRY;

#[cfg(feature = "vfs")]
pub(crate) static MOUNT_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "vfs")]
pub(crate) static MOUNT_SUCCESS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "vfs")]
pub(crate) static MOUNT_FAILURES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "vfs")]
pub(crate) static UNMOUNT_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "vfs")]
pub(crate) static UNMOUNT_SUCCESS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "vfs")]
pub(crate) static UNMOUNT_FAILURES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "vfs")]
pub(crate) static UNMOUNT_BY_PATH_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "vfs")]
pub(crate) static UNMOUNT_BY_PATH_SUCCESS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "vfs")]
pub(crate) static UNMOUNT_BY_PATH_FAILURES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "vfs")]
pub(crate) static PATH_VALIDATION_FAILURES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "vfs")]
pub(crate) static INITRD_LOAD_CALLS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "vfs")]
pub(crate) static INITRD_LOAD_FILES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "vfs")]
pub(crate) static INITRD_LOAD_BYTES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "vfs")]
pub(crate) static INITRD_LOAD_FAILURES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "vfs")]
pub(crate) static NEXT_MOUNT_ID: AtomicUsize = AtomicUsize::new(1);
#[cfg(feature = "vfs")]
pub(crate) static LAST_MOUNT_ID: AtomicUsize = AtomicUsize::new(0);

#[cfg(feature = "vfs")]
pub fn get_mount_stats() -> MountStats {
    MountStats {
        mount_attempts: MOUNT_ATTEMPTS.load(Ordering::Relaxed),
        mount_success: MOUNT_SUCCESS.load(Ordering::Relaxed),
        mount_failures: MOUNT_FAILURES.load(Ordering::Relaxed),
        unmount_attempts: UNMOUNT_ATTEMPTS.load(Ordering::Relaxed),
        unmount_success: UNMOUNT_SUCCESS.load(Ordering::Relaxed),
        unmount_failures: UNMOUNT_FAILURES.load(Ordering::Relaxed),
        unmount_by_path_attempts: UNMOUNT_BY_PATH_ATTEMPTS.load(Ordering::Relaxed),
        unmount_by_path_success: UNMOUNT_BY_PATH_SUCCESS.load(Ordering::Relaxed),
        unmount_by_path_failures: UNMOUNT_BY_PATH_FAILURES.load(Ordering::Relaxed),
        path_validation_failures: PATH_VALIDATION_FAILURES.load(Ordering::Relaxed),
        initrd_load_calls: INITRD_LOAD_CALLS.load(Ordering::Relaxed),
        initrd_load_files: INITRD_LOAD_FILES.load(Ordering::Relaxed),
        initrd_load_bytes: INITRD_LOAD_BYTES.load(Ordering::Relaxed),
        initrd_load_failures: INITRD_LOAD_FAILURES.load(Ordering::Relaxed),
        total_mounts: MOUNT_REGISTRY.lock().len(),
        last_mount_id: LAST_MOUNT_ID.load(Ordering::Relaxed),
    }
}

pub fn stats() -> MountStats {
    get_mount_stats()
}
