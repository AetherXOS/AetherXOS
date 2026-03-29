#[path = "network_ops/policy_ops.rs"]
mod policy_ops;
#[path = "network_ops/runtime_ops.rs"]
mod runtime_ops;

pub(crate) use policy_ops::{
    sys_get_network_alert_report, sys_set_network_alert_thresholds,
    sys_set_network_backpressure_policy,
};
pub(crate) use runtime_ops::{
    sys_get_network_stats, sys_network_force_poll, sys_network_reinitialize,
    sys_network_reset_stats, sys_set_network_polling,
};
