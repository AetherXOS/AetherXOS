use crate::kernel_runtime::networking::{
    NETWORK_AUTO_POLICY_SWITCH_COOLDOWN, NETWORK_AUTO_POLICY_SWITCH_COUNT,
    NETWORK_SLO_REMEDIATION_ACTIONS, NETWORK_SLO_REMEDIATION_STAGE,
};

pub(super) fn log_network_remediation_dashboard() {
    let thresholds = hypercore::modules::drivers::network_slo_thresholds();
    let slo = hypercore::modules::drivers::network_slo_report();
    let remediation_profile = hypercore::modules::drivers::network_remediation_profile();
    let remediation_tuning =
        hypercore::modules::drivers::remediation_tuning_for_profile(remediation_profile);
    let remediation_stage =
        NETWORK_SLO_REMEDIATION_STAGE.load(core::sync::atomic::Ordering::Relaxed);
    let remediation_actions =
        NETWORK_SLO_REMEDIATION_ACTIONS.load(core::sync::atomic::Ordering::Relaxed);
    let remediation_cooldown =
        NETWORK_AUTO_POLICY_SWITCH_COOLDOWN.load(core::sync::atomic::Ordering::Relaxed);

    hypercore::klog_info!(
        "Network SLO dashboard: thresholds(drop={}permille tx_util={}%% rx_util={}%% io_err={}) current(drop={}permille tx_util={}%% rx_util={}%% io_err={} breaches={})",
        thresholds.max_drop_rate_per_mille,
        thresholds.max_tx_ring_utilization_percent,
        thresholds.max_rx_ring_utilization_percent,
        thresholds.max_driver_io_errors,
        slo.drop_rate_per_mille,
        slo.tx_ring_utilization_percent,
        slo.rx_ring_utilization_percent,
        slo.driver_io_errors,
        slo.breach_count
    );
    hypercore::klog_info!(
        "Network remediation: profile={:?} streak_threshold={} cooldown_base={} jitter_mask={:#x} stage={} actions={} policy_switches={} cooldown_remaining={}",
        remediation_profile,
        remediation_tuning.breach_streak_threshold,
        remediation_tuning.cooldown_base_samples,
        remediation_tuning.cooldown_jitter_mask,
        remediation_stage,
        remediation_actions,
        NETWORK_AUTO_POLICY_SWITCH_COUNT.load(core::sync::atomic::Ordering::Relaxed),
        remediation_cooldown
    );
}
