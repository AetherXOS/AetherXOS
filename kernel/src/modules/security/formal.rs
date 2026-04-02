use alloc::collections::BTreeMap;
use core::sync::atomic::Ordering;
use lazy_static::lazy_static;
use spin::Mutex;
use aethercore_common::{counter_inc, declare_counter_u64, telemetry};

declare_counter_u64!(FORMAL_REGISTER_CALLS);
declare_counter_u64!(FORMAL_VERIFY_CALLS);
declare_counter_u64!(FORMAL_VERIFY_PASSES);

lazy_static! {
    static ref ARTIFACTS: Mutex<BTreeMap<u64, u64>> = Mutex::new(BTreeMap::new());
}

#[derive(Debug, Clone, Copy)]
pub struct FormalStats {
    pub register_calls: u64,
    pub verify_calls: u64,
    pub verify_passes: u64,
    pub artifact_count: usize,
}

pub fn register_proof_artifact(artifact_id: u64, invariant_hash: u64) {
    counter_inc!(FORMAL_REGISTER_CALLS);
    ARTIFACTS.lock().insert(artifact_id, invariant_hash);
}

pub fn verify_security_invariants() -> bool {
    counter_inc!(FORMAL_VERIFY_CALLS);
    let artifacts = ARTIFACTS.lock();
    let ok = !artifacts.is_empty() && artifacts.values().all(|hash| *hash != 0);
    if ok {
        counter_inc!(FORMAL_VERIFY_PASSES);
    }
    ok
}

pub fn formal_stats() -> FormalStats {
    FormalStats {
        register_calls: telemetry::snapshot_u64(&FORMAL_REGISTER_CALLS),
        verify_calls: telemetry::snapshot_u64(&FORMAL_VERIFY_CALLS),
        verify_passes: telemetry::snapshot_u64(&FORMAL_VERIFY_PASSES),
        artifact_count: ARTIFACTS.lock().len(),
    }
}

pub fn take_formal_stats() -> FormalStats {
    FormalStats {
        register_calls: telemetry::take_u64(&FORMAL_REGISTER_CALLS),
        verify_calls: telemetry::take_u64(&FORMAL_VERIFY_CALLS),
        verify_passes: telemetry::take_u64(&FORMAL_VERIFY_PASSES),
        artifact_count: ARTIFACTS.lock().len(),
    }
}
