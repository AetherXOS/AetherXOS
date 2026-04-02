use super::failover::{activate_runtime_network_driver, try_network_failover_for_io_health};
use crate::kernel_runtime::network_policy_helpers::select_network_failover_target;
use crate::kernel_runtime::networking::{
    NETWORK_AUTO_POLICY_SWITCH_COOLDOWN, NETWORK_AUTO_POLICY_SWITCH_COUNT,
    NETWORK_SLO_BREACH_STREAK, NETWORK_SLO_REMEDIATION_ACTIONS, NETWORK_SLO_REMEDIATION_STAGE,
    NETWORK_SLO_SAMPLE_COUNTER,
};

pub(super) fn maybe_auto_switch_network_driver_on_slo(
    slo: aethercore::modules::drivers::NetworkDriverSloReport,
) -> bool {
    let profile = aethercore::modules::drivers::network_remediation_profile();
    let tuning = aethercore::modules::drivers::remediation_tuning_for_profile(profile);
    let cooldown = NETWORK_AUTO_POLICY_SWITCH_COOLDOWN.load(core::sync::atomic::Ordering::Relaxed);
    if cooldown > 0 {
        NETWORK_AUTO_POLICY_SWITCH_COOLDOWN
            .store(cooldown - 1, core::sync::atomic::Ordering::Relaxed);
    }
    if slo.breach_count == 0 {
        NETWORK_SLO_BREACH_STREAK.store(0, core::sync::atomic::Ordering::Relaxed);
        NETWORK_SLO_REMEDIATION_STAGE.store(0, core::sync::atomic::Ordering::Relaxed);
        return false;
    }

    let streak = NETWORK_SLO_BREACH_STREAK.fetch_add(1, core::sync::atomic::Ordering::Relaxed) + 1;
    let cooldown_remaining =
        NETWORK_AUTO_POLICY_SWITCH_COOLDOWN.load(core::sync::atomic::Ordering::Relaxed);
    if streak < tuning.breach_streak_threshold || cooldown_remaining > 0 {
        return false;
    }

    let max_stage = if tuning.rebind_before_failover { 3 } else { 2 };
    let current_stage = NETWORK_SLO_REMEDIATION_STAGE.load(core::sync::atomic::Ordering::Relaxed);
    let next_stage = core::cmp::min(current_stage.saturating_add(1), max_stage);
    let mut failover_performed = false;
    let applied = match next_stage {
        1 => {
            let target_profile = if slo.tx_ring_breach || slo.rx_ring_breach {
                aethercore::modules::drivers::NetworkPollProfile::Throughput
            } else if slo.drop_rate_breach {
                aethercore::modules::drivers::NetworkPollProfile::Balanced
            } else {
                aethercore::modules::drivers::NetworkPollProfile::LowLatency
            };
            aethercore::modules::drivers::apply_network_poll_profile(target_profile);
            if slo.tx_ring_breach || slo.rx_ring_breach {
                aethercore::modules::drivers::configure_network_ring_limit(2048);
            }
            aethercore::klog_warn!(
                "Network remediation stage=1 profile={:?} poll_profile={:?} breach(drop={},tx={},rx={},io={})",
                profile,
                target_profile,
                slo.drop_rate_breach,
                slo.tx_ring_breach,
                slo.rx_ring_breach,
                slo.io_error_breach
            );
            true
        }
        2 if tuning.rebind_before_failover => {
            let active = aethercore::modules::drivers::active_network_driver();
            let rebind_ok = match active {
                aethercore::modules::drivers::ActiveNetworkDriver::VirtIo => {
                    aethercore::modules::drivers::with_virtio_runtime_driver_mut(|runtime_driver| {
                        super::service::rebind_virtio_driver(runtime_driver)
                    })
                    .unwrap_or(false)
                }
                aethercore::modules::drivers::ActiveNetworkDriver::E1000 => {
                    aethercore::modules::drivers::with_e1000_runtime_driver_mut(|runtime_driver| {
                        super::service::rebind_e1000_driver(runtime_driver)
                    })
                    .unwrap_or(false)
                }
                aethercore::modules::drivers::ActiveNetworkDriver::None => false,
            };
            if rebind_ok {
                aethercore::klog_warn!(
                    "Network remediation stage=2 profile={:?} action=rebind driver={:?}",
                    profile,
                    active
                );
            }
            rebind_ok
        }
        _ => {
            let current = aethercore::modules::drivers::active_network_driver();
            let fallback = select_network_failover_target(
                current,
                aethercore::modules::drivers::has_virtio_runtime_driver(),
                aethercore::modules::drivers::has_e1000_runtime_driver(),
            );
            if fallback == aethercore::modules::drivers::ActiveNetworkDriver::None {
                false
            } else {
                let switched =
                    activate_runtime_network_driver(fallback, "slo-remediation-failover");
                if switched {
                    NETWORK_AUTO_POLICY_SWITCH_COUNT
                        .fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                    failover_performed = true;
                    aethercore::klog_warn!(
                        "Network remediation stage={} profile={:?} action=failover target={:?}",
                        next_stage,
                        profile,
                        fallback
                    );
                }
                switched
            }
        }
    };

    if !applied {
        return false;
    }

    NETWORK_SLO_BREACH_STREAK.store(0, core::sync::atomic::Ordering::Relaxed);
    NETWORK_SLO_REMEDIATION_ACTIONS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    let store_stage = if failover_performed { 0 } else { next_stage };
    NETWORK_SLO_REMEDIATION_STAGE.store(store_stage, core::sync::atomic::Ordering::Relaxed);
    let sample = NETWORK_SLO_SAMPLE_COUNTER.load(core::sync::atomic::Ordering::Relaxed);
    let shift = core::cmp::min(next_stage.saturating_sub(1), 6);
    let base = tuning.cooldown_base_samples.saturating_mul(1u64 << shift);
    let jitter = sample & tuning.cooldown_jitter_mask;
    NETWORK_AUTO_POLICY_SWITCH_COOLDOWN.store(
        base.saturating_add(jitter),
        core::sync::atomic::Ordering::Relaxed,
    );
    true
}
