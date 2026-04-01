use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

const RECONNECT_BASE_BACKOFF_TICKS: u64 = 1;

static NFS_MOUNT_CALLS: AtomicU64 = AtomicU64::new(0);
static NFS_READ_CALLS: AtomicU64 = AtomicU64::new(0);
static NFS_RECONNECT_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static NFS_RECONNECT_SUCCESS: AtomicU64 = AtomicU64::new(0);
static NFS_RECONNECT_FAILURES: AtomicU64 = AtomicU64::new(0);
static NFS_FORCED_DISCONNECTS: AtomicU64 = AtomicU64::new(0);
static P9_ATTACH_CALLS: AtomicU64 = AtomicU64::new(0);
static P9_READ_CALLS: AtomicU64 = AtomicU64::new(0);
static P9_RECONNECT_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static P9_RECONNECT_SUCCESS: AtomicU64 = AtomicU64::new(0);
static P9_RECONNECT_FAILURES: AtomicU64 = AtomicU64::new(0);
static P9_FORCED_DISCONNECTS: AtomicU64 = AtomicU64::new(0);

lazy_static! {
    static ref NFS_FILES: Mutex<BTreeMap<String, Vec<u8>>> = Mutex::new(BTreeMap::new());
    static ref P9_FILES: Mutex<BTreeMap<String, Vec<u8>>> = Mutex::new(BTreeMap::new());
    static ref NFS_SESSION: Mutex<ReconnectSession> = Mutex::new(ReconnectSession::default());
    static ref P9_SESSION: Mutex<ReconnectSession> = Mutex::new(ReconnectSession::default());
}

#[derive(Debug, Clone)]
struct ReconnectSession {
    connected: bool,
    endpoint: String,
    target: String,
    reconnect_backoff_ticks: u64,
    last_reconnect_attempt_tick: u64,
}

impl Default for ReconnectSession {
    fn default() -> Self {
        Self {
            connected: false,
            endpoint: String::new(),
            target: String::new(),
            reconnect_backoff_ticks: RECONNECT_BASE_BACKOFF_TICKS,
            last_reconnect_attempt_tick: 0,
        }
    }
}

impl ReconnectSession {
    fn connect(&mut self, endpoint: &str, target: &str) {
        self.connected = true;
        self.endpoint = endpoint.into();
        self.target = target.into();
        self.reconnect_backoff_ticks = RECONNECT_BASE_BACKOFF_TICKS;
        self.last_reconnect_attempt_tick = 0;
    }

    fn disconnect(&mut self) {
        self.connected = false;
    }

    fn can_attempt_reconnect(&self, now_tick: u64) -> bool {
        if self.last_reconnect_attempt_tick == 0 {
            return true;
        }
        now_tick.saturating_sub(self.last_reconnect_attempt_tick) >= self.reconnect_backoff_ticks
    }

    fn note_successful_reconnect(&mut self, now_tick: u64) {
        self.connected = true;
        self.last_reconnect_attempt_tick = now_tick;
        self.reconnect_backoff_ticks = RECONNECT_BASE_BACKOFF_TICKS;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NetworkFsStats {
    pub nfs_mount_calls: u64,
    pub nfs_read_calls: u64,
    pub nfs_reconnect_attempts: u64,
    pub nfs_reconnect_success: u64,
    pub nfs_reconnect_failures: u64,
    pub nfs_forced_disconnects: u64,
    pub nfs_connected: bool,
    pub nfs_reconnect_backoff_ticks: u64,
    pub p9_attach_calls: u64,
    pub p9_read_calls: u64,
    pub p9_reconnect_attempts: u64,
    pub p9_reconnect_success: u64,
    pub p9_reconnect_failures: u64,
    pub p9_forced_disconnects: u64,
    pub p9_connected: bool,
    pub p9_reconnect_backoff_ticks: u64,
}

fn ensure_nfs_reconnected() -> Result<(), &'static str> {
    let mut session = NFS_SESSION.lock();
    if session.connected {
        return Ok(());
    }
    if session.endpoint.is_empty() || session.target.is_empty() {
        NFS_RECONNECT_FAILURES.fetch_add(1, Ordering::Relaxed);
        return Err("nfs session not configured");
    }

    let now_tick = crate::kernel::watchdog::global_tick();
    if !session.can_attempt_reconnect(now_tick) {
        return Err("nfs reconnect backoff active");
    }

    NFS_RECONNECT_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
    // Baseline reconnect policy: once session parameters are present, reconnect succeeds.
    session.note_successful_reconnect(now_tick);
    NFS_RECONNECT_SUCCESS.fetch_add(1, Ordering::Relaxed);
    Ok(())
}

fn ensure_p9_reconnected() -> Result<(), &'static str> {
    let mut session = P9_SESSION.lock();
    if session.connected {
        return Ok(());
    }
    if session.target.is_empty() {
        P9_RECONNECT_FAILURES.fetch_add(1, Ordering::Relaxed);
        return Err("9p session not configured");
    }

    let now_tick = crate::kernel::watchdog::global_tick();
    if !session.can_attempt_reconnect(now_tick) {
        return Err("9p reconnect backoff active");
    }

    P9_RECONNECT_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
    // Baseline reconnect policy: once attach tag is present, reconnect succeeds.
    session.note_successful_reconnect(now_tick);
    P9_RECONNECT_SUCCESS.fetch_add(1, Ordering::Relaxed);
    Ok(())
}

pub fn nfs_mount(endpoint: &str, export: &str) -> Result<(), &'static str> {
    if endpoint.is_empty() || export.is_empty() {
        return Err("invalid nfs endpoint/export");
    }
    NFS_MOUNT_CALLS.fetch_add(1, Ordering::Relaxed);

    {
        let mut session = NFS_SESSION.lock();
        session.connect(endpoint, export);
    }

    let mut files = NFS_FILES.lock();
    files.insert("/nfs/hello.txt".into(), b"hello-from-nfs".to_vec());
    Ok(())
}

pub fn nfs_read(path: &str, out: &mut [u8]) -> Result<usize, &'static str> {
    NFS_READ_CALLS.fetch_add(1, Ordering::Relaxed);
    ensure_nfs_reconnected()?;

    let files = NFS_FILES.lock();
    let content = files.get(path).ok_or("nfs file not found")?;
    let copied = core::cmp::min(out.len(), content.len());
    out[..copied].copy_from_slice(&content[..copied]);
    Ok(copied)
}

pub fn p9_attach(tag: &str) -> Result<(), &'static str> {
    if tag.is_empty() {
        return Err("invalid 9p attach tag");
    }
    P9_ATTACH_CALLS.fetch_add(1, Ordering::Relaxed);

    {
        let mut session = P9_SESSION.lock();
        session.connect("9p", tag);
    }

    let mut files = P9_FILES.lock();
    files.insert("/9p/host.txt".into(), b"hello-from-9p".to_vec());
    Ok(())
}

pub fn p9_read(path: &str, out: &mut [u8]) -> Result<usize, &'static str> {
    P9_READ_CALLS.fetch_add(1, Ordering::Relaxed);
    ensure_p9_reconnected()?;

    let files = P9_FILES.lock();
    let content = files.get(path).ok_or("9p file not found")?;
    let copied = core::cmp::min(out.len(), content.len());
    out[..copied].copy_from_slice(&content[..copied]);
    Ok(copied)
}

pub fn force_nfs_disconnect() {
    NFS_FORCED_DISCONNECTS.fetch_add(1, Ordering::Relaxed);
    NFS_SESSION.lock().disconnect();
}

pub fn force_p9_disconnect() {
    P9_FORCED_DISCONNECTS.fetch_add(1, Ordering::Relaxed);
    P9_SESSION.lock().disconnect();
}

pub fn network_fs_stats() -> NetworkFsStats {
    let nfs = NFS_SESSION.lock();
    let p9 = P9_SESSION.lock();
    NetworkFsStats {
        nfs_mount_calls: NFS_MOUNT_CALLS.load(Ordering::Relaxed),
        nfs_read_calls: NFS_READ_CALLS.load(Ordering::Relaxed),
        nfs_reconnect_attempts: NFS_RECONNECT_ATTEMPTS.load(Ordering::Relaxed),
        nfs_reconnect_success: NFS_RECONNECT_SUCCESS.load(Ordering::Relaxed),
        nfs_reconnect_failures: NFS_RECONNECT_FAILURES.load(Ordering::Relaxed),
        nfs_forced_disconnects: NFS_FORCED_DISCONNECTS.load(Ordering::Relaxed),
        nfs_connected: nfs.connected,
        nfs_reconnect_backoff_ticks: nfs.reconnect_backoff_ticks,
        p9_attach_calls: P9_ATTACH_CALLS.load(Ordering::Relaxed),
        p9_read_calls: P9_READ_CALLS.load(Ordering::Relaxed),
        p9_reconnect_attempts: P9_RECONNECT_ATTEMPTS.load(Ordering::Relaxed),
        p9_reconnect_success: P9_RECONNECT_SUCCESS.load(Ordering::Relaxed),
        p9_reconnect_failures: P9_RECONNECT_FAILURES.load(Ordering::Relaxed),
        p9_forced_disconnects: P9_FORCED_DISCONNECTS.load(Ordering::Relaxed),
        p9_connected: p9.connected,
        p9_reconnect_backoff_ticks: p9.reconnect_backoff_ticks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn nfs_reconnects_after_forced_disconnect() {
        nfs_mount("10.0.2.2:2049", "/exports/root").expect("nfs mount");
        force_nfs_disconnect();
        let mut out = [0u8; 32];
        let n = nfs_read("/nfs/hello.txt", &mut out).expect("nfs read after reconnect");
        assert_eq!(&out[..n], b"hello-from-nfs");

        let stats = network_fs_stats();
        assert!(stats.nfs_reconnect_attempts >= 1);
        assert!(stats.nfs_reconnect_success >= 1);
        assert!(stats.nfs_connected);
    }

    #[test_case]
    fn p9_reconnects_after_forced_disconnect() {
        p9_attach("hostshare").expect("9p attach");
        force_p9_disconnect();
        let mut out = [0u8; 32];
        let n = p9_read("/9p/host.txt", &mut out).expect("9p read after reconnect");
        assert_eq!(&out[..n], b"hello-from-9p");

        let stats = network_fs_stats();
        assert!(stats.p9_reconnect_attempts >= 1);
        assert!(stats.p9_reconnect_success >= 1);
        assert!(stats.p9_connected);
    }
}
