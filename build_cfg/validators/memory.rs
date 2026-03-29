//! Memory config validation — heap, slab, allocator, compaction.

use crate::build_cfg::config_types::MemoryConfig;

const VALID_ALLOCATORS: &[&str] = &[
    "Bump",
    "LinkedListAllocator",
    "Slab",
    "Buddy",
    "PoolAllocator",
];
const VALID_RECLAIM_PROFILES: &[&str] = &["Aggressive", "Balanced", "Conservative"];
const MIN_HEAP_MB: usize = 1;
const MAX_HEAP_MB: usize = 16384;
const MIN_SLAB_REFILL: usize = 512;
const MAX_SLAB_REFILL: usize = 1024 * 1024;
const MAX_SLAB_CACHE_LIMIT: usize = 4096;
const MAX_SLAB_RELEASE_BATCH: usize = 1024;
const MIN_POOL_BLOCK_SIZE: usize = 64;
const MAX_POOL_BLOCK_SIZE: usize = 65536;
const MAX_COMPACTION_BUDGET: usize = 65536;

pub fn validate(c: &MemoryConfig) -> Vec<String> {
    let mut e = Vec::new();

    if !VALID_ALLOCATORS.contains(&c.allocator.as_str()) {
        e.push(format!(
            "memory.allocator '{}' invalid, expected one of {:?}",
            c.allocator, VALID_ALLOCATORS
        ));
    }
    if !VALID_RECLAIM_PROFILES.contains(&c.slab_reclaim_profile.as_str()) {
        e.push(format!(
            "memory.slab_reclaim_profile '{}' invalid, expected one of {:?}",
            c.slab_reclaim_profile, VALID_RECLAIM_PROFILES
        ));
    }
    if c.heap_size_mb < MIN_HEAP_MB || c.heap_size_mb > MAX_HEAP_MB {
        e.push(format!(
            "memory.heap_size_mb {} out of range [{}, {}]",
            c.heap_size_mb, MIN_HEAP_MB, MAX_HEAP_MB
        ));
    }
    if c.slab_refill_bytes < MIN_SLAB_REFILL || c.slab_refill_bytes > MAX_SLAB_REFILL {
        e.push(format!(
            "memory.slab_refill_bytes {} out of range [{}, {}]",
            c.slab_refill_bytes, MIN_SLAB_REFILL, MAX_SLAB_REFILL
        ));
    }
    if c.slab_cache_limit == 0 || c.slab_cache_limit > MAX_SLAB_CACHE_LIMIT {
        e.push(format!(
            "memory.slab_cache_limit {} out of range [1, {}]",
            c.slab_cache_limit, MAX_SLAB_CACHE_LIMIT
        ));
    }
    if c.slab_release_batch == 0 || c.slab_release_batch > MAX_SLAB_RELEASE_BATCH {
        e.push(format!(
            "memory.slab_release_batch {} out of range [1, {}]",
            c.slab_release_batch, MAX_SLAB_RELEASE_BATCH
        ));
    }
    if c.pool_block_size < MIN_POOL_BLOCK_SIZE || c.pool_block_size > MAX_POOL_BLOCK_SIZE {
        e.push(format!(
            "memory.pool_block_size {} out of range [{}, {}]",
            c.pool_block_size, MIN_POOL_BLOCK_SIZE, MAX_POOL_BLOCK_SIZE
        ));
    }
    if !c.pool_block_size.is_power_of_two() {
        e.push(format!(
            "memory.pool_block_size {} must be a power of two",
            c.pool_block_size
        ));
    }
    if c.compaction_budget_pages > MAX_COMPACTION_BUDGET {
        e.push(format!(
            "memory.compaction_budget_pages {} exceeds max {}",
            c.compaction_budget_pages, MAX_COMPACTION_BUDGET
        ));
    }
    if c.slab_pressure_scan_budget == 0 {
        e.push("memory.slab_pressure_scan_budget must be > 0".to_string());
    }
    if c.slab_max_tracked_segments < 64 {
        e.push(format!(
            "memory.slab_max_tracked_segments {} must be >= 64",
            c.slab_max_tracked_segments
        ));
    }

    e
}
