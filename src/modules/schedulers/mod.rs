#[cfg(feature = "sched_cfs")]
pub mod cfs;
#[cfg(feature = "sched_cooperative")]
pub mod cooperative;
#[cfg(feature = "sched_edf")]
pub mod edf;
#[cfg(feature = "sched_energy_aware")]
pub mod energy_aware;
#[cfg(feature = "sched_fifo")]
pub mod fifo;
#[cfg(feature = "sched_gang")]
pub mod gang;
#[cfg(feature = "sched_lifo")]
pub mod lifo;
#[cfg(feature = "sched_lottery")]
pub mod lottery;
#[cfg(feature = "sched_mlfq")]
pub mod mlfq;
#[cfg(feature = "sched_round_robin")]
pub mod round_robin;
#[cfg(feature = "sched_weighted_round_robin")]
pub mod weighted_round_robin;

#[cfg(feature = "sched_cfs")]
pub use cfs::CFS;
#[cfg(feature = "sched_cooperative")]
pub use cooperative::Cooperative;
#[cfg(feature = "sched_edf")]
pub use edf::EDF;
#[cfg(feature = "sched_energy_aware")]
pub use energy_aware::{energy_aware_stats, pick_cpu_by_efficiency, EnergyAwareStats};
#[cfg(feature = "sched_fifo")]
pub use fifo::FIFO;
#[cfg(feature = "sched_gang")]
pub use gang::{gang_assign_task, gang_create, gang_pick_members, gang_stats, GangStats};
#[cfg(feature = "sched_lifo")]
pub use lifo::LIFO;
#[cfg(feature = "sched_lottery")]
pub use lottery::Lottery;
#[cfg(feature = "sched_lottery")]
pub use lottery::{
    lottery_initial_seed, lottery_lcg_increment, lottery_lcg_multiplier,
    lottery_min_tickets_per_task, lottery_runtime_config, lottery_tickets_per_priority_level,
    set_lottery_initial_seed, set_lottery_lcg_increment, set_lottery_lcg_multiplier,
    set_lottery_min_tickets_per_task, set_lottery_runtime_config,
    set_lottery_tickets_per_priority_level, LotteryRuntimeConfig,
};
#[cfg(feature = "sched_mlfq")]
pub use mlfq::MLFQ;
#[cfg(feature = "sched_round_robin")]
pub use round_robin::RoundRobin;
#[cfg(feature = "sched_weighted_round_robin")]
pub use weighted_round_robin::WeightedRoundRobin;

#[cfg(feature = "sched_cfs")]
pub type MuQSS = CFS;
#[cfg(feature = "sched_cfs")]
pub type EEVDF = CFS;
#[cfg(feature = "sched_edf")]
pub type RealTimeHard = EDF;
#[cfg(feature = "sched_edf")]
pub type RealTimeSoft = EDF;
#[cfg(feature = "sched_cooperative")]
pub type Idle = Cooperative;
#[cfg(feature = "sched_cfs")]
pub type Batch = CFS;
#[cfg(feature = "sched_cooperative")]
pub type UserSpace = Cooperative;

pub mod selector {
    use super::*;

    #[cfg(all(feature = "sched_round_robin", param_scheduler = "RoundRobin"))]
    pub type ActiveScheduler = RoundRobin;
    #[cfg(all(
        feature = "sched_weighted_round_robin",
        param_scheduler = "WeightedRoundRobin"
    ))]
    pub type ActiveScheduler = WeightedRoundRobin;
    #[cfg(all(feature = "sched_fifo", param_scheduler = "FIFO"))]
    pub type ActiveScheduler = FIFO;
    #[cfg(all(feature = "sched_lifo", param_scheduler = "LIFO"))]
    pub type ActiveScheduler = LIFO;
    #[cfg(all(feature = "sched_cooperative", param_scheduler = "Cooperative"))]
    pub type ActiveScheduler = Cooperative;
    #[cfg(all(feature = "sched_lottery", param_scheduler = "Lottery"))]
    pub type ActiveScheduler = Lottery;

    #[cfg(all(feature = "sched_cfs", param_scheduler = "CFS"))]
    pub type ActiveScheduler = CFS;
    #[cfg(all(feature = "sched_mlfq", param_scheduler = "MLFQ"))]
    pub type ActiveScheduler = MLFQ;
    #[cfg(all(feature = "sched_edf", param_scheduler = "EDF"))]
    pub type ActiveScheduler = EDF;

    #[cfg(not(any(
        all(feature = "sched_round_robin", param_scheduler = "RoundRobin"),
        all(
            feature = "sched_weighted_round_robin",
            param_scheduler = "WeightedRoundRobin"
        ),
        all(feature = "sched_fifo", param_scheduler = "FIFO"),
        all(feature = "sched_lifo", param_scheduler = "LIFO"),
        all(feature = "sched_cooperative", param_scheduler = "Cooperative"),
        all(feature = "sched_lottery", param_scheduler = "Lottery"),
        all(feature = "sched_cfs", param_scheduler = "CFS"),
        all(feature = "sched_mlfq", param_scheduler = "MLFQ"),
        all(feature = "sched_edf", param_scheduler = "EDF")
    )))]
    #[cfg(feature = "sched_round_robin")]
    pub type ActiveScheduler = RoundRobin;

    #[cfg(not(any(
        all(feature = "sched_round_robin", param_scheduler = "RoundRobin"),
        all(
            feature = "sched_weighted_round_robin",
            param_scheduler = "WeightedRoundRobin"
        ),
        all(feature = "sched_fifo", param_scheduler = "FIFO"),
        all(feature = "sched_lifo", param_scheduler = "LIFO"),
        all(feature = "sched_cooperative", param_scheduler = "Cooperative"),
        all(feature = "sched_lottery", param_scheduler = "Lottery"),
        all(feature = "sched_cfs", param_scheduler = "CFS"),
        all(feature = "sched_mlfq", param_scheduler = "MLFQ"),
        all(feature = "sched_edf", param_scheduler = "EDF"),
        feature = "sched_round_robin"
    )))]
    #[cfg(feature = "sched_cooperative")]
    pub type ActiveScheduler = Cooperative;
}

pub mod config;
pub use config::{
    cfs_runtime_config, edf_runtime_config, mlfq_runtime_config, rt_runtime_config,
    scheduler_runtime_config, set_cfs_runtime_config, set_edf_runtime_config,
    set_mlfq_runtime_config, set_rt_runtime_config, set_scheduler_runtime_config, CfsRuntimeConfig,
    EdfRuntimeConfig, MlfqRuntimeConfig, RtRuntimeConfig, SchedulerRuntimeConfig,
};
