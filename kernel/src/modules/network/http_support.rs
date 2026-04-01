#[cfg(feature = "network_http")]
use super::*;
#[cfg(feature = "network_http")]
use alloc::format;
#[cfg(feature = "network_http")]
use alloc::sync::Arc;
#[cfg(feature = "network_http")]
use state::HTTP_STATIC_ASSETS;

#[cfg(feature = "network_http")]
fn compute_asset_etag(path: &str, body_len: usize) -> u64 {
    let mut hash = 1469598103934665603u64;
    for b in path.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash ^ (body_len as u64)
}

#[cfg(feature = "network_http")]
pub fn http_register_static_asset(
    path: &str,
    content_type: &str,
    body: Vec<u8>,
) -> Result<(), &'static str> {
    if path.is_empty() || !path.starts_with('/') {
        return Err("invalid http path");
    }
    HTTP_REGISTER_CALLS.fetch_add(1, Ordering::Relaxed);
    let mut assets = HTTP_STATIC_ASSETS.lock();
    if assets.len() >= crate::config::KernelConfig::network_http_asset_limit()
        && !assets.contains_key(path)
    {
        return Err("http asset registry full");
    }
    let etag = compute_asset_etag(path, body.len());
    let asset = state::HttpStaticAsset {
        path: path.to_string(),
        content_type: content_type.to_string(),
        body: Arc::new(body),
        etag,
    };
    assets.insert(path.to_string(), asset);
    Ok(())
}

#[cfg(feature = "network_http")]
pub fn http_remove_static_asset(path: &str) -> bool {
    HTTP_REMOVE_CALLS.fetch_add(1, Ordering::Relaxed);
    HTTP_STATIC_ASSETS.lock().remove(path).is_some()
}

#[cfg(feature = "network_http")]
pub fn http_sendfile(
    path: &str,
    offset: usize,
    max_len: Option<usize>,
) -> Option<state::HttpSendfileView> {
    HTTP_SENDFILE_CALLS.fetch_add(1, Ordering::Relaxed);
    let assets = HTTP_STATIC_ASSETS.lock();
    let asset = assets.get(path)?;
    let total = asset.body.len();
    if offset >= total {
        return Some(state::HttpSendfileView {
            body: Arc::clone(&asset.body),
            offset: total,
            len: 0,
        });
    }
    let available = total - offset;
    let len = max_len.unwrap_or(available).min(available);
    Some(state::HttpSendfileView {
        body: Arc::clone(&asset.body),
        offset,
        len,
    })
}

#[cfg(feature = "network_http")]
pub fn http_handle_static_request(
    method: &str,
    path: &str,
    if_none_match: Option<u64>,
) -> state::HttpResponse {
    HTTP_REQUEST_CALLS.fetch_add(1, Ordering::Relaxed);
    if method != "GET" && method != "HEAD" {
        return state::HttpResponse {
            status: 405,
            headers: vec![("allow".to_string(), "GET, HEAD".to_string())],
            body: None,
        };
    }

    let assets = HTTP_STATIC_ASSETS.lock();
    let Some(asset) = assets.get(path) else {
        HTTP_RESP_404.fetch_add(1, Ordering::Relaxed);
        return state::HttpResponse {
            status: 404,
            headers: Vec::new(),
            body: None,
        };
    };

    if if_none_match == Some(asset.etag) {
        HTTP_RESP_304.fetch_add(1, Ordering::Relaxed);
        return state::HttpResponse {
            status: 304,
            headers: vec![
                ("etag".to_string(), format!("{}", asset.etag)),
                ("content-type".to_string(), asset.content_type.clone()),
                ("content-length".to_string(), "0".to_string()),
            ],
            body: None,
        };
    }

    HTTP_RESP_200.fetch_add(1, Ordering::Relaxed);
    HTTP_BYTES_SERVED.fetch_add(asset.body.len() as u64, Ordering::Relaxed);
    let body = if method == "HEAD" {
        None
    } else {
        Some(state::HttpSendfileView {
            body: Arc::clone(&asset.body),
            offset: 0,
            len: asset.body.len(),
        })
    };
    state::HttpResponse {
        status: 200,
        headers: vec![
            ("etag".to_string(), format!("{}", asset.etag)),
            ("content-type".to_string(), asset.content_type.clone()),
            (
                "content-length".to_string(),
                format!("{}", asset.body.len()),
            ),
        ],
        body,
    }
}

#[cfg(feature = "network_http")]
pub fn http_static_asset_count() -> usize {
    HTTP_STATIC_ASSETS.lock().len()
}
