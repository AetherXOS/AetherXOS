//! Core config validation — rebalance, watchdog, IRQ storm, affinity.

use crate::build_cfg::config_types::CoreConfig;

const VALID_IDLE_STRATEGIES: &[&str] = &["Halt", "SpinWait", "Mwait"];
const VALID_PANIC_ACTIONS: &[&str] = &["Halt", "Reboot", "Dump"];
const VALID_WATCHDOG_ACTIONS: &[&str] = &["Halt", "Reboot", "Log"];
const VALID_AFFINITY_POLICIES: &[&str] = &["PreferLocal", "Strict", "Any"];
const MAX_REBALANCE_INTERVAL: u64 = 1_000_000;
const MAX_REBALANCE_BATCH: usize = 1024;
const MAX_CRASH_LOG_CAPACITY: usize = 4096;

pub fn validate(c: &CoreConfig) -> Vec<String> {
    let mut e = Vec::new();

    if !VALID_IDLE_STRATEGIES.contains(&c.idle_strategy.as_str()) {
        e.push(format!(
            "core.idle_strategy '{}' invalid, expected one of {:?}",
            c.idle_strategy, VALID_IDLE_STRATEGIES
        ));
    }
    if !VALID_PANIC_ACTIONS.contains(&c.panic_action.as_str()) {
        e.push(format!(
            "core.panic_action '{}' invalid, expected one of {:?}",
            c.panic_action, VALID_PANIC_ACTIONS
        ));
    }
    if !VALID_WATCHDOG_ACTIONS.contains(&c.soft_watchdog_action.as_str()) {
        e.push(format!(
            "core.soft_watchdog_action '{}' invalid, expected one of {:?}",
            c.soft_watchdog_action, VALID_WATCHDOG_ACTIONS
        ));
    }
    if !VALID_AFFINITY_POLICIES.contains(&c.affinity_policy.as_str()) {
        e.push(format!(
            "core.affinity_policy '{}' invalid, expected one of {:?}",
            c.affinity_policy, VALID_AFFINITY_POLICIES
        ));
    }
    if c.enable_periodic_rebalance {
        if c.rebalance_interval_ticks == 0 || c.rebalance_interval_ticks > MAX_REBALANCE_INTERVAL {
            e.push(format!(
                "core.rebalance_interval_ticks {} out of range [1, {}]",
                c.rebalance_interval_ticks, MAX_REBALANCE_INTERVAL
            ));
        }
        if c.rebalance_batch_size == 0 || c.rebalance_batch_size > MAX_REBALANCE_BATCH {
            e.push(format!(
                "core.rebalance_batch_size {} out of range [1, {}]",
                c.rebalance_batch_size, MAX_REBALANCE_BATCH
            ));
        }
    }
    if c.enable_interrupt_storm_protection && c.irq_storm_threshold == 0 {
        e.push("core.irq_storm_threshold must be > 0 when storm protection is enabled".to_string());
    }
    if c.crash_log_capacity == 0 || c.crash_log_capacity > MAX_CRASH_LOG_CAPACITY {
        e.push(format!(
            "core.crash_log_capacity {} out of range [1, {}]",
            c.crash_log_capacity, MAX_CRASH_LOG_CAPACITY
        ));
    }

    e
}
