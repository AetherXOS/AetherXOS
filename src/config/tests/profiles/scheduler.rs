use super::*;

#[test_case]
fn scheduler_runtime_profile_roundtrip_and_reset() {
    KernelConfig::reset_runtime_overrides();

    let profile = super::SchedulerRuntimeProfile {
        cfs_min_granularity_ns: 2_000_000,
        cfs_latency_target_ns: 6_000_000,
        mlfq_base_slice_ns: 4_000_000,
        mlfq_boost_interval_ticks: 17,
        mlfq_demote_on_slice_exhaustion: false,
        edf_enforce_deadline: false,
        edf_default_relative_deadline_ns: 8_000_000,
        rt_group_reservation_enabled: false,
        rt_period_ns: 9_000_000,
        rt_total_utilization_cap_percent: 77,
        rt_max_groups: 123,
        lottery_tickets_per_priority_level: 15,
        lottery_min_tickets_per_task: 3,
    };
    KernelConfig::set_scheduler_runtime_profile(Some(profile));

    let got = KernelConfig::scheduler_runtime_profile();
    assert_eq!(got, profile);

    KernelConfig::set_scheduler_runtime_profile(None);
    let reset = KernelConfig::scheduler_runtime_profile();
    assert_eq!(
        reset.cfs_min_granularity_ns,
        crate::generated_consts::SCHED_CFS_MIN_GRANULARITY_NS
    );
    assert_eq!(
        reset.cfs_latency_target_ns,
        crate::generated_consts::SCHED_CFS_LATENCY_TARGET_NS
    );
    assert_eq!(
        reset.mlfq_base_slice_ns,
        crate::generated_consts::SCHED_MLFQ_BASE_SLICE_NS
    );
    assert_eq!(
        reset.mlfq_boost_interval_ticks,
        crate::generated_consts::SCHED_MLFQ_BOOST_INTERVAL_TICKS
    );
    assert_eq!(
        reset.mlfq_demote_on_slice_exhaustion,
        crate::generated_consts::SCHED_MLFQ_DEMOTE_ON_SLICE_EXHAUSTION
    );
    assert_eq!(
        reset.edf_enforce_deadline,
        crate::generated_consts::SCHED_EDF_ENFORCE_DEADLINE
    );
    assert_eq!(
        reset.edf_default_relative_deadline_ns,
        crate::generated_consts::SCHED_EDF_DEFAULT_RELATIVE_DEADLINE_NS
    );
    assert_eq!(
        reset.rt_group_reservation_enabled,
        crate::generated_consts::SCHED_RT_ENABLE_GROUP_RESERVATION
    );
    assert_eq!(
        reset.rt_period_ns,
        crate::generated_consts::SCHED_RT_PERIOD_NS
    );
    assert_eq!(
        reset.rt_total_utilization_cap_percent,
        crate::generated_consts::SCHED_RT_TOTAL_UTILIZATION_CAP_PERCENT
    );
    assert_eq!(
        reset.rt_max_groups,
        crate::generated_consts::SCHED_RT_MAX_GROUPS
    );
    assert_eq!(
        reset.lottery_tickets_per_priority_level,
        crate::generated_consts::SCHED_LOTTERY_TICKETS_PER_PRIORITY
    );
    assert_eq!(
        reset.lottery_min_tickets_per_task,
        crate::generated_consts::SCHED_LOTTERY_MIN_TICKETS
    );
}
