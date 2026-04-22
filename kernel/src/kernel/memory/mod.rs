pub mod compaction;
pub mod lru;
pub mod manager;
pub mod numa;
pub mod oom;
pub mod paging;
mod paging_support;
pub mod slab;
pub mod swap;
pub mod rt_pools;

pub use compaction::{CompactionResult, PageMobility, Zone};
pub use lru::{LruPage, LruReclaimer, PageFlags, ReclaimConfig, ReclaimResult};
pub use manager::MemoryManager;
pub use numa::{NumaAllocator, NumaRegion};
pub use oom::{OomAction, OomCandidate, PressureLevel};
pub(crate) use paging_support::{validate_page_aligned_range, PAGE_ALIGN_MASK, PAGE_SIZE_BYTES_U64};
pub use slab::{SlabAllocator, SlabCache, SlabCacheStats};
pub use swap::{SwapArea, SwapManager, SwapSlot, SwapStats};
