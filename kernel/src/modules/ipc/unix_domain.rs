use alloc::collections::{BTreeMap, VecDeque};
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

use super::common::{current_task_id_or_kernel, IPC_UNIX_SOCKET_QUEUE_LIMIT};

static UNIX_BIND_CALLS: AtomicU64 = AtomicU64::new(0);
static UNIX_SEND_CALLS: AtomicU64 = AtomicU64::new(0);
static UNIX_RECV_CALLS: AtomicU64 = AtomicU64::new(0);
static UNIX_SEND_DROPS: AtomicU64 = AtomicU64::new(0);
static UNIX_RECV_HITS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
pub struct UnixSocketStats {
    pub bind_calls: u64,
    pub send_calls: u64,
    pub recv_calls: u64,
    pub send_drops: u64,
    pub recv_hits: u64,
    pub bound_sockets: usize,
}

use crate::interfaces::task::TaskId;

lazy_static! {
    static ref UNIX_SOCKETS: Mutex<BTreeMap<String, VecDeque<Vec<u8>>>> =
        Mutex::new(BTreeMap::new());
    static ref UNIX_OWNERS: Mutex<BTreeMap<String, TaskId>> = Mutex::new(BTreeMap::new());
}

pub fn unix_bind(path: &str) -> Result<(), &'static str> {
    UNIX_BIND_CALLS.fetch_add(1, Ordering::Relaxed);
    if path.is_empty() || !path.starts_with('/') {
        return Err("invalid unix socket path");
    }

    let tid = current_task_id_or_kernel();

    let mut owners = UNIX_OWNERS.lock();
    if owners.contains_key(path) {
        return Err("unix socket already bound");
    }
    owners.insert(path.into(), tid);

    let mut sockets = UNIX_SOCKETS.lock();
    sockets.entry(path.into()).or_insert_with(VecDeque::new);
    Ok(())
}

pub fn unix_send(path: &str, payload: &[u8]) -> Result<usize, &'static str> {
    UNIX_SEND_CALLS.fetch_add(1, Ordering::Relaxed);
    let mut sockets = UNIX_SOCKETS.lock();
    let queue = sockets.get_mut(path).ok_or("unix socket not bound")?;
    if queue.len() >= IPC_UNIX_SOCKET_QUEUE_LIMIT {
        UNIX_SEND_DROPS.fetch_add(1, Ordering::Relaxed);
        return Err("unix socket queue full");
    }
    queue.push_back(payload.to_vec());
    Ok(payload.len())
}

pub fn unix_recv(path: &str, out: &mut [u8]) -> Result<usize, &'static str> {
    UNIX_RECV_CALLS.fetch_add(1, Ordering::Relaxed);

    let tid = current_task_id_or_kernel();

    {
        let owners = UNIX_OWNERS.lock();
        let owner = owners.get(path).ok_or("unix socket not bound")?;
        if *owner != tid {
            return Err("permission denied: not socket owner");
        }
    }

    let mut sockets = UNIX_SOCKETS.lock();
    let queue = sockets.get_mut(path).ok_or("unix socket not bound")?;
    let frame = queue.pop_front().ok_or("unix socket empty")?;
    let copied = core::cmp::min(frame.len(), out.len());
    out[..copied].copy_from_slice(&frame[..copied]);
    UNIX_RECV_HITS.fetch_add(1, Ordering::Relaxed);
    Ok(copied)
}

pub fn unix_stats() -> UnixSocketStats {
    UnixSocketStats {
        bind_calls: UNIX_BIND_CALLS.load(Ordering::Relaxed),
        send_calls: UNIX_SEND_CALLS.load(Ordering::Relaxed),
        recv_calls: UNIX_RECV_CALLS.load(Ordering::Relaxed),
        send_drops: UNIX_SEND_DROPS.load(Ordering::Relaxed),
        recv_hits: UNIX_RECV_HITS.load(Ordering::Relaxed),
        bound_sockets: UNIX_SOCKETS.lock().len(),
    }
}
