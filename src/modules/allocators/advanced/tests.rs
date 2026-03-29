use super::*;

#[test_case]
fn compact_memory_respects_global_budget() {
    set_advanced_tuning(AdvancedAllocatorTuning {
        compaction_budget_pages: 12,
        oom_kill_threshold: 0,
        prefer_local_numa: true,
    });
    register_compaction_candidate(0x200_0000, 100);
    let moved = compact_memory(100);
    assert!(moved <= 12, "compaction exceeded budget: {}", moved);
}

#[test_case]
fn oom_threshold_filters_candidates() {
    set_advanced_tuning(AdvancedAllocatorTuning {
        compaction_budget_pages: 32,
        oom_kill_threshold: 99,
        prefer_local_numa: true,
    });
    update_oom_score(1, 32);
    update_oom_score(2, 100);
    assert_eq!(pick_oom_victim(), Some(2));
    record_oom_kill(2);
    assert_eq!(pick_oom_victim(), None);
}

#[test_case]
fn hotplug_pending_drain_counts_pages() {
    let before = HOTPLUG_TOTAL_PAGES.load(Ordering::Relaxed);
    hotplug_add_memory(0x4000_0000, 64).unwrap();
    let drained = drain_hotplug_pending();
    assert!(drained >= 64);
    let after = HOTPLUG_TOTAL_PAGES.load(Ordering::Relaxed);
    assert!(after >= before + 64);
}

#[test_case]
fn numa_prefer_local_returns_local_node() {
    set_advanced_tuning(AdvancedAllocatorTuning {
        compaction_budget_pages: 32,
        oom_kill_threshold: 99,
        prefer_local_numa: true,
    });
    assert_eq!(preferred_numa_node(3, 4), 3);
    assert_eq!(preferred_numa_node(7, 4), 3);
}
