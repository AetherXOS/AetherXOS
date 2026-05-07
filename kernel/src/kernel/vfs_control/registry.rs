#[cfg(feature = "vfs")]
use alloc::boxed::Box;
#[cfg(feature = "vfs")]
use alloc::vec::Vec;
#[cfg(feature = "vfs")]
use crate::kernel::sync::IrqSafeMutex;
#[cfg(feature = "vfs")]
use super::types::MountEntry;

#[cfg(feature = "vfs")]
pub(crate) static MOUNT_REGISTRY: IrqSafeMutex<Vec<MountEntry>> = IrqSafeMutex::new(Vec::new());
#[cfg(feature = "vfs")]
pub(crate) static RAMFS_INSTANCES: IrqSafeMutex<Vec<(usize, Box<crate::modules::vfs::RamFs>)>> =
    IrqSafeMutex::new(Vec::new());

pub(crate) fn mount_count() -> usize {
    MOUNT_REGISTRY.lock().len()
}
