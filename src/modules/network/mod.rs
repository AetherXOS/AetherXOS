pub mod bridge;
pub mod epoll;
mod filter_support;
#[cfg(feature = "network_http")]
pub mod http;
#[cfg(feature = "network_http")]
pub mod http_client;
mod http_support;
/// Network Module
/// Defines traits for Network Interface Controllers (NICs) and Packet structures.
/// Real stack (TCP/IP) is provided via feature-gated smoltcp bridge for Core runtime use.

#[cfg(feature = "network_https")]
pub mod https;
mod runtime;
mod state_counters;
pub mod sockopts;
mod state;
mod support;
mod metrics_ops;
mod metrics_control;
mod metrics_facade;
mod policy_ops;
mod public_exports;
mod runtime_facade;
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
#[cfg(feature = "network_http")]
use state::HTTP_STATIC_ASSETS;
#[cfg(feature = "network_wireguard")]
use state::WG_PEERS;
use state::{DNS_TABLE, TCP_LISTENERS, TCP_PENDING_ACCEPT, TCP_STREAM_QUEUES, UDP_ENDPOINTS};
use support::{
    compute_network_alert_report, latency_percentiles, policy_from_u64, policy_to_u64,
    record_latency_bucket, reset_counter, reset_counters, reset_latency_buckets, update_high_water,
};
#[cfg(test)]
mod tests;
