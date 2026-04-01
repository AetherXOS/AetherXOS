use super::super::*;

pub(crate) fn sys_set_network_backpressure_policy(
    loopback: usize,
    udp: usize,
    tcp: usize,
) -> usize {
    SYSCALL_NETWORK_BACKPRESSURE_POLICY_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) =
        require_control_plane_access(crate::modules::security::RESOURCE_NETWORK_CONTROL)
    {
        return err;
    }

    #[cfg(feature = "networking")]
    {
        let Some(loopback_mode) = BackpressurePolicyMode::from_usize(loopback) else {
            return invalid_arg();
        };

        #[cfg(feature = "network_transport")]
        let Some(udp_mode) = BackpressurePolicyMode::from_usize(udp) else {
            return invalid_arg();
        };
        #[cfg(feature = "network_transport")]
        let Some(tcp_mode) = BackpressurePolicyMode::from_usize(tcp) else {
            return invalid_arg();
        };

        #[cfg(not(feature = "network_transport"))]
        {
            if udp != 0 || tcp != 0 {
                return invalid_arg();
            }
        }

        crate::modules::network::bridge::set_backpressure_policy_table(
            crate::modules::network::bridge::BackpressurePolicyTable {
                loopback: loopback_mode.to_network(),
                #[cfg(feature = "network_transport")]
                udp: udp_mode.to_network(),
                #[cfg(feature = "network_transport")]
                tcp: tcp_mode.to_network(),
            },
        );
        0
    }

    #[cfg(not(feature = "networking"))]
    {
        let _ = (loopback, udp, tcp);
        invalid_arg()
    }
}

pub(crate) fn sys_set_network_alert_thresholds(
    min_health_score: usize,
    max_drops: usize,
    max_queue_high_water: usize,
) -> usize {
    SYSCALL_NETWORK_ALERT_THRESHOLDS_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) =
        require_control_plane_access(crate::modules::security::RESOURCE_NETWORK_CONTROL)
    {
        return err;
    }

    #[cfg(feature = "networking")]
    {
        crate::modules::network::bridge::set_network_alert_thresholds(
            crate::modules::network::bridge::NetworkAlertThresholds {
                min_health_score: min_health_score as u64,
                max_drops: max_drops as u64,
                max_queue_high_water: max_queue_high_water as u64,
            },
        );
        0
    }

    #[cfg(not(feature = "networking"))]
    {
        let _ = (min_health_score, max_drops, max_queue_high_water);
        invalid_arg()
    }
}

pub(crate) fn sys_get_network_alert_report(ptr: usize, len: usize) -> usize {
    SYSCALL_NETWORK_ALERT_REPORT_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) = require_control_plane_access(crate::modules::security::RESOURCE_NETWORK_STATS)
    {
        return err;
    }

    #[cfg(feature = "networking")]
    {
        let report = crate::modules::network::bridge::evaluate_network_alerts();
        write_user_words(
            ptr,
            len,
            [
                report.health_breach as usize,
                report.drops_breach as usize,
                report.queue_breach as usize,
                report.breach_count as usize,
            ],
        )
    }

    #[cfg(not(feature = "networking"))]
    {
        let _ = (ptr, len);
        invalid_arg()
    }
}
