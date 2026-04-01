#[cfg(feature = "allocators")]
pub(crate) fn log_slab_runtime() {
    let slab = hypercore::modules::allocators::slab::runtime_stats();
    hypercore::klog_info!(
        "Slab runtime: alloc_calls={} fast={} refill={} steal={} fallback={} refill_failures={} reclaim={}/{} swept={} pressure_passes={} reclaim_lat(avg/p95/p99/max)={}/{}/{}/{} qd(last/avg/p95/p99/max)={}/{}/{}/{}/{} seg_active={} seg_peak={} seg_track_failures={} profile={} scan_budget={}",
        slab.alloc_calls,
        slab.alloc_fast_hits,
        slab.alloc_refill_hits,
        slab.alloc_steal_hits,
        slab.alloc_fallback_hits,
        slab.refill_failures,
        slab.reclaim_successes,
        slab.reclaim_attempts,
        slab.reclaim_sweeped_blocks,
        slab.pressure_reclaim_passes,
        slab.reclaim_latency_avg_ticks,
        slab.reclaim_latency_p95_ticks,
        slab.reclaim_latency_p99_ticks,
        slab.reclaim_latency_max_ticks,
        slab.reclaim_queue_depth_last,
        slab.reclaim_queue_depth_avg,
        slab.reclaim_queue_depth_p95,
        slab.reclaim_queue_depth_p99,
        slab.reclaim_queue_depth_max,
        slab.active_segments,
        slab.peak_active_segments,
        slab.segment_track_failures,
        slab.reclaim_profile,
        slab.pressure_scan_budget
    );
}

#[cfg(feature = "allocators")]
pub(crate) fn log_allocator_diagnostics() {
    let jla = hypercore::modules::allocators::jemalloc_lite_stats();
    hypercore::klog_info!(
        "Allocator diagnostics: jemalloc_lite_alloc_attempts={}",
        jla.alloc_attempts
    );
}
