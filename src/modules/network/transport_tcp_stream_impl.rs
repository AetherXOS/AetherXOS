use super::*;
use alloc::collections::VecDeque;
use super::filter_support::apply_filters;
use super::metrics_ops::{current_latency_tick, update_tcp_high_water};

#[cfg(feature = "network_transport")]
impl TcpStream {
    pub fn local_port(&self) -> u16 {
        self.local_port
    }

    pub fn peer_port(&self) -> u16 {
        self.peer_port
    }

    pub fn send(&self, payload: &[u8]) -> Result<usize, &'static str> {
        TCP_SEND_CALLS.fetch_add(1, Ordering::Relaxed);
        let start_tick = current_latency_tick();
        if let Err(err) = apply_filters(
            FilterProtocol::Tcp,
            Some(self.local_port),
            Some(self.peer_port),
            payload.len(),
        ) {
            record_latency_bucket(
                current_latency_tick().saturating_sub(start_tick),
                &TCP_SEND_LAT_BUCKET_0,
                &TCP_SEND_LAT_BUCKET_1,
                &TCP_SEND_LAT_BUCKET_2_3,
                &TCP_SEND_LAT_BUCKET_4_7,
                &TCP_SEND_LAT_BUCKET_GE8,
            );
            return Err(err);
        }
        let mut queues = TCP_STREAM_QUEUES.lock();
        let queue = queues.entry(self.peer_port).or_insert_with(VecDeque::new);
        if queue.len() >= crate::config::KernelConfig::network_tcp_queue_limit() {
            match policy_from_u64(TCP_BACKPRESSURE_POLICY.load(Ordering::Relaxed)) {
                BackpressurePolicy::Drop => {
                    BACKPRESSURE_DROP_ACTIONS.fetch_add(1, Ordering::Relaxed);
                    TCP_SEND_DROPS.fetch_add(1, Ordering::Relaxed);
                    record_latency_bucket(
                        current_latency_tick().saturating_sub(start_tick),
                        &TCP_SEND_LAT_BUCKET_0,
                        &TCP_SEND_LAT_BUCKET_1,
                        &TCP_SEND_LAT_BUCKET_2_3,
                        &TCP_SEND_LAT_BUCKET_4_7,
                        &TCP_SEND_LAT_BUCKET_GE8,
                    );
                    return Err("tcp stream queue full");
                }
                BackpressurePolicy::Defer => {
                    BACKPRESSURE_DEFER_ACTIONS.fetch_add(1, Ordering::Relaxed);
                    record_latency_bucket(
                        current_latency_tick().saturating_sub(start_tick),
                        &TCP_SEND_LAT_BUCKET_0,
                        &TCP_SEND_LAT_BUCKET_1,
                        &TCP_SEND_LAT_BUCKET_2_3,
                        &TCP_SEND_LAT_BUCKET_4_7,
                        &TCP_SEND_LAT_BUCKET_GE8,
                    );
                    return Err("tcp stream queue deferred");
                }
                BackpressurePolicy::ForcePoll => {
                    BACKPRESSURE_FORCE_POLL_ACTIONS.fetch_add(1, Ordering::Relaxed);
                    drop(queues);
                    let _ = force_poll_once();
                    let mut queues_retry = TCP_STREAM_QUEUES.lock();
                    let queue_retry = queues_retry
                        .entry(self.peer_port)
                        .or_insert_with(VecDeque::new);
                    if queue_retry.len() >= crate::config::KernelConfig::network_tcp_queue_limit() {
                        BACKPRESSURE_DROP_ACTIONS.fetch_add(1, Ordering::Relaxed);
                        TCP_SEND_DROPS.fetch_add(1, Ordering::Relaxed);
                        record_latency_bucket(
                            current_latency_tick().saturating_sub(start_tick),
                            &TCP_SEND_LAT_BUCKET_0,
                            &TCP_SEND_LAT_BUCKET_1,
                            &TCP_SEND_LAT_BUCKET_2_3,
                            &TCP_SEND_LAT_BUCKET_4_7,
                            &TCP_SEND_LAT_BUCKET_GE8,
                        );
                        return Err("tcp stream queue full after force poll");
                    }
                    queue_retry.push_back(payload.to_vec());
                    update_tcp_high_water(queue_retry.len());
                    record_latency_bucket(
                        current_latency_tick().saturating_sub(start_tick),
                        &TCP_SEND_LAT_BUCKET_0,
                        &TCP_SEND_LAT_BUCKET_1,
                        &TCP_SEND_LAT_BUCKET_2_3,
                        &TCP_SEND_LAT_BUCKET_4_7,
                        &TCP_SEND_LAT_BUCKET_GE8,
                    );
                    return Ok(payload.len());
                }
            }
        }
        queue.push_back(payload.to_vec());
        update_tcp_high_water(queue.len());
        record_latency_bucket(
            current_latency_tick().saturating_sub(start_tick),
            &TCP_SEND_LAT_BUCKET_0,
            &TCP_SEND_LAT_BUCKET_1,
            &TCP_SEND_LAT_BUCKET_2_3,
            &TCP_SEND_LAT_BUCKET_4_7,
            &TCP_SEND_LAT_BUCKET_GE8,
        );
        Ok(payload.len())
    }

    pub fn recv(&self) -> Option<Vec<u8>> {
        TCP_RECV_CALLS.fetch_add(1, Ordering::Relaxed);
        let start_tick = current_latency_tick();
        let mut queues = TCP_STREAM_QUEUES.lock();
        let Some(queue) = queues.get_mut(&self.local_port) else {
            record_latency_bucket(
                current_latency_tick().saturating_sub(start_tick),
                &TCP_RECV_LAT_BUCKET_0,
                &TCP_RECV_LAT_BUCKET_1,
                &TCP_RECV_LAT_BUCKET_2_3,
                &TCP_RECV_LAT_BUCKET_4_7,
                &TCP_RECV_LAT_BUCKET_GE8,
            );
            return None;
        };
        let chunk = queue.pop_front();
        if chunk.is_some() {
            TCP_RECV_HITS.fetch_add(1, Ordering::Relaxed);
        }
        record_latency_bucket(
            current_latency_tick().saturating_sub(start_tick),
            &TCP_RECV_LAT_BUCKET_0,
            &TCP_RECV_LAT_BUCKET_1,
            &TCP_RECV_LAT_BUCKET_2_3,
            &TCP_RECV_LAT_BUCKET_4_7,
            &TCP_RECV_LAT_BUCKET_GE8,
        );
        chunk
    }
}