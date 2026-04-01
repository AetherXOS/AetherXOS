use super::super::*;
use super::validation::runtime_policy_snapshot_contract_holds;

pub fn run_runtime_policy_contract_self_test() -> RuntimePolicyContractReport {
    let mut checks = 0u32;
    let mut failures = 0u32;
    let mut last_error_code = 0u32;

    macro_rules! check {
        ($code:expr, $cond:expr, $msg:expr) => {{
            checks = checks.saturating_add(1);
            if !($cond) {
                failures = failures.saturating_add(1);
                last_error_code = $code;
                crate::klog_error!("[RUNTIME POLICY CONTRACT] E{}: {}", $code, $msg);
            }
        }};
    }

    let snapshot = runtime_policy_snapshot();
    check!(
        3201,
        runtime_policy_snapshot_contract_holds(snapshot),
        "runtime policy snapshot must mirror effective virtualization policy and governor"
    );
    check!(
        3202,
        snapshot.drift_sample_interval_ticks > 0
            && snapshot.drift_reapply_cooldown_ticks >= snapshot.drift_sample_interval_ticks,
        "runtime policy drift intervals are inconsistent"
    );
    check!(
        3203,
        drift_reason_name(snapshot.last_drift_reason) != "",
        "runtime policy last drift reason must resolve to a non-empty label"
    );

    if failures == 0 {
        crate::klog_info!("[RUNTIME POLICY CONTRACT] passed checks={}", checks);
    } else {
        crate::klog_error!(
            "[RUNTIME POLICY CONTRACT] failed checks={} failures={} last_error=E{}",
            checks,
            failures,
            last_error_code
        );
    }

    RuntimePolicyContractReport {
        checks,
        failures,
        last_error_code,
    }
}
