//! Governor config validation — watchdog, RT bounds, spin limits.

use crate::build_cfg::config_types::GovernorConfig;

pub fn validate(c: &GovernorConfig) -> Vec<String> {
    let mut e = Vec::new();

    if c.watchdog_hard_stall_ns == 0 {
        e.push("governor.watchdog_hard_stall_ns must be > 0".to_string());
    }
    if c.rt_force_min_ticks == 0 {
        e.push("governor.rt_force_min_ticks must be > 0".to_string());
    }
    if c.rt_deadline_burst_threshold == 0 {
        e.push("governor.rt_deadline_burst_threshold must be > 0".to_string());
    }
    if c.irqsafe_mutex_deadlock_spin_limit == 0 {
        e.push("governor.irqsafe_mutex_deadlock_spin_limit must be > 0".to_string());
    }
    if c.power_runqueue_saturation_limit == 0 {
        e.push("governor.power_runqueue_saturation_limit must be > 0".to_string());
    }
    if c.load_balance_percentile_window == 0 {
        e.push("governor.load_balance_percentile_window must be > 0".to_string());
    }
    if c.runtime_policy_drift_sample_interval_ticks == 0 {
        e.push("governor.runtime_policy_drift_sample_interval_ticks must be > 0".to_string());
    }
    if c.runtime_policy_drift_reapply_cooldown_ticks == 0 {
        e.push("governor.runtime_policy_drift_reapply_cooldown_ticks must be > 0".to_string());
    }

    e
}
