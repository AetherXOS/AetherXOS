pub mod advanced;
pub mod bitmap_pmm;
pub mod buddy;
pub mod bump_allocator;
pub mod linked_list_allocator;
pub mod pool_allocator;

pub use advanced::{
    advanced_stats, advanced_tuning, compact_memory, hotplug_add_memory_pages, pick_oom_victim,
    preferred_numa_node, set_advanced_tuning, slub_alloc, update_oom_score, AdvancedAllocatorStats,
    AdvancedAllocatorTuning,
};
pub use bitmap_pmm::BitmapAllocator;
pub use buddy::BuddyAllocator;
pub use bump_allocator::BumpAllocator;
pub use linked_list_allocator::LinkedListAllocator;
pub use pool_allocator::PoolAllocator;

use crate::interfaces::memory::HeapAllocator;
use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicU64, Ordering};

static JEMALLOC_LITE_ALLOC_ATTEMPTS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
pub struct JemallocLiteStats {
    pub alloc_attempts: u64,
}

pub fn jemalloc_lite_stats() -> JemallocLiteStats {
    JemallocLiteStats {
        alloc_attempts: JEMALLOC_LITE_ALLOC_ATTEMPTS.load(Ordering::Relaxed),
    }
}

// --- Selectors ---

pub mod selector {
    use super::*;

    #[cfg(param_allocator = "Bump")]
    pub type ActiveHeapAllocator = BumpAllocator;

    #[cfg(param_allocator = "LinkedListAllocator")]
    pub type ActiveHeapAllocator = LinkedListAllocator;

    #[cfg(param_allocator = "Buddy")]
    pub type ActiveHeapAllocator = BuddyAllocator;

    #[cfg(param_allocator = "Slab")]
    pub type ActiveHeapAllocator = SlabAllocator;

    #[cfg(param_allocator = "PoolAllocator")]
    pub type ActiveHeapAllocator = PoolAllocator;

    #[cfg(not(any(
        param_allocator = "Bump",
        param_allocator = "LinkedListAllocator",
        param_allocator = "Buddy",
        param_allocator = "Slab",
        param_allocator = "PoolAllocator"
    )))]
    pub type ActiveHeapAllocator = BumpAllocator; // Fallback to simplest

    // PMM Selection (current default page allocator policy)
    pub type ActivePageAllocator = BitmapAllocator;
}

pub mod slab;
pub use slab::{
    set_slab_pressure_scan_budget, set_slab_reclaim_profile, set_slab_runtime_config,
    set_slab_tuning, slab_pressure_scan_budget, slab_reclaim_profile, slab_runtime_config,
    slab_tuning, SlabAllocator, SlabReclaimProfile, SlabRuntimeConfig, SlabTuning,
};

pub struct JemallocLite;
unsafe impl GlobalAlloc for JemallocLite {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        JEMALLOC_LITE_ALLOC_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
        crate::klog_error!(
            "JemallocLite alloc invoked unexpectedly (unsupported allocator profile), aborting"
        );
        crate::kernel::fatal_halt("unsupported_allocator_profile")
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}
impl HeapAllocator for JemallocLite {
    fn init(&self, _start: usize, _size: usize) {}
}
impl JemallocLite {
    pub const fn new() -> Self {
        Self
    }
}
