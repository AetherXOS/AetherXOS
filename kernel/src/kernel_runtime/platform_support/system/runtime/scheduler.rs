use super::super::current_virtualization_log_snapshot;

pub(crate) fn log_rt_preemption_guard() {
    let rt = hypercore::kernel::rt_preemption::stats();
    let virt = current_virtualization_log_snapshot();
    hypercore::klog_info!(
        "RT preemption guard: ticks={} reschedules={} forced={} manual_force={} streak={} max_streak={} runqueue={} starvation={} edf_pressure={} force_threshold={} force_override={} burst={}/{} deadline_alert={} deadline_events={} virt_dispatch={} virt_preemption={} virt_lane={} virt_window={} virt_mode={} virt_governor={} latency_bias={} energy_bias={}",
        rt.ticks,
        rt.reschedules,
        rt.forced_reschedules,
        rt.manual_force_requests,
        rt.continue_streak,
        rt.max_continue_streak,
        rt.last_runqueue_len,
        rt.starvation_alert,
        rt.edf_pressure_events,
        rt.force_threshold_ticks,
        rt.force_threshold_override_ticks,
        rt.forced_burst_count,
        rt.deadline_burst_threshold,
        rt.deadline_alert_active,
        rt.deadline_alert_events,
        virt.dispatch_class,
        virt.preemption_policy,
        virt.scheduler_lane,
        virt.dispatch_window,
        virt.selected_mode,
        virt.governor_class,
        virt.latency_bias,
        virt.energy_bias
    );
}

pub(crate) fn log_watchdog_runtime() {
    let wd = hypercore::kernel::watchdog::stats();
    hypercore::klog_info!(
        "Watchdog: tick={} checks={} stalls={} last_stalled_cpu={} hard_panic_ticks={} hard_panics={}",
        wd.global_tick,
        wd.checks,
        wd.stall_detections,
        wd.last_stalled_cpu,
        wd.hard_panic_ticks,
        wd.hard_panic_triggered
    );
}

pub(crate) fn log_load_balance_runtime() {
    let lb = hypercore::kernel::load_balance::stats_snapshot();
    let virt = current_virtualization_log_snapshot();
    hypercore::klog_info!(
        "LoadBalance: attempts={} moved={} affinity_skips={} prefer_local_skips={} prefer_local_forced={} hist_lt2={} hist_2_3={} hist_4_7={} hist_8_15={} hist_ge16={} p50={} p90={} p99={} samples={} virt_exec_profile={} virt_lane={} virt_mode={} virt_dispatch={} virt_governor={} latency_bias={} energy_bias={}",
        lb.attempts,
        lb.moved,
        lb.affinity_skips,
        lb.prefer_local_skips,
        lb.prefer_local_forced_moves,
        lb.imbalance_lt2,
        lb.imbalance_2_3,
        lb.imbalance_4_7,
        lb.imbalance_8_15,
        lb.imbalance_ge16,
        lb.imbalance_p50,
        lb.imbalance_p90,
        lb.imbalance_p99,
        lb.imbalance_samples,
        virt.execution_profile,
        virt.scheduler_lane,
        virt.selected_mode,
        virt.dispatch_class,
        virt.governor_class,
        virt.latency_bias,
        virt.energy_bias
    );
}
