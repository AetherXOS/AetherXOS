use alloc::sync::Arc;
use core::sync::atomic::{AtomicU64, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

static HTTPS_INSTALL_CALLS: AtomicU64 = AtomicU64::new(0);
static HTTPS_TERMINATE_CALLS: AtomicU64 = AtomicU64::new(0);
static HTTPS_TERMINATE_ERRORS: AtomicU64 = AtomicU64::new(0);
static HTTPS_POLICY_REJECTS: AtomicU64 = AtomicU64::new(0);

lazy_static! {
    static ref HTTPS_SERVER_CONFIG: Mutex<Option<Arc<rustls::ServerConfig>>> = Mutex::new(None);
}

#[derive(Debug, Clone, Copy)]
pub struct HttpsStats {
    pub install_calls: u64,
    pub terminate_calls: u64,
    pub terminate_errors: u64,
    pub policy_rejects: u64,
    pub config_installed: bool,
    pub tls_policy_profile: &'static str,
}

pub fn https_install_server_config(config: Arc<rustls::ServerConfig>) {
    HTTPS_INSTALL_CALLS.fetch_add(1, Ordering::Relaxed);
    *HTTPS_SERVER_CONFIG.lock() = Some(config);
}

pub fn https_terminate_tls_record(record: &[u8], out: &mut [u8]) -> Result<usize, &'static str> {
    HTTPS_TERMINATE_CALLS.fetch_add(1, Ordering::Relaxed);

    if HTTPS_SERVER_CONFIG.lock().is_none() {
        HTTPS_TERMINATE_ERRORS.fetch_add(1, Ordering::Relaxed);
        return Err("https config missing");
    }

    if record.is_empty() {
        HTTPS_TERMINATE_ERRORS.fetch_add(1, Ordering::Relaxed);
        return Err("empty tls record");
    }

    if crate::config::KernelConfig::network_tls_policy_profile()
        == crate::config::TlsPolicyProfile::Strict
        && out.len() < record.len()
    {
        HTTPS_TERMINATE_ERRORS.fetch_add(1, Ordering::Relaxed);
        HTTPS_POLICY_REJECTS.fetch_add(1, Ordering::Relaxed);
        return Err("strict tls policy requires full record output capacity");
    }

    let copied = core::cmp::min(out.len(), record.len());
    out[..copied].copy_from_slice(&record[..copied]);
    Ok(copied)
}

pub fn https_stats() -> HttpsStats {
    HttpsStats {
        install_calls: HTTPS_INSTALL_CALLS.load(Ordering::Relaxed),
        terminate_calls: HTTPS_TERMINATE_CALLS.load(Ordering::Relaxed),
        terminate_errors: HTTPS_TERMINATE_ERRORS.load(Ordering::Relaxed),
        policy_rejects: HTTPS_POLICY_REJECTS.load(Ordering::Relaxed),
        config_installed: HTTPS_SERVER_CONFIG.lock().is_some(),
        tls_policy_profile: crate::config::KernelConfig::network_tls_policy_profile_name(),
    }
}
