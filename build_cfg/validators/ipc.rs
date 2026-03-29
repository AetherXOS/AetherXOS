//! IPC config validation — channels, message sizes, buffer limits.

use crate::build_cfg::config_types::IpcConfig;

const VALID_MECHANISMS: &[&str] = &[
    "ZeroCopy",
    "MessagePassing",
    "SignalOnly",
    "Pipes",
    "RingBuffer",
    "Futex",
];

pub fn validate(c: &IpcConfig) -> Vec<String> {
    let mut e = Vec::new();

    if !VALID_MECHANISMS.contains(&c.mechanism.as_str()) {
        e.push(format!(
            "ipc.mechanism '{}' invalid, expected one of {:?}",
            c.mechanism, VALID_MECHANISMS
        ));
    }
    if c.max_channels == 0 || c.max_channels > 65536 {
        e.push(format!(
            "ipc.max_channels {} out of range [1, 65536]",
            c.max_channels
        ));
    }
    if c.msg_size_limit == 0 || c.msg_size_limit > 1048576 {
        e.push(format!(
            "ipc.msg_size_limit {} out of range [1, 1048576]",
            c.msg_size_limit
        ));
    }
    if c.ring_buffer_size_kb == 0 || c.ring_buffer_size_kb > 16384 {
        e.push(format!(
            "ipc.ring_buffer_size_kb {} out of range [1, 16384]",
            c.ring_buffer_size_kb
        ));
    }
    if c.unix_socket_queue_limit == 0 || c.unix_socket_queue_limit > 65536 {
        e.push(format!(
            "ipc.unix_socket_queue_limit {} out of range [1, 65536]",
            c.unix_socket_queue_limit
        ));
    }
    if c.futex_wake_event_limit == 0 || c.futex_wake_event_limit > 65536 {
        e.push(format!(
            "ipc.futex_wake_event_limit {} out of range [1, 65536]",
            c.futex_wake_event_limit
        ));
    }

    e
}
