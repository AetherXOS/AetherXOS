//! Scheduler config validation — strategy, per-scheduler ranges.

use crate::build_cfg::config_types::SchedulerConfig;

const VALID_STRATEGIES: &[&str] = &[
    "RoundRobin",
    "CFS",
    "EDF",
    "FIFO",
    "Cooperative",
    "Lottery",
    "WeightedRoundRobin",
    "LIFO",
    "MLFQ",
    "Idle",
    "MuQSS",
    "EEVDF",
    "RealTimeHard",
    "RealTimeSoft",
    "Batch",
    "UserSpace",
];

pub fn validate(c: &SchedulerConfig) -> Vec<String> {
    let mut e = Vec::new();

    if !VALID_STRATEGIES.contains(&c.strategy.as_str()) {
        e.push(format!(
            "scheduler.strategy '{}' invalid, expected one of {:?}",
            c.strategy, VALID_STRATEGIES
        ));
    }
    if c.priority_levels == 0 || c.priority_levels > 256 {
        e.push(format!(
            "scheduler.priority_levels {} out of range [1, 256]",
            c.priority_levels
        ));
    }

    if let Some(rr) = &c.round_robin {
        if rr.max_tasks == 0 || rr.max_tasks > 65536 {
            e.push(format!(
                "scheduler.round_robin.max_tasks {} out of range [1, 65536]",
                rr.max_tasks
            ));
        }
        if rr.default_slice_ns < 100_000 || rr.default_slice_ns > 10_000_000_000 {
            e.push(format!(
                "scheduler.round_robin.default_slice_ns {} out of range [100000, 10000000000]",
                rr.default_slice_ns
            ));
        }
    }

    if let Some(cfs) = &c.cfs {
        if cfs.min_granularity_ns == 0 || cfs.min_granularity_ns > 10_000_000_000 {
            e.push(format!(
                "scheduler.cfs.min_granularity_ns {} out of range [1, 10000000000]",
                cfs.min_granularity_ns
            ));
        }
        if cfs.latency_target_ns == 0 || cfs.latency_target_ns > 60_000_000_000 {
            e.push(format!(
                "scheduler.cfs.latency_target_ns {} out of range [1, 60000000000]",
                cfs.latency_target_ns
            ));
        }
        if cfs.min_granularity_ns > cfs.latency_target_ns {
            e.push("scheduler.cfs.min_granularity_ns must be <= latency_target_ns".to_string());
        }
    }

    if let Some(edf) = &c.edf {
        if edf.max_deadlines == 0 || edf.max_deadlines > 65536 {
            e.push(format!(
                "scheduler.edf.max_deadlines {} out of range [1, 65536]",
                edf.max_deadlines
            ));
        }
        if edf.default_relative_deadline_ns == 0 {
            e.push("scheduler.edf.default_relative_deadline_ns must be > 0".to_string());
        }
    }

    if let Some(mlfq) = &c.mlfq {
        if mlfq.num_queues == 0 || mlfq.num_queues > 64 {
            e.push(format!(
                "scheduler.mlfq.num_queues {} out of range [1, 64]",
                mlfq.num_queues
            ));
        }
        if mlfq.base_slice_ns == 0 {
            e.push("scheduler.mlfq.base_slice_ns must be > 0".to_string());
        }
        if mlfq.boost_interval_ticks == 0 {
            e.push("scheduler.mlfq.boost_interval_ticks must be > 0".to_string());
        }
    }

    if let Some(rt) = &c.rt {
        if rt.total_utilization_cap_percent == 0 || rt.total_utilization_cap_percent > 100 {
            e.push(format!(
                "scheduler.rt.total_utilization_cap_percent {} out of range [1, 100]",
                rt.total_utilization_cap_percent
            ));
        }
        if rt.period_ns == 0 {
            e.push("scheduler.rt.period_ns must be > 0".to_string());
        }
        if rt.max_groups == 0 || rt.max_groups > 65536 {
            e.push(format!(
                "scheduler.rt.max_groups {} out of range [1, 65536]",
                rt.max_groups
            ));
        }
    }

    e
}
