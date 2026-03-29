use super::*;

pub(crate) static DRIFT_SAMPLE_CALLS: AtomicU64 = AtomicU64::new(0);
pub(crate) static DRIFT_EVENTS: AtomicU64 = AtomicU64::new(0);
pub(crate) static DRIFT_REAPPLY_CALLS: AtomicU64 = AtomicU64::new(0);
pub(crate) static DRIFT_REAPPLY_SUPPRESSED_COOLDOWN: AtomicU64 = AtomicU64::new(0);
pub(crate) static LAST_DRIFT_REASON: AtomicU64 = AtomicU64::new(DriftReasonCode::None as u64);
pub(crate) static LAST_DRIFT_SAMPLED_TICK: AtomicU64 = AtomicU64::new(0);
pub(crate) static LAST_REAPPLY_TICK: AtomicU64 = AtomicU64::new(0);
pub(crate) static LAST_DRIVER_WAIT_TIMEOUT_TOTAL: AtomicU64 = AtomicU64::new(0);
pub(crate) static LAST_DRIVER_WAIT_TIMEOUT_DELTA: AtomicU64 = AtomicU64::new(0);
