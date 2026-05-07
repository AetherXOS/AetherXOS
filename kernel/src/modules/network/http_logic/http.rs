#[cfg(feature = "network_http")]
pub use crate::modules::network::{HttpResponse, HttpSendfileView, HttpStaticAsset};

#[cfg(feature = "network_http")]
pub fn register_static_asset(
    path: &str,
    content_type: &str,
    body: alloc::vec::Vec<u8>,
) -> Result<(), &'static str> {
    crate::modules::network::http_register_static_asset(path, content_type, body)
}

#[cfg(feature = "network_http")]
pub fn remove_static_asset(path: &str) -> bool {
    crate::modules::network::http_remove_static_asset(path)
}

#[cfg(feature = "network_http")]
pub fn sendfile(path: &str, offset: usize, max_len: Option<usize>) -> Option<HttpSendfileView> {
    crate::modules::network::http_sendfile(path, offset, max_len)
}

#[cfg(feature = "network_http")]
pub fn handle_static_request(method: &str, path: &str, if_none_match: Option<u64>) -> HttpResponse {
    crate::modules::network::http_handle_static_request(method, path, if_none_match)
}

#[cfg(feature = "network_http")]
pub fn static_asset_count() -> usize {
    crate::modules::network::http_static_asset_count()
}
