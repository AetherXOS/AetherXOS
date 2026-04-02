use crate::interfaces::task::TaskId;
use crate::interfaces::IpcChannel;
use crate::modules::ipc::common::bounded_push_bytes;
use crate::modules::ipc::common::IPC_PAGE_SIZE_BYTES;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use aethercore_common::{counter_inc, declare_counter_u64, telemetry};
use spin::Mutex;

const DEFAULT_CHANNEL_ID: TaskId = TaskId(0);
const MAX_CHANNEL_DEPTH: usize = 128;
const MAX_MESSAGE_SIZE: usize = IPC_PAGE_SIZE_BYTES;

declare_counter_u64!(IPC_MSG_CHANNEL_CREATE_CALLS);
declare_counter_u64!(IPC_MSG_SEND_CALLS);
declare_counter_u64!(IPC_MSG_SEND_DROPS_OVERSIZE);
declare_counter_u64!(IPC_MSG_SEND_DROPS_BACKPRESSURE);
declare_counter_u64!(IPC_MSG_RECV_CALLS);
declare_counter_u64!(IPC_MSG_RECV_HITS);
declare_counter_u64!(IPC_MSG_RECV_TRUNCATED);

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
        channel_create_calls: telemetry::snapshot_u64(&IPC_MSG_CHANNEL_CREATE_CALLS),
        send_calls: telemetry::snapshot_u64(&IPC_MSG_SEND_CALLS),
        send_drops_oversize: telemetry::snapshot_u64(&IPC_MSG_SEND_DROPS_OVERSIZE),
        send_drops_backpressure: telemetry::snapshot_u64(&IPC_MSG_SEND_DROPS_BACKPRESSURE),
        receive_calls: telemetry::snapshot_u64(&IPC_MSG_RECV_CALLS),
        receive_hits: telemetry::snapshot_u64(&IPC_MSG_RECV_HITS),
        receive_truncated: telemetry::snapshot_u64(&IPC_MSG_RECV_TRUNCATED),
    }
}

pub fn take_stats() -> MessagePassingStats {
    MessagePassingStats {
        channel_create_calls: telemetry::take_u64(&IPC_MSG_CHANNEL_CREATE_CALLS),
        send_calls: telemetry::take_u64(&IPC_MSG_SEND_CALLS),
        send_drops_oversize: telemetry::take_u64(&IPC_MSG_SEND_DROPS_OVERSIZE),
        send_drops_backpressure: telemetry::take_u64(&IPC_MSG_SEND_DROPS_BACKPRESSURE),
        receive_calls: telemetry::take_u64(&IPC_MSG_RECV_CALLS),
        receive_hits: telemetry::take_u64(&IPC_MSG_RECV_HITS),
        receive_truncated: telemetry::take_u64(&IPC_MSG_RECV_TRUNCATED),
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
        counter_inc!(IPC_MSG_CHANNEL_CREATE_CALLS);
        self.channels
            .lock()
            .entry(channel_id)
            .or_insert_with(VecDeque::new);
    }

    pub fn send_to(&self, channel_id: TaskId, msg: &[u8]) {
        counter_inc!(IPC_MSG_SEND_CALLS);
        if msg.len() > MAX_MESSAGE_SIZE {
            counter_inc!(IPC_MSG_SEND_DROPS_OVERSIZE);
            return;
        }

        let mut locked = self.channels.lock();
        let queue = locked.entry(channel_id).or_insert_with(VecDeque::new);
        if !bounded_push_bytes(queue, msg, MAX_CHANNEL_DEPTH) {
            counter_inc!(IPC_MSG_SEND_DROPS_BACKPRESSURE);
            return;
        }
    }

    pub fn receive_from(&self, channel_id: TaskId, buffer: &mut [u8]) -> Option<usize> {
        counter_inc!(IPC_MSG_RECV_CALLS);
        let mut locked = self.channels.lock();
        let queue = locked.get_mut(&channel_id)?;
        let msg = queue.pop_front()?;
        let count = core::cmp::min(msg.len(), buffer.len());
        buffer[..count].copy_from_slice(&msg[..count]);
        counter_inc!(IPC_MSG_RECV_HITS);
        if count < msg.len() {
            counter_inc!(IPC_MSG_RECV_TRUNCATED);
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
