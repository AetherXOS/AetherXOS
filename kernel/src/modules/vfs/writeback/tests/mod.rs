mod sinks;
mod helpers;
mod basic;
mod periodic;
mod sync;
mod journal;

use crate::modules::vfs::cache;
use super::{
    writeback_stats,
    reset_writeback_state_for_tests,
    register_writable_mount,
    unregister_writable_mount,
    register_inode,
    periodic_writeback,
    sync_all,
    GLOBAL_WRITEBACK,
    DirtyPageKey,
    DirtyPageEntry,
    evict_inodes_for_tests,
    fsync_inode,
    mark_dirty,
    PAGE_SIZE,
};
use alloc::sync::Arc;
use alloc::vec;
use spin::Mutex as SpinMutex;

pub use sinks::*;
pub use helpers::*;
