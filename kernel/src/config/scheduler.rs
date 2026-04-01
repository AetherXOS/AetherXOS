use core::sync::atomic::Ordering;

use super::*;

impl KernelConfig {
    pub fn mlfq_num_queues() -> usize {
        SCHED_MLFQ_NUM_QUEUES
    }

    pub fn cfs_min_granularity_ns() -> u64 {
        load_u64_override_clamped(
            &SCHED_CFS_MIN_GRANULARITY_NS_OVERRIDE,
            DEFAULT_SCHED_CFS_MIN_GRANULARITY_NS,
            1,
            MAX_SCHED_CFS_MIN_GRANULARITY_NS,
        )
    }

    pub fn cfs_latency_target_ns() -> u64 {
        let raw = load_u64_override_clamped(
            &SCHED_CFS_LATENCY_TARGET_NS_OVERRIDE,
            DEFAULT_SCHED_CFS_LATENCY_TARGET_NS,
            1,
            MAX_SCHED_CFS_LATENCY_TARGET_NS,
        );
        raw.max(Self::cfs_min_granularity_ns())
    }

    pub fn mlfq_base_slice_ns() -> u64 {
        load_u64_override_clamped(
            &SCHED_MLFQ_BASE_SLICE_NS_OVERRIDE,
            DEFAULT_SCHED_MLFQ_BASE_SLICE_NS,
            1,
            MAX_SCHED_MLFQ_BASE_SLICE_NS,
        )
    }

    pub fn mlfq_boost_interval_ticks() -> u64 {
        load_u64_override_clamped(
            &SCHED_MLFQ_BOOST_INTERVAL_TICKS_OVERRIDE,
            DEFAULT_SCHED_MLFQ_BOOST_INTERVAL_TICKS,
            1,
            MAX_SCHED_MLFQ_BOOST_INTERVAL_TICKS,
        )
    }

    pub fn mlfq_demote_on_slice_exhaustion() -> bool {
        decode_bool_override(
            SCHED_MLFQ_DEMOTE_ON_SLICE_EXHAUSTION_OVERRIDE.load(Ordering::Relaxed),
            DEFAULT_SCHED_MLFQ_DEMOTE_ON_SLICE_EXHAUSTION,
        )
    }

    pub fn edf_enforce_deadline() -> bool {
        decode_bool_override(
            SCHED_EDF_ENFORCE_DEADLINE_OVERRIDE.load(Ordering::Relaxed),
            DEFAULT_SCHED_EDF_ENFORCE_DEADLINE,
        )
    }

    pub fn edf_default_relative_deadline_ns() -> u64 {
        load_u64_override_clamped(
            &SCHED_EDF_DEFAULT_RELATIVE_DEADLINE_NS_OVERRIDE,
            DEFAULT_SCHED_EDF_DEFAULT_RELATIVE_DEADLINE_NS,
            1,
            MAX_SCHED_EDF_DEFAULT_RELATIVE_DEADLINE_NS,
        )
    }

    pub fn rt_group_reservation_enabled() -> bool {
        decode_bool_override(
            SCHED_RT_ENABLE_GROUP_RESERVATION_OVERRIDE.load(Ordering::Relaxed),
            DEFAULT_SCHED_RT_ENABLE_GROUP_RESERVATION,
        )
    }

    pub fn rt_period_ns() -> u64 {
        load_u64_override_clamped(
            &SCHED_RT_PERIOD_NS_OVERRIDE,
            DEFAULT_SCHED_RT_PERIOD_NS,
            1,
            MAX_SCHED_RT_PERIOD_NS,
        )
    }

    pub fn rt_total_utilization_cap_percent() -> u8 {
        load_u8_from_usize_override_clamped(
            &SCHED_RT_TOTAL_UTILIZATION_CAP_PERCENT_OVERRIDE,
            DEFAULT_SCHED_RT_TOTAL_UTILIZATION_CAP_PERCENT,
            1,
            MAX_SCHED_RT_TOTAL_UTILIZATION_CAP_PERCENT,
        )
    }

    pub fn rt_max_groups() -> usize {
        load_usize_override_clamped(
            &SCHED_RT_MAX_GROUPS_OVERRIDE,
            DEFAULT_SCHED_RT_MAX_GROUPS,
            1,
            MAX_SCHED_RT_MAX_GROUPS,
        )
    }

    pub fn scheduler_runtime_profile() -> SchedulerRuntimeProfile {
        SchedulerRuntimeProfile {
            cfs_min_granularity_ns: Self::cfs_min_granularity_ns(),
            cfs_latency_target_ns: Self::cfs_latency_target_ns(),
            mlfq_base_slice_ns: Self::mlfq_base_slice_ns(),
            mlfq_boost_interval_ticks: Self::mlfq_boost_interval_ticks(),
            mlfq_demote_on_slice_exhaustion: Self::mlfq_demote_on_slice_exhaustion(),
            edf_enforce_deadline: Self::edf_enforce_deadline(),
            edf_default_relative_deadline_ns: Self::edf_default_relative_deadline_ns(),
            rt_group_reservation_enabled: Self::rt_group_reservation_enabled(),
            rt_period_ns: Self::rt_period_ns(),
            rt_total_utilization_cap_percent: Self::rt_total_utilization_cap_percent(),
            rt_max_groups: Self::rt_max_groups(),
            lottery_tickets_per_priority_level: Self::sched_lottery_tickets_per_priority_level(),
            lottery_min_tickets_per_task: Self::sched_lottery_min_tickets_per_task(),
        }
    }

    pub fn scheduler_cargo_profile() -> SchedulerRuntimeProfile {
        SchedulerRuntimeProfile {
            cfs_min_granularity_ns: DEFAULT_SCHED_CFS_MIN_GRANULARITY_NS,
            cfs_latency_target_ns: DEFAULT_SCHED_CFS_LATENCY_TARGET_NS,
            mlfq_base_slice_ns: DEFAULT_SCHED_MLFQ_BASE_SLICE_NS,
            mlfq_boost_interval_ticks: DEFAULT_SCHED_MLFQ_BOOST_INTERVAL_TICKS,
            mlfq_demote_on_slice_exhaustion: DEFAULT_SCHED_MLFQ_DEMOTE_ON_SLICE_EXHAUSTION,
            edf_enforce_deadline: DEFAULT_SCHED_EDF_ENFORCE_DEADLINE,
            edf_default_relative_deadline_ns: DEFAULT_SCHED_EDF_DEFAULT_RELATIVE_DEADLINE_NS,
            rt_group_reservation_enabled: DEFAULT_SCHED_RT_ENABLE_GROUP_RESERVATION,
            rt_period_ns: DEFAULT_SCHED_RT_PERIOD_NS,
            rt_total_utilization_cap_percent: DEFAULT_SCHED_RT_TOTAL_UTILIZATION_CAP_PERCENT,
            rt_max_groups: DEFAULT_SCHED_RT_MAX_GROUPS,
            lottery_tickets_per_priority_level: DEFAULT_SCHED_LOTTERY_TICKETS_PER_PRIORITY_LEVEL,
            lottery_min_tickets_per_task: DEFAULT_SCHED_LOTTERY_MIN_TICKETS_PER_TASK,
        }
    }

    pub fn syscall_max_path_len() -> usize {
        load_usize_override_clamped(
            &SYSCALL_MAX_PATH_LEN_OVERRIDE,
            DEFAULT_SYSCALL_MAX_PATH_LEN,
            1,
            MAX_SYSCALL_MAX_PATH_LEN,
        )
    }

    pub fn set_syscall_max_path_len(value: Option<usize>) {
        store_usize_override(&SYSCALL_MAX_PATH_LEN_OVERRIDE, value);
    }

    pub fn sched_lottery_initial_seed() -> u64 {
        load_u64_override(
            &SCHED_LOTTERY_INITIAL_SEED_OVERRIDE,
            DEFAULT_SCHED_LOTTERY_INITIAL_SEED,
        )
    }

    pub fn set_sched_lottery_initial_seed(value: Option<u64>) {
        store_u64_override(&SCHED_LOTTERY_INITIAL_SEED_OVERRIDE, value);
    }

    pub fn sched_lottery_tickets_per_priority_level() -> u64 {
        load_u64_override_clamped(
            &SCHED_LOTTERY_TICKETS_PER_PRIORITY_LEVEL_OVERRIDE,
            DEFAULT_SCHED_LOTTERY_TICKETS_PER_PRIORITY_LEVEL,
            1,
            MAX_SCHED_LOTTERY_TICKETS_PER_PRIORITY_LEVEL,
        )
    }

    pub fn set_sched_lottery_tickets_per_priority_level(value: Option<u64>) {
        store_u64_override(&SCHED_LOTTERY_TICKETS_PER_PRIORITY_LEVEL_OVERRIDE, value);
    }

    pub fn sched_lottery_min_tickets_per_task() -> u64 {
        load_u64_override_clamped(
            &SCHED_LOTTERY_MIN_TICKETS_PER_TASK_OVERRIDE,
            DEFAULT_SCHED_LOTTERY_MIN_TICKETS_PER_TASK,
            1,
            MAX_SCHED_LOTTERY_MIN_TICKETS_PER_TASK,
        )
    }

    pub fn set_sched_lottery_min_tickets_per_task(value: Option<u64>) {
        store_u64_override(&SCHED_LOTTERY_MIN_TICKETS_PER_TASK_OVERRIDE, value);
    }

    pub fn sched_lottery_lcg_multiplier() -> u64 {
        let override_value = SCHED_LOTTERY_LCG_MULTIPLIER_OVERRIDE.load(Ordering::Relaxed);
        let raw = if override_value == 0 {
            DEFAULT_SCHED_LOTTERY_LCG_MULTIPLIER
        } else {
            override_value.min(MAX_SCHED_LOTTERY_LCG_PARAM)
        };
        raw.max(1) | 1
    }

    pub fn set_sched_lottery_lcg_multiplier(value: Option<u64>) {
        store_u64_override(&SCHED_LOTTERY_LCG_MULTIPLIER_OVERRIDE, value);
    }

    pub fn sched_lottery_lcg_increment() -> u64 {
        let override_value = SCHED_LOTTERY_LCG_INCREMENT_OVERRIDE.load(Ordering::Relaxed);
        let raw = if override_value == 0 {
            DEFAULT_SCHED_LOTTERY_LCG_INCREMENT
        } else {
            override_value.min(MAX_SCHED_LOTTERY_LCG_PARAM)
        };
        raw.max(1) | 1
    }

    pub fn set_sched_lottery_lcg_increment(value: Option<u64>) {
        store_u64_override(&SCHED_LOTTERY_LCG_INCREMENT_OVERRIDE, value);
    }

    pub fn set_cfs_min_granularity_ns(value: Option<u64>) {
        store_u64_override(&SCHED_CFS_MIN_GRANULARITY_NS_OVERRIDE, value);
    }

    pub fn set_cfs_latency_target_ns(value: Option<u64>) {
        store_u64_override(&SCHED_CFS_LATENCY_TARGET_NS_OVERRIDE, value);
    }

    pub fn set_mlfq_base_slice_ns(value: Option<u64>) {
        store_u64_override(&SCHED_MLFQ_BASE_SLICE_NS_OVERRIDE, value);
    }

    pub fn set_mlfq_boost_interval_ticks(value: Option<u64>) {
        store_u64_override(&SCHED_MLFQ_BOOST_INTERVAL_TICKS_OVERRIDE, value);
    }

    pub fn set_mlfq_demote_on_slice_exhaustion(value: Option<bool>) {
        SCHED_MLFQ_DEMOTE_ON_SLICE_EXHAUSTION_OVERRIDE
            .store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_edf_enforce_deadline(value: Option<bool>) {
        SCHED_EDF_ENFORCE_DEADLINE_OVERRIDE.store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_edf_default_relative_deadline_ns(value: Option<u64>) {
        store_u64_override(&SCHED_EDF_DEFAULT_RELATIVE_DEADLINE_NS_OVERRIDE, value);
    }

    pub fn set_rt_group_reservation_enabled(value: Option<bool>) {
        SCHED_RT_ENABLE_GROUP_RESERVATION_OVERRIDE
            .store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn set_rt_period_ns(value: Option<u64>) {
        store_u64_override(&SCHED_RT_PERIOD_NS_OVERRIDE, value);
    }

    pub fn set_rt_total_utilization_cap_percent(value: Option<u8>) {
        SCHED_RT_TOTAL_UTILIZATION_CAP_PERCENT_OVERRIDE
            .store(value.unwrap_or(0) as usize, Ordering::Relaxed);
    }

    pub fn set_rt_max_groups(value: Option<usize>) {
        store_usize_override(&SCHED_RT_MAX_GROUPS_OVERRIDE, value);
    }

    pub fn set_scheduler_runtime_profile(value: Option<SchedulerRuntimeProfile>) {
        apply_profile_override(
            value,
            |profile| {
                Self::set_cfs_min_granularity_ns(Some(profile.cfs_min_granularity_ns));
                Self::set_cfs_latency_target_ns(Some(profile.cfs_latency_target_ns));
                Self::set_mlfq_base_slice_ns(Some(profile.mlfq_base_slice_ns));
                Self::set_mlfq_boost_interval_ticks(Some(profile.mlfq_boost_interval_ticks));
                Self::set_mlfq_demote_on_slice_exhaustion(Some(
                    profile.mlfq_demote_on_slice_exhaustion,
                ));
                Self::set_edf_enforce_deadline(Some(profile.edf_enforce_deadline));
                Self::set_edf_default_relative_deadline_ns(Some(
                    profile.edf_default_relative_deadline_ns,
                ));
                Self::set_rt_group_reservation_enabled(Some(profile.rt_group_reservation_enabled));
                Self::set_rt_period_ns(Some(profile.rt_period_ns));
                Self::set_rt_total_utilization_cap_percent(Some(
                    profile.rt_total_utilization_cap_percent,
                ));
                Self::set_rt_max_groups(Some(profile.rt_max_groups));
                Self::set_sched_lottery_tickets_per_priority_level(Some(
                    profile.lottery_tickets_per_priority_level,
                ));
                Self::set_sched_lottery_min_tickets_per_task(Some(
                    profile.lottery_min_tickets_per_task,
                ));
            },
            || {
                Self::set_cfs_min_granularity_ns(None);
                Self::set_cfs_latency_target_ns(None);
                Self::set_mlfq_base_slice_ns(None);
                Self::set_mlfq_boost_interval_ticks(None);
                Self::set_mlfq_demote_on_slice_exhaustion(None);
                Self::set_edf_enforce_deadline(None);
                Self::set_edf_default_relative_deadline_ns(None);
                Self::set_rt_group_reservation_enabled(None);
                Self::set_rt_period_ns(None);
                Self::set_rt_total_utilization_cap_percent(None);
                Self::set_rt_max_groups(None);
                Self::set_sched_lottery_tickets_per_priority_level(None);
                Self::set_sched_lottery_min_tickets_per_task(None);
            },
        );
    }
}
