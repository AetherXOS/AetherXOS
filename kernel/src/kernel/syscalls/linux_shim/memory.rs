use super::*;

#[path = "memory/mapping_helpers.rs"]
mod mapping_helpers;
#[path = "memory/mlock_ops.rs"]
mod mlock_ops;
#[path = "memory/mmap_ops.rs"]
mod mmap_ops;
#[path = "memory/mmap_support.rs"]
mod mmap_support;

#[cfg(not(feature = "linux_compat"))]
pub(super) use mlock_ops::{
    sys_linux_mlock, sys_linux_mlockall, sys_linux_munlock, sys_linux_munlockall,
};
#[cfg(not(feature = "linux_compat"))]
pub(super) use mmap_ops::{
    sys_linux_madvise, sys_linux_mmap, sys_linux_mprotect, sys_linux_mremap, sys_linux_munmap,
};
