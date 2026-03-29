use core::sync::atomic::Ordering;

use super::{
    KernelConfig, NetworkRuntimeProfile, NetworkSloRuntimeConfig, TlsPolicyProfile,
    DEFAULT_LIBNET_ADAPTIVE_HEALTH_LOW_THRESHOLD, DEFAULT_LIBNET_ADAPTIVE_POLL_HIGH_THRESHOLD,
    DEFAULT_LIBNET_ADAPTIVE_QUEUE_DEPTH_DIVISOR, DEFAULT_LIBNET_POLL_INTERVAL_BALANCED,
    DEFAULT_LIBNET_POLL_INTERVAL_LOW_LATENCY, DEFAULT_LIBNET_POLL_INTERVAL_POWERSAVE,
    DEFAULT_LIBNET_POSIX_BLOCKING_RECV_RETRIES, DEFAULT_LIBNET_POSIX_EPHEMERAL_START,
    DEFAULT_LIBNET_POSIX_FD_START, DEFAULT_NETWORK_EPOLL_MAX_EVENTS,
    DEFAULT_NETWORK_EPOLL_MAX_FDS_PER_INSTANCE, DEFAULT_NETWORK_FILTER_RULE_LIMIT,
    DEFAULT_NETWORK_HTTP_ASSET_LIMIT, DEFAULT_NETWORK_LOOPBACK_QUEUE_LIMIT,
    DEFAULT_NETWORK_RUNTIME_POLL_INTERVAL_MIN, DEFAULT_NETWORK_SLO_LOG_INTERVAL_MULTIPLIER,
    DEFAULT_NETWORK_SLO_SAMPLE_INTERVAL, DEFAULT_NETWORK_TCP_QUEUE_LIMIT,
    DEFAULT_NETWORK_TLS_POLICY_PROFILE, DEFAULT_NETWORK_UDP_QUEUE_LIMIT,
    DEFAULT_NETWORK_WIREGUARD_MAX_PEERS, LIBNET_ADAPTIVE_HEALTH_LOW_THRESHOLD_OVERRIDE,
    LIBNET_ADAPTIVE_POLL_HIGH_THRESHOLD_OVERRIDE, LIBNET_ADAPTIVE_QUEUE_DEPTH_DIVISOR_OVERRIDE,
    LIBNET_POLL_INTERVAL_BALANCED_OVERRIDE, LIBNET_POLL_INTERVAL_LOW_LATENCY_OVERRIDE,
    LIBNET_POLL_INTERVAL_POWERSAVE_OVERRIDE, LIBNET_POSIX_BLOCKING_RECV_RETRIES_OVERRIDE,
    LIBNET_POSIX_EPHEMERAL_START_OVERRIDE, LIBNET_POSIX_FD_START_OVERRIDE,
    MAX_LIBNET_ADAPTIVE_HEALTH_LOW_THRESHOLD, MAX_LIBNET_ADAPTIVE_POLL_HIGH_THRESHOLD,
    MAX_LIBNET_ADAPTIVE_QUEUE_DEPTH_DIVISOR, MAX_LIBNET_POLL_INTERVAL,
    MAX_LIBNET_POSIX_BLOCKING_RECV_RETRIES, MAX_LIBNET_POSIX_EPHEMERAL_START,
    MAX_LIBNET_POSIX_FD_START, MAX_NETWORK_EPOLL_MAX_EVENTS,
    MAX_NETWORK_EPOLL_MAX_FDS_PER_INSTANCE, MAX_NETWORK_FILTER_RULE_LIMIT,
    MAX_NETWORK_HTTP_ASSET_LIMIT, MAX_NETWORK_LOOPBACK_QUEUE_LIMIT,
    MAX_NETWORK_RUNTIME_POLL_INTERVAL_MIN, MAX_NETWORK_SLO_LOG_INTERVAL_MULTIPLIER,
    MAX_NETWORK_SLO_SAMPLE_INTERVAL, MAX_NETWORK_TCP_QUEUE_LIMIT, MAX_NETWORK_UDP_QUEUE_LIMIT,
    MAX_NETWORK_WIREGUARD_MAX_PEERS, NETWORK_EPOLL_MAX_EVENTS_OVERRIDE,
    NETWORK_EPOLL_MAX_FDS_PER_INSTANCE_OVERRIDE, NETWORK_FILTER_RULE_LIMIT_OVERRIDE,
    NETWORK_HTTP_ASSET_LIMIT_OVERRIDE, NETWORK_LOOPBACK_QUEUE_LIMIT_OVERRIDE,
    NETWORK_RUNTIME_POLL_INTERVAL_MIN_OVERRIDE, NETWORK_SLO_LOG_INTERVAL_MULTIPLIER_OVERRIDE,
    NETWORK_SLO_SAMPLE_INTERVAL_OVERRIDE, NETWORK_TCP_QUEUE_LIMIT_OVERRIDE,
    NETWORK_TLS_POLICY_PROFILE_OVERRIDE, NETWORK_UDP_QUEUE_LIMIT_OVERRIDE,
    NETWORK_WIREGUARD_MAX_PEERS_OVERRIDE, TLS_POLICY_PROFILE_OVERRIDE_BALANCED,
    TLS_POLICY_PROFILE_OVERRIDE_DEFAULT, TLS_POLICY_PROFILE_OVERRIDE_MINIMAL,
    TLS_POLICY_PROFILE_OVERRIDE_STRICT,
};

impl KernelConfig {
    pub fn libnet_poll_interval_low_latency() -> u64 {
        let override_value = LIBNET_POLL_INTERVAL_LOW_LATENCY_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_LIBNET_POLL_INTERVAL_LOW_LATENCY
        } else {
            override_value.max(1).min(MAX_LIBNET_POLL_INTERVAL)
        }
    }

    pub fn set_libnet_poll_interval_low_latency(value: Option<u64>) {
        LIBNET_POLL_INTERVAL_LOW_LATENCY_OVERRIDE.store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn libnet_poll_interval_balanced() -> u64 {
        let override_value = LIBNET_POLL_INTERVAL_BALANCED_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_LIBNET_POLL_INTERVAL_BALANCED
        } else {
            override_value.max(1).min(MAX_LIBNET_POLL_INTERVAL)
        }
    }

    pub fn set_libnet_poll_interval_balanced(value: Option<u64>) {
        LIBNET_POLL_INTERVAL_BALANCED_OVERRIDE.store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn libnet_poll_interval_powersave() -> u64 {
        let override_value = LIBNET_POLL_INTERVAL_POWERSAVE_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_LIBNET_POLL_INTERVAL_POWERSAVE
        } else {
            override_value.max(1).min(MAX_LIBNET_POLL_INTERVAL)
        }
    }

    pub fn set_libnet_poll_interval_powersave(value: Option<u64>) {
        LIBNET_POLL_INTERVAL_POWERSAVE_OVERRIDE.store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn libnet_adaptive_queue_depth_divisor() -> usize {
        let override_value = LIBNET_ADAPTIVE_QUEUE_DEPTH_DIVISOR_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_LIBNET_ADAPTIVE_QUEUE_DEPTH_DIVISOR
        } else {
            override_value
                .max(1)
                .min(MAX_LIBNET_ADAPTIVE_QUEUE_DEPTH_DIVISOR)
        }
    }

    pub fn set_libnet_adaptive_queue_depth_divisor(value: Option<usize>) {
        LIBNET_ADAPTIVE_QUEUE_DEPTH_DIVISOR_OVERRIDE.store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn libnet_adaptive_health_low_threshold() -> u64 {
        let override_value = LIBNET_ADAPTIVE_HEALTH_LOW_THRESHOLD_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_LIBNET_ADAPTIVE_HEALTH_LOW_THRESHOLD
        } else {
            override_value.min(MAX_LIBNET_ADAPTIVE_HEALTH_LOW_THRESHOLD)
        }
    }

    pub fn set_libnet_adaptive_health_low_threshold(value: Option<u64>) {
        LIBNET_ADAPTIVE_HEALTH_LOW_THRESHOLD_OVERRIDE.store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn libnet_adaptive_poll_high_threshold() -> u64 {
        let override_value = LIBNET_ADAPTIVE_POLL_HIGH_THRESHOLD_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_LIBNET_ADAPTIVE_POLL_HIGH_THRESHOLD
        } else {
            override_value
                .max(1)
                .min(MAX_LIBNET_ADAPTIVE_POLL_HIGH_THRESHOLD)
        }
    }

    pub fn set_libnet_adaptive_poll_high_threshold(value: Option<u64>) {
        LIBNET_ADAPTIVE_POLL_HIGH_THRESHOLD_OVERRIDE.store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn network_runtime_poll_interval_min() -> u64 {
        let override_value = NETWORK_RUNTIME_POLL_INTERVAL_MIN_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_NETWORK_RUNTIME_POLL_INTERVAL_MIN
        } else {
            override_value
                .max(1)
                .min(MAX_NETWORK_RUNTIME_POLL_INTERVAL_MIN)
        }
    }

    pub fn set_network_runtime_poll_interval_min(value: Option<u64>) {
        NETWORK_RUNTIME_POLL_INTERVAL_MIN_OVERRIDE.store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn network_epoll_max_events() -> usize {
        let override_value = NETWORK_EPOLL_MAX_EVENTS_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_NETWORK_EPOLL_MAX_EVENTS
        } else {
            override_value.max(1).min(MAX_NETWORK_EPOLL_MAX_EVENTS)
        }
    }

    pub fn set_network_epoll_max_events(value: Option<usize>) {
        NETWORK_EPOLL_MAX_EVENTS_OVERRIDE.store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn network_epoll_max_fds_per_instance() -> usize {
        let override_value = NETWORK_EPOLL_MAX_FDS_PER_INSTANCE_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_NETWORK_EPOLL_MAX_FDS_PER_INSTANCE
        } else {
            override_value
                .max(1)
                .min(MAX_NETWORK_EPOLL_MAX_FDS_PER_INSTANCE)
        }
    }

    pub fn set_network_epoll_max_fds_per_instance(value: Option<usize>) {
        NETWORK_EPOLL_MAX_FDS_PER_INSTANCE_OVERRIDE.store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn network_loopback_queue_limit() -> usize {
        let override_value = NETWORK_LOOPBACK_QUEUE_LIMIT_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_NETWORK_LOOPBACK_QUEUE_LIMIT
        } else {
            override_value.max(1).min(MAX_NETWORK_LOOPBACK_QUEUE_LIMIT)
        }
    }

    pub fn set_network_loopback_queue_limit(value: Option<usize>) {
        NETWORK_LOOPBACK_QUEUE_LIMIT_OVERRIDE.store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn network_udp_queue_limit() -> usize {
        let override_value = NETWORK_UDP_QUEUE_LIMIT_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_NETWORK_UDP_QUEUE_LIMIT
        } else {
            override_value.max(1).min(MAX_NETWORK_UDP_QUEUE_LIMIT)
        }
    }

    pub fn set_network_udp_queue_limit(value: Option<usize>) {
        NETWORK_UDP_QUEUE_LIMIT_OVERRIDE.store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn network_tcp_queue_limit() -> usize {
        let override_value = NETWORK_TCP_QUEUE_LIMIT_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_NETWORK_TCP_QUEUE_LIMIT
        } else {
            override_value.max(1).min(MAX_NETWORK_TCP_QUEUE_LIMIT)
        }
    }

    pub fn set_network_tcp_queue_limit(value: Option<usize>) {
        NETWORK_TCP_QUEUE_LIMIT_OVERRIDE.store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn network_filter_rule_limit() -> usize {
        let override_value = NETWORK_FILTER_RULE_LIMIT_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_NETWORK_FILTER_RULE_LIMIT
        } else {
            override_value.max(1).min(MAX_NETWORK_FILTER_RULE_LIMIT)
        }
    }

    pub fn set_network_filter_rule_limit(value: Option<usize>) {
        NETWORK_FILTER_RULE_LIMIT_OVERRIDE.store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn network_wireguard_max_peers() -> usize {
        let override_value = NETWORK_WIREGUARD_MAX_PEERS_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_NETWORK_WIREGUARD_MAX_PEERS
        } else {
            override_value.max(1).min(MAX_NETWORK_WIREGUARD_MAX_PEERS)
        }
    }

    pub fn set_network_wireguard_max_peers(value: Option<usize>) {
        NETWORK_WIREGUARD_MAX_PEERS_OVERRIDE.store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn network_http_asset_limit() -> usize {
        let override_value = NETWORK_HTTP_ASSET_LIMIT_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_NETWORK_HTTP_ASSET_LIMIT
        } else {
            override_value.max(1).min(MAX_NETWORK_HTTP_ASSET_LIMIT)
        }
    }

    pub fn set_network_http_asset_limit(value: Option<usize>) {
        NETWORK_HTTP_ASSET_LIMIT_OVERRIDE.store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn network_slo_sample_interval() -> u64 {
        let override_value = NETWORK_SLO_SAMPLE_INTERVAL_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_NETWORK_SLO_SAMPLE_INTERVAL.max(1)
        } else {
            override_value.max(1).min(MAX_NETWORK_SLO_SAMPLE_INTERVAL)
        }
    }

    pub fn set_network_slo_sample_interval(value: Option<u64>) {
        NETWORK_SLO_SAMPLE_INTERVAL_OVERRIDE.store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn network_slo_log_interval_multiplier() -> u64 {
        let override_value = NETWORK_SLO_LOG_INTERVAL_MULTIPLIER_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_NETWORK_SLO_LOG_INTERVAL_MULTIPLIER.max(1)
        } else {
            override_value
                .max(1)
                .min(MAX_NETWORK_SLO_LOG_INTERVAL_MULTIPLIER)
        }
    }

    pub fn set_network_slo_log_interval_multiplier(value: Option<u64>) {
        NETWORK_SLO_LOG_INTERVAL_MULTIPLIER_OVERRIDE.store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn network_slo_runtime_config() -> NetworkSloRuntimeConfig {
        NetworkSloRuntimeConfig {
            sample_interval: Self::network_slo_sample_interval(),
            log_interval_multiplier: Self::network_slo_log_interval_multiplier(),
        }
    }

    pub fn set_network_slo_runtime_config(config: Option<NetworkSloRuntimeConfig>) {
        if let Some(cfg) = config {
            Self::set_network_slo_sample_interval(Some(cfg.sample_interval));
            Self::set_network_slo_log_interval_multiplier(Some(cfg.log_interval_multiplier));
        } else {
            Self::set_network_slo_sample_interval(None);
            Self::set_network_slo_log_interval_multiplier(None);
        }
    }

    pub fn network_tls_policy_profile_name() -> &'static str {
        Self::network_tls_policy_profile().as_str()
    }

    pub fn network_tls_policy_profile() -> TlsPolicyProfile {
        match NETWORK_TLS_POLICY_PROFILE_OVERRIDE.load(Ordering::Relaxed) {
            TLS_POLICY_PROFILE_OVERRIDE_MINIMAL => TlsPolicyProfile::Minimal,
            TLS_POLICY_PROFILE_OVERRIDE_BALANCED => TlsPolicyProfile::Balanced,
            TLS_POLICY_PROFILE_OVERRIDE_STRICT => TlsPolicyProfile::Strict,
            _ => TlsPolicyProfile::from_str(DEFAULT_NETWORK_TLS_POLICY_PROFILE),
        }
    }

    pub fn set_network_tls_policy_profile(value: Option<TlsPolicyProfile>) {
        let encoded = match value {
            None => TLS_POLICY_PROFILE_OVERRIDE_DEFAULT,
            Some(TlsPolicyProfile::Minimal) => TLS_POLICY_PROFILE_OVERRIDE_MINIMAL,
            Some(TlsPolicyProfile::Balanced) => TLS_POLICY_PROFILE_OVERRIDE_BALANCED,
            Some(TlsPolicyProfile::Strict) => TLS_POLICY_PROFILE_OVERRIDE_STRICT,
        };
        NETWORK_TLS_POLICY_PROFILE_OVERRIDE.store(encoded, Ordering::Relaxed);
    }

    pub fn network_runtime_profile() -> NetworkRuntimeProfile {
        NetworkRuntimeProfile {
            tls_policy_profile: Self::network_tls_policy_profile(),
            slo: Self::network_slo_runtime_config(),
        }
    }

    pub fn network_cargo_profile() -> NetworkRuntimeProfile {
        NetworkRuntimeProfile {
            tls_policy_profile: TlsPolicyProfile::from_str(DEFAULT_NETWORK_TLS_POLICY_PROFILE),
            slo: NetworkSloRuntimeConfig {
                sample_interval: DEFAULT_NETWORK_SLO_SAMPLE_INTERVAL.max(1),
                log_interval_multiplier: DEFAULT_NETWORK_SLO_LOG_INTERVAL_MULTIPLIER.max(1),
            },
        }
    }

    pub fn set_network_runtime_profile(value: Option<NetworkRuntimeProfile>) {
        if let Some(profile) = value {
            Self::set_network_tls_policy_profile(Some(profile.tls_policy_profile));
            Self::set_network_slo_runtime_config(Some(profile.slo));
        } else {
            Self::set_network_tls_policy_profile(None);
            Self::set_network_slo_runtime_config(None);
        }
    }

    pub fn libnet_posix_fd_start() -> u32 {
        let override_value = LIBNET_POSIX_FD_START_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_LIBNET_POSIX_FD_START
        } else {
            u32::try_from(override_value)
                .unwrap_or(DEFAULT_LIBNET_POSIX_FD_START)
                .clamp(DEFAULT_LIBNET_POSIX_FD_START, MAX_LIBNET_POSIX_FD_START)
        }
    }

    pub fn set_libnet_posix_fd_start(value: Option<u32>) {
        LIBNET_POSIX_FD_START_OVERRIDE.store(value.unwrap_or(0) as usize, Ordering::Relaxed);
    }

    pub fn libnet_posix_ephemeral_start() -> u16 {
        let override_value = LIBNET_POSIX_EPHEMERAL_START_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_LIBNET_POSIX_EPHEMERAL_START
        } else {
            u16::try_from(override_value)
                .unwrap_or(DEFAULT_LIBNET_POSIX_EPHEMERAL_START)
                .clamp(
                    DEFAULT_LIBNET_POSIX_EPHEMERAL_START,
                    MAX_LIBNET_POSIX_EPHEMERAL_START,
                )
        }
    }

    pub fn set_libnet_posix_ephemeral_start(value: Option<u16>) {
        LIBNET_POSIX_EPHEMERAL_START_OVERRIDE.store(value.unwrap_or(0) as usize, Ordering::Relaxed);
    }

    pub fn libnet_posix_blocking_recv_retries() -> usize {
        let override_value = LIBNET_POSIX_BLOCKING_RECV_RETRIES_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_LIBNET_POSIX_BLOCKING_RECV_RETRIES
        } else {
            override_value
                .max(1)
                .min(MAX_LIBNET_POSIX_BLOCKING_RECV_RETRIES)
        }
    }

    pub fn set_libnet_posix_blocking_recv_retries(value: Option<usize>) {
        LIBNET_POSIX_BLOCKING_RECV_RETRIES_OVERRIDE.store(value.unwrap_or(0), Ordering::Relaxed);
    }
}
