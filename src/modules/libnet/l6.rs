#[cfg(feature = "network_https")]
pub fn install_tls_server_config(config: alloc::sync::Arc<rustls::ServerConfig>) {
    if crate::modules::libnet::policy::ensure_l6_enabled().is_err() {
        return;
    }
    crate::modules::network::https::https_install_server_config(config);
}

#[cfg(feature = "network_https")]
pub fn terminate_tls_record(record: &[u8], out: &mut [u8]) -> Result<usize, &'static str> {
    crate::modules::libnet::policy::ensure_l6_enabled()?;
    crate::modules::network::https::https_terminate_tls_record(record, out)
}

pub fn tls_feature_enabled() -> bool {
    cfg!(feature = "network_https")
}

#[cfg(feature = "network_https")]
pub fn tls_stats() -> crate::modules::network::https::HttpsStats {
    crate::modules::network::https::https_stats()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn tls_stats_matches_feature() {
        if !tls_feature_enabled() {
            assert!(!cfg!(feature = "network_https"));
        } else {
            assert!(cfg!(feature = "network_https"));
        }
    }
}
