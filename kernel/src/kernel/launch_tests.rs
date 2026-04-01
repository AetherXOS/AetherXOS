use super::*;
use core::sync::atomic::Ordering;

#[test_case]
fn launch_stats_expose_runtime_fini_counters() {
    let prev_seen = RUNTIME_FINI_TRAMPOLINES_SEEN.swap(7, Ordering::Relaxed);
    let prev_deferred = RUNTIME_FINI_EXECUTION_DEFERRED.swap(5, Ordering::Relaxed);

    let snapshot = stats();
    assert_eq!(snapshot.runtime_fini_trampolines_seen, 7);
    assert_eq!(snapshot.runtime_fini_execution_deferred, 5);

    RUNTIME_FINI_TRAMPOLINES_SEEN.store(prev_seen, Ordering::Relaxed);
    RUNTIME_FINI_EXECUTION_DEFERRED.store(prev_deferred, Ordering::Relaxed);
}
