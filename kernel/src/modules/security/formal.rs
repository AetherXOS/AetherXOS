use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

static FORMAL_REGISTER_CALLS: AtomicU64 = AtomicU64::new(0);
static FORMAL_VERIFY_CALLS: AtomicU64 = AtomicU64::new(0);
static FORMAL_VERIFY_PASSES: AtomicU64 = AtomicU64::new(0);

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
    FORMAL_REGISTER_CALLS.fetch_add(1, Ordering::Relaxed);
    ARTIFACTS.lock().insert(artifact_id, invariant_hash);
}

pub fn verify_security_invariants() -> bool {
    FORMAL_VERIFY_CALLS.fetch_add(1, Ordering::Relaxed);
    let artifacts = ARTIFACTS.lock();
    let ok = !artifacts.is_empty() && artifacts.values().all(|hash| *hash != 0);
    if ok {
        FORMAL_VERIFY_PASSES.fetch_add(1, Ordering::Relaxed);
    }
    ok
}

pub fn formal_stats() -> FormalStats {
    FormalStats {
        register_calls: FORMAL_REGISTER_CALLS.load(Ordering::Relaxed),
        verify_calls: FORMAL_VERIFY_CALLS.load(Ordering::Relaxed),
        verify_passes: FORMAL_VERIFY_PASSES.load(Ordering::Relaxed),
        artifact_count: ARTIFACTS.lock().len(),
    }
}
