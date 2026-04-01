#[cfg(param_scheduler = "EDF")]
pub(crate) fn log_scheduler_runtime() {
    let edf = hypercore::modules::schedulers::edf::runtime_stats();
    hypercore::klog_info!(
        "EDF runtime: ticks={} misses={} resched={} window_resets={} throttles={}",
        edf.ticks,
        edf.deadline_misses,
        edf.reschedule_hints,
        edf.window_resets,
        edf.group_throttle_events
    );
}

#[cfg(param_scheduler = "Lottery")]
pub(crate) fn log_scheduler_runtime() {
    let lot = hypercore::modules::schedulers::lottery::runtime_stats();
    hypercore::klog_info!(
        "Lottery runtime: add={} remove={} picks={} empty={} fallback_first={} replay_seq={} replay_overwrites={}",
        lot.add_calls,
        lot.remove_calls,
        lot.pick_calls,
        lot.pick_empty,
        lot.fallback_first,
        lot.replay_latest_seq,
        lot.replay_overwrites
    );
}

#[cfg(not(any(param_scheduler = "EDF", param_scheduler = "Lottery")))]
pub(crate) fn log_scheduler_runtime() {}
