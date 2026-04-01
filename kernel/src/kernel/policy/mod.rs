use crate::hal::common::virt::{
    current_virtualization_runtime_governor, GOVERNOR_BIAS_AGGRESSIVE, GOVERNOR_BIAS_RELAXED,
};
use core::sync::atomic::{AtomicU64, Ordering};

mod apply;
mod contract;
mod drift;
mod evaluation;
mod preset;
mod sources;
mod state;
mod thresholds;
mod types;

pub use self::apply::apply_runtime_policy_preset;
pub use self::contract::run_runtime_policy_contract_self_test;
#[cfg(test)]
pub(crate) use self::contract::runtime_policy_snapshot_contract_holds;
#[cfg(test)]
pub(crate) use self::drift::can_reapply_now;
pub use self::drift::{runtime_policy_snapshot, sample_policy_drift_if_due};
pub use self::preset::{runtime_policy_preset, set_runtime_policy_preset};
use self::thresholds::{drift_threshold_profile, pressure_exceeds_threshold};
#[cfg(test)]
pub(crate) use self::thresholds::{
    governor_adjusted_drift_limit, governor_adjusted_driver_wait_limit,
};
pub use self::types::{
    CoreRuntimePolicyDriftReport, CoreRuntimePolicyPreset, CoreRuntimePolicySnapshot,
    RuntimePolicyContractReport,
};
use self::types::{DriftReasonCode, DriftThresholdProfile};

pub fn drift_reason_name(reason: u8) -> &'static str {
    DriftReasonCode::from_u8(reason).map(|r| r.name()).unwrap_or("unknown")
}

#[cfg(test)]
mod tests;
