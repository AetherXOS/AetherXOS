#[cfg(all(feature = "drivers", feature = "networking"))]
pub(crate) fn service_network_runtime() {
    use crate::kernel_runtime::network_remediation::{
        maybe_auto_switch_network_driver_on_slo, service_registered_network_driver_io,
    };
    use crate::kernel_runtime::networking::{
        network_slo_log_interval_multiplier, network_slo_sample_interval,
        NETWORK_AUTO_POLICY_SWITCH_COUNT, NETWORK_SLO_LAST_LOG_SAMPLE, NETWORK_SLO_SAMPLE_COUNTER,
    };
    use core::sync::atomic::Ordering;

    if hypercore::modules::drivers::has_active_network_driver() {
        if !service_registered_network_driver_io() {
            hypercore::modules::drivers::service_network_queues();
        }

        let sample = NETWORK_SLO_SAMPLE_COUNTER
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_add(1);

        let sample_interval = network_slo_sample_interval();
        if sample % sample_interval == 0 {
            let slo = hypercore::modules::drivers::network_slo_report();
            let log_period = sample_interval.saturating_mul(network_slo_log_interval_multiplier());
            let last_log = NETWORK_SLO_LAST_LOG_SAMPLE.load(Ordering::Relaxed);
            let should_log_now = sample.saturating_sub(last_log) >= log_period;

            if slo.breach_count > 0 && should_log_now {
                NETWORK_SLO_LAST_LOG_SAMPLE.store(sample, Ordering::Relaxed);
                hypercore::klog_warn!(
                    "[NET SLO] driver={:?} drop={}‰ tx={}% rx={}% io_err={} breaches={}",
                    slo.driver,
                    slo.drop_rate_per_mille,
                    slo.tx_ring_utilization_percent,
                    slo.rx_ring_utilization_percent,
                    slo.driver_io_errors,
                    slo.breach_count
                );
            }

            let switched = maybe_auto_switch_network_driver_on_slo(slo);
            if switched && should_log_now {
                let switches = NETWORK_AUTO_POLICY_SWITCH_COUNT.load(Ordering::Relaxed);
                let profile = hypercore::modules::drivers::network_remediation_profile();
                let tuning = hypercore::modules::drivers::remediation_tuning_for_profile(profile);
                hypercore::klog_warn!(
                    "[NET SLO] auto-switch #{} profile={:?} cooldown={} jitter={:#x}",
                    switches,
                    profile,
                    tuning.cooldown_base_samples,
                    tuning.cooldown_jitter_mask
                );
            }
        }
    }
}
