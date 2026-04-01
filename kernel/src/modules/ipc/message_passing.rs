use crate::interfaces::task::TaskId;
use crate::interfaces::IpcChannel;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

const DEFAULT_CHANNEL_ID: TaskId = TaskId(0);
const MAX_CHANNEL_DEPTH: usize = 128;
const MAX_MESSAGE_SIZE: usize = 4096;

static IPC_MSG_CHANNEL_CREATE_CALLS: AtomicU64 = AtomicU64::new(0);
static IPC_MSG_SEND_CALLS: AtomicU64 = AtomicU64::new(0);
static IPC_MSG_SEND_DROPS_OVERSIZE: AtomicU64 = AtomicU64::new(0);
static IPC_MSG_SEND_DROPS_BACKPRESSURE: AtomicU64 = AtomicU64::new(0);
static IPC_MSG_RECV_CALLS: AtomicU64 = AtomicU64::new(0);
static IPC_MSG_RECV_HITS: AtomicU64 = AtomicU64::new(0);
static IPC_MSG_RECV_TRUNCATED: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
pub struct MessagePassingStats {
    pub channel_create_calls: u64,
    pub send_calls: u64,
    pub send_drops_oversize: u64,
    pub send_drops_backpressure: u64,
    pub receive_calls: u64,
    pub receive_hits: u64,
    pub receive_truncated: u64,
}

pub fn stats() -> MessagePassingStats {
    MessagePassingStats {
        channel_create_calls: IPC_MSG_CHANNEL_CREATE_CALLS.load(Ordering::Relaxed),
        send_calls: IPC_MSG_SEND_CALLS.load(Ordering::Relaxed),
        send_drops_oversize: IPC_MSG_SEND_DROPS_OVERSIZE.load(Ordering::Relaxed),
        send_drops_backpressure: IPC_MSG_SEND_DROPS_BACKPRESSURE.load(Ordering::Relaxed),
        receive_calls: IPC_MSG_RECV_CALLS.load(Ordering::Relaxed),
        receive_hits: IPC_MSG_RECV_HITS.load(Ordering::Relaxed),
        receive_truncated: IPC_MSG_RECV_TRUNCATED.load(Ordering::Relaxed),
    }
}

/// Message Passing IPC.
/// The standard "Post Office" model. Secure, copied, safe.
/// Global mailbox system: Tasks send messages to a specific Channel ID.

pub struct MessagePassing {
    channels: Mutex<BTreeMap<TaskId, VecDeque<Vec<u8>>>>,
}

impl MessagePassing {
    pub const fn new() -> Self {
        Self {
            channels: Mutex::new(BTreeMap::new()),
        }
    }

    // Create a new channel (e.g. at task creation)
    pub fn create_channel(&self, channel_id: TaskId) {
        IPC_MSG_CHANNEL_CREATE_CALLS.fetch_add(1, Ordering::Relaxed);
        self.channels
            .lock()
            .entry(channel_id)
            .or_insert_with(VecDeque::new);
    }

    pub fn send_to(&self, channel_id: TaskId, msg: &[u8]) {
        IPC_MSG_SEND_CALLS.fetch_add(1, Ordering::Relaxed);
        if msg.len() > MAX_MESSAGE_SIZE {
            IPC_MSG_SEND_DROPS_OVERSIZE.fetch_add(1, Ordering::Relaxed);
            return;
        }

        let mut locked = self.channels.lock();
        let queue = locked.entry(channel_id).or_insert_with(VecDeque::new);
        if queue.len() >= MAX_CHANNEL_DEPTH {
            IPC_MSG_SEND_DROPS_BACKPRESSURE.fetch_add(1, Ordering::Relaxed);
            return;
        }
        queue.push_back(msg.to_vec());
    }

    pub fn receive_from(&self, channel_id: TaskId, buffer: &mut [u8]) -> Option<usize> {
        IPC_MSG_RECV_CALLS.fetch_add(1, Ordering::Relaxed);
        let mut locked = self.channels.lock();
        let queue = locked.get_mut(&channel_id)?;
        let msg = queue.pop_front()?;
        let count = core::cmp::min(msg.len(), buffer.len());
        buffer[..count].copy_from_slice(&msg[..count]);
        IPC_MSG_RECV_HITS.fetch_add(1, Ordering::Relaxed);
        if count < msg.len() {
            IPC_MSG_RECV_TRUNCATED.fetch_add(1, Ordering::Relaxed);
        }
        Some(count)
    }
}

impl IpcChannel for MessagePassing {
    fn send(&self, msg: &[u8]) {
        self.send_to(DEFAULT_CHANNEL_ID, msg);
    }

    fn receive(&self, buffer: &mut [u8]) -> Option<usize> {
        self.receive_from(DEFAULT_CHANNEL_ID, buffer)
    }
}

// Extended functionality for real usage
impl MessagePassing {
    pub fn receive_owned(&self, channel_id: TaskId) -> Option<Vec<u8>> {
        let mut locked = self.channels.lock();
        locked.get_mut(&channel_id)?.pop_front()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn message_passing_truncation_is_accounted() {
        let mp = MessagePassing::new();
        mp.send(b"abcdef");
        let mut out = [0u8; 3];
        let got = mp.receive(&mut out);
        assert_eq!(got, Some(3));
        assert_eq!(&out, b"abc");
    }

    #[test_case]
    fn message_passing_oversize_is_dropped() {
        let mp = MessagePassing::new();
        let huge = [7u8; MAX_MESSAGE_SIZE + 1];
        mp.send(&huge);
        let mut out = [0u8; 8];
        assert_eq!(mp.receive(&mut out), None);
    }
}
