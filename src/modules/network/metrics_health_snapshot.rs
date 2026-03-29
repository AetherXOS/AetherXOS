use super::*;

pub(super) struct RuntimeHealthSnapshot {
    pub polls: u64,
    pub poll_errors: u64,
    pub init_errors: u64,
    pub score: u64,
}

pub(super) fn collect_runtime_health_snapshot() -> RuntimeHealthSnapshot {
    let polls = SMOLTCP_POLLS.load(Ordering::Relaxed);
    let poll_errors = SMOLTCP_POLL_ERRORS.load(Ordering::Relaxed);
    let init_errors = SMOLTCP_INIT_ERRORS.load(Ordering::Relaxed);
    let score = polls
        .saturating_sub(poll_errors)
        .saturating_sub(init_errors.saturating_mul(4));

    RuntimeHealthSnapshot {
        polls,
        poll_errors,
        init_errors,
        score,
    }
}