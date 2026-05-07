mod runtime;
mod types;
#[cfg(feature = "posix_mman")]
mod vdso;
#[cfg(feature = "posix_mman")]
pub use vdso::init_linux_runtime_pages;
pub mod registry;
pub mod process_impl;

#[cfg(test)]
mod tests;

use crate::interfaces::task::ProcessId;
use core::sync::atomic::{AtomicUsize, Ordering};

pub use runtime::bind_prepared_image_snapshot;
pub use types::{
    MappingRecord, ProcessLifecycleState, ProcessRuntimeContractSnapshot, RuntimeLifecycleHooks,
};
pub use process_impl::{Process, PROCESS_NAME_LEN};

pub(crate) use crate::kernel::memory::{PAGE_ALIGN_MASK, PAGE_SIZE_BYTES_U64};

impl ProcessId {
    pub fn new() -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(1);
        ProcessId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}
