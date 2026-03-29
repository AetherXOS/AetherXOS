#[derive(Debug, Clone, Copy)]
pub struct Http2ServiceConfig {
    pub max_concurrent_streams: u32,
}

impl Default for Http2ServiceConfig {
    fn default() -> Self {
        Self {
            max_concurrent_streams: 128,
        }
    }
}

pub fn http_feature_enabled() -> bool {
    cfg!(feature = "network_http")
}

#[cfg(feature = "network_http")]
pub use crate::modules::network::http::{HttpResponse, HttpSendfileView};

#[cfg(feature = "network_http")]
pub fn register_static_asset(
    path: &str,
    content_type: &str,
    body: alloc::vec::Vec<u8>,
) -> Result<(), &'static str> {
    crate::modules::libnet::policy::ensure_l7_enabled()?;
    crate::modules::network::http::register_static_asset(path, content_type, body)
}

#[cfg(feature = "network_http")]
pub fn remove_static_asset(path: &str) -> bool {
    if crate::modules::libnet::policy::ensure_l7_enabled().is_err() {
        return false;
    }
    crate::modules::network::http::remove_static_asset(path)
}

#[cfg(feature = "network_http")]
pub fn sendfile(path: &str, offset: usize, max_len: Option<usize>) -> Option<HttpSendfileView> {
    if crate::modules::libnet::policy::ensure_l7_enabled().is_err() {
        return None;
    }
    crate::modules::network::http::sendfile(path, offset, max_len)
}

#[cfg(feature = "network_http")]
pub fn handle_static_request(method: &str, path: &str, if_none_match: Option<u64>) -> HttpResponse {
    if crate::modules::libnet::policy::ensure_l7_enabled().is_err() {
        return HttpResponse {
            status: 403,
            headers: alloc::vec::Vec::new(),
            body: None,
        };
    }
    crate::modules::network::http::handle_static_request(method, path, if_none_match)
}

#[cfg(feature = "network_http")]
pub fn static_asset_count() -> usize {
    if crate::modules::libnet::policy::ensure_l7_enabled().is_err() {
        return 0;
    }
    crate::modules::network::http::static_asset_count()
}

#[cfg(feature = "libnet_l7_http2")]
pub fn http2_marker_types() -> (&'static str, &'static str) {
    let _ = core::any::type_name::<h2::Reason>();
    let _ = core::any::type_name::<hyper::http::Request<()>>();
    ("h2", "hyper")
}

#[cfg(not(feature = "libnet_l7_http2"))]
pub fn http2_marker_types() -> (&'static str, &'static str) {
    ("h2-disabled", "hyper-disabled")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn http_feature_enabled_matches_cfg() {
        if !http_feature_enabled() {
            assert!(!cfg!(feature = "network_http"));
        } else {
            assert!(cfg!(feature = "network_http"));
        }
    }

    #[test_case]
    fn http2_marker_types_returns_expected_strings() {
        let (h2_marker, hyper_marker) = http2_marker_types();
        if cfg!(feature = "libnet_l7_http2") {
            assert_ne!(h2_marker, "h2-disabled");
            assert_ne!(hyper_marker, "hyper-disabled");
        } else {
            assert_eq!(h2_marker, "h2-disabled");
            assert_eq!(hyper_marker, "hyper-disabled");
        }
    }
}
