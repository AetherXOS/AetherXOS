pub mod bridge;
pub mod protocols;
pub mod epoll;
pub mod rdma_dpdk;
pub mod zero_copy_network;
mod filter_support;
#[cfg(feature = "network_http")]
#[path = "http_logic/http.rs"]
pub mod http;
#[cfg(feature = "network_http")]
#[path = "http_logic/http_client.rs"]
pub mod http_client;
#[cfg(feature = "network_http")]
#[path = "http_logic/http_support.rs"]
mod http_support;
/// Network Module
/// Defines traits for Network Interface Controllers (NICs) and Packet structures.
/// Real stack (TCP/IP) is provided via feature-gated smoltcp bridge for Core runtime use.

#[cfg(feature = "network_https")]
#[path = "http_logic/https.rs"]
pub mod https;
mod runtime;
mod state_counters;
pub mod sockopts;
mod state;
mod support;
pub mod metrics;
pub use self::metrics::ops as metrics_ops;
pub use self::metrics::control as metrics_control;
pub use self::metrics::facade as metrics_facade;
mod policy_ops;
#[path = "exports/public_exports.rs"]
pub mod public_exports;
mod runtime_facade;
#[path = "transport/transport_ops.rs"]
mod transport_ops;
#[cfg(feature = "network_transport")]
pub mod transport;
pub mod types;
mod wireguard_support;

pub use public_exports::*;

#[cfg(feature = "network_http")]
use alloc::vec;
use alloc::vec::Vec;
use core::sync::atomic::Ordering;
use state_counters::*;
use state::clear_runtime_state_tables;
use state::{DNS_TABLE, TCP_LISTENERS, TCP_PENDING_ACCEPT, TCP_STREAM_QUEUES, UDP_ENDPOINTS};
use support::{
    compute_network_alert_report, latency_percentiles, policy_from_u64, policy_to_u64,
    record_latency_bucket, reset_counter, reset_counters, reset_latency_buckets,
};
#[cfg(test)]
mod tests;
