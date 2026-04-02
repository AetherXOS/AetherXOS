use alloc::collections::{BTreeMap, VecDeque};
use alloc::string::String;
use alloc::vec::Vec;
use aethercore_common::{counter_inc, declare_counter_u64, telemetry};
use core::sync::atomic::Ordering;
use lazy_static::lazy_static;
use spin::Mutex;

use super::common::{current_task_id_or_kernel, IPC_UNIX_SOCKET_QUEUE_LIMIT};
use super::common::bounded_push_bytes;

declare_counter_u64!(UNIX_BIND_CALLS);
declare_counter_u64!(UNIX_SEND_CALLS);
declare_counter_u64!(UNIX_RECV_CALLS);
declare_counter_u64!(UNIX_SEND_DROPS);
declare_counter_u64!(UNIX_RECV_HITS);

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
    counter_inc!(UNIX_BIND_CALLS);
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
    counter_inc!(UNIX_SEND_CALLS);
    let mut sockets = UNIX_SOCKETS.lock();
    let queue = sockets.get_mut(path).ok_or("unix socket not bound")?;
    if !bounded_push_bytes(queue, payload, IPC_UNIX_SOCKET_QUEUE_LIMIT) {
        counter_inc!(UNIX_SEND_DROPS);
        return Err("unix socket queue full");
    }
    Ok(payload.len())
}

pub fn unix_recv(path: &str, out: &mut [u8]) -> Result<usize, &'static str> {
    counter_inc!(UNIX_RECV_CALLS);

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
    counter_inc!(UNIX_RECV_HITS);
    Ok(copied)
}

pub fn unix_stats() -> UnixSocketStats {
    UnixSocketStats {
        bind_calls: telemetry::snapshot_u64(&UNIX_BIND_CALLS),
        send_calls: telemetry::snapshot_u64(&UNIX_SEND_CALLS),
        recv_calls: telemetry::snapshot_u64(&UNIX_RECV_CALLS),
        send_drops: telemetry::snapshot_u64(&UNIX_SEND_DROPS),
        recv_hits: telemetry::snapshot_u64(&UNIX_RECV_HITS),
        bound_sockets: UNIX_SOCKETS.lock().len(),
    }
}

pub fn unix_take_stats() -> UnixSocketStats {
    UnixSocketStats {
        bind_calls: telemetry::take_u64(&UNIX_BIND_CALLS),
        send_calls: telemetry::take_u64(&UNIX_SEND_CALLS),
        recv_calls: telemetry::take_u64(&UNIX_RECV_CALLS),
        send_drops: telemetry::take_u64(&UNIX_SEND_DROPS),
        recv_hits: telemetry::take_u64(&UNIX_RECV_HITS),
        bound_sockets: UNIX_SOCKETS.lock().len(),
    }
}
