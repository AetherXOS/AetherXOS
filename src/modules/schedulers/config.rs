use crate::config::KernelConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CfsRuntimeConfig {
    pub min_granularity_ns: u64,
    pub latency_target_ns: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MlfqRuntimeConfig {
    pub num_queues: usize,
    pub base_slice_ns: u64,
    pub boost_interval_ticks: u64,
    pub demote_on_slice_exhaustion: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EdfRuntimeConfig {
    pub enforce_deadline: bool,
    pub default_relative_deadline_ns: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RtRuntimeConfig {
    pub group_reservation_enabled: bool,
    pub period_ns: u64,
    pub total_utilization_cap_percent: u8,
    pub max_groups: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerRuntimeConfig {
    pub cfs: CfsRuntimeConfig,
    pub mlfq: MlfqRuntimeConfig,
    pub edf: EdfRuntimeConfig,
    pub rt: RtRuntimeConfig,
    #[cfg(feature = "sched_lottery")]
    pub lottery: super::lottery::LotteryRuntimeConfig,
}

pub fn cfs_runtime_config() -> CfsRuntimeConfig {
    CfsRuntimeConfig {
        min_granularity_ns: KernelConfig::cfs_min_granularity_ns(),
        latency_target_ns: KernelConfig::cfs_latency_target_ns(),
    }
}

pub fn set_cfs_runtime_config(config: CfsRuntimeConfig) {
    KernelConfig::set_cfs_min_granularity_ns(Some(config.min_granularity_ns));
    KernelConfig::set_cfs_latency_target_ns(Some(config.latency_target_ns));
}

pub fn mlfq_runtime_config() -> MlfqRuntimeConfig {
    MlfqRuntimeConfig {
        num_queues: KernelConfig::mlfq_num_queues(),
        base_slice_ns: KernelConfig::mlfq_base_slice_ns(),
        boost_interval_ticks: KernelConfig::mlfq_boost_interval_ticks(),
        demote_on_slice_exhaustion: KernelConfig::mlfq_demote_on_slice_exhaustion(),
    }
}

pub fn set_mlfq_runtime_config(config: MlfqRuntimeConfig) {
    KernelConfig::set_mlfq_base_slice_ns(Some(config.base_slice_ns));
    KernelConfig::set_mlfq_boost_interval_ticks(Some(config.boost_interval_ticks));
    KernelConfig::set_mlfq_demote_on_slice_exhaustion(Some(config.demote_on_slice_exhaustion));
}

pub fn edf_runtime_config() -> EdfRuntimeConfig {
    EdfRuntimeConfig {
        enforce_deadline: KernelConfig::edf_enforce_deadline(),
        default_relative_deadline_ns: KernelConfig::edf_default_relative_deadline_ns(),
    }
}

pub fn set_edf_runtime_config(config: EdfRuntimeConfig) {
    KernelConfig::set_edf_enforce_deadline(Some(config.enforce_deadline));
    KernelConfig::set_edf_default_relative_deadline_ns(Some(config.default_relative_deadline_ns));
}

pub fn rt_runtime_config() -> RtRuntimeConfig {
    RtRuntimeConfig {
        group_reservation_enabled: KernelConfig::rt_group_reservation_enabled(),
        period_ns: KernelConfig::rt_period_ns(),
        total_utilization_cap_percent: KernelConfig::rt_total_utilization_cap_percent(),
        max_groups: KernelConfig::rt_max_groups(),
    }
}

pub fn set_rt_runtime_config(config: RtRuntimeConfig) {
    KernelConfig::set_rt_group_reservation_enabled(Some(config.group_reservation_enabled));
    KernelConfig::set_rt_period_ns(Some(config.period_ns));
    KernelConfig::set_rt_total_utilization_cap_percent(Some(config.total_utilization_cap_percent));
    KernelConfig::set_rt_max_groups(Some(config.max_groups));
}

pub fn scheduler_runtime_config() -> SchedulerRuntimeConfig {
    SchedulerRuntimeConfig {
        cfs: cfs_runtime_config(),
        mlfq: mlfq_runtime_config(),
        edf: edf_runtime_config(),
        rt: rt_runtime_config(),
        #[cfg(feature = "sched_lottery")]
        lottery: super::lottery::lottery_runtime_config(),
    }
}

pub fn set_scheduler_runtime_config(config: SchedulerRuntimeConfig) {
    set_cfs_runtime_config(config.cfs);
    set_mlfq_runtime_config(config.mlfq);
    set_edf_runtime_config(config.edf);
    set_rt_runtime_config(config.rt);
    #[cfg(feature = "sched_lottery")]
    super::lottery::set_lottery_runtime_config(config.lottery);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn scheduler_runtime_config_roundtrip() {
        KernelConfig::reset_runtime_overrides();
        let original = scheduler_runtime_config();

        let mut updated = original;
        updated.cfs.min_granularity_ns = u64::MAX;
        updated.cfs.latency_target_ns = 1;
        updated.mlfq.base_slice_ns = 123;
        updated.mlfq.boost_interval_ticks = 456;
        updated.mlfq.demote_on_slice_exhaustion = !original.mlfq.demote_on_slice_exhaustion;
        updated.edf.enforce_deadline = !original.edf.enforce_deadline;
        updated.edf.default_relative_deadline_ns = u64::MAX;
        updated.rt.group_reservation_enabled = !original.rt.group_reservation_enabled;
        updated.rt.period_ns = u64::MAX;
        updated.rt.total_utilization_cap_percent = u8::MAX;
        updated.rt.max_groups = usize::MAX;
        #[cfg(feature = "sched_lottery")]
        {
            updated.lottery.initial_seed = 0x1234_5678;
            updated.lottery.tickets_per_priority_level = 17;
            updated.lottery.min_tickets_per_task = 7;
            updated.lottery.lcg_multiplier = 8;
            updated.lottery.lcg_increment = 0;
        }
        set_scheduler_runtime_config(updated);

        let after = scheduler_runtime_config();
        assert_eq!(after.cfs.min_granularity_ns, 10_000_000_000);
        assert_eq!(after.cfs.latency_target_ns, 10_000_000_000);
        assert_eq!(after.mlfq.base_slice_ns, 123);
        assert_eq!(after.mlfq.boost_interval_ticks, 456);
        assert_eq!(
            after.mlfq.demote_on_slice_exhaustion,
            updated.mlfq.demote_on_slice_exhaustion
        );
        assert_eq!(after.edf.enforce_deadline, updated.edf.enforce_deadline);
        assert_eq!(after.edf.default_relative_deadline_ns, 60_000_000_000);
        assert_eq!(
            after.rt.group_reservation_enabled,
            updated.rt.group_reservation_enabled
        );
        assert_eq!(after.rt.period_ns, 60_000_000_000);
        assert_eq!(after.rt.total_utilization_cap_percent, 100);
        assert_eq!(after.rt.max_groups, 65_536);
        #[cfg(feature = "sched_lottery")]
        {
            assert_eq!(after.lottery.initial_seed, 0x1234_5678);
            assert_eq!(after.lottery.tickets_per_priority_level, 17);
            assert_eq!(after.lottery.min_tickets_per_task, 7);
            assert_eq!(after.lottery.lcg_multiplier % 2, 1);
            assert_eq!(after.lottery.lcg_increment % 2, 1);
        }

        KernelConfig::reset_runtime_overrides();
        assert_eq!(scheduler_runtime_config(), original);
    }
}
