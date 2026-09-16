//! Domain-specific memory configuration.

use core::sync::atomic::{AtomicU64, Ordering};
use crate::generated_consts;

static HEAP_SIZE_OVERRIDE: AtomicU64 = AtomicU64::new(0);

pub struct MemoryConfig;

impl MemoryConfig {
    pub fn heap_size_mb() -> usize {
        let ov = HEAP_SIZE_OVERRIDE.load(Ordering::Relaxed);
        if ov != 0 { ov as usize } else { generated_consts::MEM_HEAP_SIZE_MB }
    }

    pub fn set_heap_size_mb(val: usize) {
        HEAP_SIZE_OVERRIDE.store(val as u64, Ordering::Relaxed);
    }

    pub fn reset_heap_size_mb() {
        HEAP_SIZE_OVERRIDE.store(0, Ordering::Relaxed);
    }

    pub fn guardian_pages() -> bool {
        generated_consts::MEM_GUARDIAN_PAGES
    }

    pub fn slab_reclaim_profile() -> &'static str {
        generated_consts::MEM_SLAB_RECLAIM_PROFILE
    }
}
