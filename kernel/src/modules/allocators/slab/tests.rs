use super::*;

#[test_case]
fn class_index_selects_expected_size_class() {
    let layout = Layout::from_size_align(48, 16).expect("valid layout");
    assert_eq!(class_index_for_layout(layout), Some(1));
}

#[test_case]
fn class_index_rejects_oversized_layout() {
    let layout = Layout::from_size_align(8192, 8).expect("valid layout");
    assert_eq!(class_index_for_layout(layout), None);
}

#[test_case]
fn refill_layout_helper_returns_valid_layout_for_known_block() {
    let layout = refill_layout_for_block_size(64).expect("refill layout");
    assert!(layout.size() >= 64);
    assert!(layout.align() >= 64);
}

#[test_case]
fn slab_tuning_is_normalized() {
    set_slab_tuning(SlabTuning::new(0, 0, 0, false));
    let tuning = slab_tuning();
    assert!(tuning.refill_bytes >= 4096);
    assert!(tuning.cache_limit >= 1);
    assert!(tuning.release_batch >= 1);
    assert!(!tuning.cross_cpu_steal);
}

#[test_case]
fn slab_extreme_fragmentation_pattern_sim() {
    use alloc::vec::Vec;
    let mut pointers = Vec::new();
    for i in 0..200 {
        let layout = Layout::from_size_align((i % 8 + 1) * 32, 32).unwrap();
        unsafe {
            let ptr = alloc::alloc::alloc(layout);
            if !ptr.is_null() {
                pointers.push((ptr, layout, true));
            }
        }
    }
    for i in (0..pointers.len()).step_by(2) {
        if pointers[i].2 {
            let (ptr, layout, _) = pointers[i];
            unsafe { alloc::alloc::dealloc(ptr, layout) };
            pointers[i].2 = false;
        }
    }
    for _ in 0..100 {
        let layout = Layout::from_size_align(64, 64).unwrap();
        unsafe {
            let ptr = alloc::alloc::alloc(layout);
            if !ptr.is_null() {
                pointers.push((ptr, layout, true));
            }
        }
    }
    for entry in pointers.iter_mut() {
        if entry.2 {
            let (ptr, layout, _) = *entry;
            unsafe { alloc::alloc::dealloc(ptr, layout) };
            entry.2 = false;
        }
    }
}

#[test_case]
fn block_alignment_helper_behaves_as_expected() {
    let aligned = 0x2000usize as *mut u8;
    let unaligned = 0x2008usize as *mut u8;
    assert!(is_block_aligned(aligned, 32));
    assert!(!is_block_aligned(unaligned, 32));
}

#[test_case]
fn reclaim_profile_runtime_override_roundtrip() {
    set_slab_reclaim_profile(SlabReclaimProfile::Conservative);
    assert_eq!(slab_reclaim_profile(), SlabReclaimProfile::Conservative);
    set_slab_reclaim_profile(SlabReclaimProfile::Aggressive);
    assert_eq!(slab_reclaim_profile(), SlabReclaimProfile::Aggressive);
}

#[test_case]
fn histogram_percentile_is_monotonic() {
    let buckets: [u64; 5] = [5, 3, 2, 1, 1];
    let total = buckets.iter().sum();
    let p50 = histogram_percentile_from_buckets(total, buckets, 50);
    let p95 = histogram_percentile_from_buckets(total, buckets, 95);
    let p99 = histogram_percentile_from_buckets(total, buckets, 99);
    assert!(p50 <= p95);
    assert!(p95 <= p99);
}

#[test_case]
fn pressure_budget_respects_profile_order() {
    let alloc = SlabAllocator::new();
    SLAB_RELEASE_BATCH.store(16, Ordering::Relaxed);
    SLAB_PRESSURE_SCAN_BUDGET.store(32, Ordering::Relaxed);
    set_slab_reclaim_profile(SlabReclaimProfile::Conservative);
    let conservative = alloc.pressure_reclaim_budget();
    set_slab_reclaim_profile(SlabReclaimProfile::Balanced);
    let balanced = alloc.pressure_reclaim_budget();
    set_slab_reclaim_profile(SlabReclaimProfile::Aggressive);
    let aggressive = alloc.pressure_reclaim_budget();
    assert!(conservative <= balanced);
    assert!(balanced <= aggressive);
}

#[test_case]
fn scache_drain_range_removes_only_nodes_in_target_window() {
    let mut cache = SCache::new();
    let mut blocks = [[0u8; 32]; 3];
    unsafe {
        cache.dealloc(blocks[0].as_mut_ptr());
        cache.dealloc(blocks[1].as_mut_ptr());
        cache.dealloc(blocks[2].as_mut_ptr());
        let start = blocks[1].as_ptr() as usize;
        let end = start + 32;
        assert_eq!(cache.drain_range(start, end), 1);
    }
    assert_eq!(cache.count, 2);
}

#[test_case]
fn scache_steal_one_pops_current_head() {
    let mut cache = SCache::new();
    let mut block = [0u8; 64];
    unsafe {
        cache.dealloc(block.as_mut_ptr());
        let stolen = cache.steal_one();
        assert_eq!(stolen, block.as_mut_ptr());
    }
    assert_eq!(cache.count, 0);
}
