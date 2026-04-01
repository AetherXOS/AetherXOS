use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkRuntimeHealthAction {
    None,
    ReinitializeRuntime,
    ForcePollingUntilRecovered,
}

pub(super) struct RuntimeHealthSnapshot {
    pub polls: u64,
    pub poll_errors: u64,
    pub init_errors: u64,
    pub score: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct NetworkRuntimeHealthReport {
    pub polls: u64,
    pub poll_errors: u64,
    pub init_errors: u64,
    pub score: u64,
    pub poll_error_rate_per_mille: u64,
    pub low_poll_activity: bool,
    pub degraded: bool,
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

pub(super) fn evaluate_runtime_health(snapshot: RuntimeHealthSnapshot) -> NetworkRuntimeHealthReport {
    let poll_error_rate_per_mille = if snapshot.polls == 0 {
        0
    } else {
        snapshot
            .poll_errors
            .saturating_mul(1000)
            .saturating_div(snapshot.polls)
    };

    let low_poll_activity = snapshot.polls < 8;
    let degraded = poll_error_rate_per_mille > 120 || snapshot.init_errors > 0 || snapshot.score < 8;

    NetworkRuntimeHealthReport {
        polls: snapshot.polls,
        poll_errors: snapshot.poll_errors,
        init_errors: snapshot.init_errors,
        score: snapshot.score,
        poll_error_rate_per_mille,
        low_poll_activity,
        degraded,
    }
}

pub(super) fn recommended_runtime_action(
    report: NetworkRuntimeHealthReport,
) -> NetworkRuntimeHealthAction {
    if report.init_errors > 0 {
        return NetworkRuntimeHealthAction::ReinitializeRuntime;
    }
    if report.degraded || report.low_poll_activity {
        return NetworkRuntimeHealthAction::ForcePollingUntilRecovered;
    }
    NetworkRuntimeHealthAction::None
}