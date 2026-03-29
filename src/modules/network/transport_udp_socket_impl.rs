use super::*;
use alloc::collections::VecDeque;
use super::filter_support::apply_filters;
use super::metrics_ops::{current_latency_tick, update_udp_high_water};

#[cfg(feature = "network_transport")]
impl UdpSocket {
    pub fn local_port(&self) -> u16 {
        self.local_port
    }

    pub fn send_to(&self, dst_port: u16, payload: &[u8]) -> Result<usize, &'static str> {
        if dst_port == 0 {
            return Err("invalid udp destination");
        }
        UDP_SEND_CALLS.fetch_add(1, Ordering::Relaxed);
        let start_tick = current_latency_tick();
        if let Err(err) = apply_filters(
            FilterProtocol::Udp,
            Some(self.local_port),
            Some(dst_port),
            payload.len(),
        ) {
            record_latency_bucket(
                current_latency_tick().saturating_sub(start_tick),
                &UDP_SEND_LAT_BUCKET_0,
                &UDP_SEND_LAT_BUCKET_1,
                &UDP_SEND_LAT_BUCKET_2_3,
                &UDP_SEND_LAT_BUCKET_4_7,
                &UDP_SEND_LAT_BUCKET_GE8,
            );
            return Err(err);
        }
        let mut endpoints = UDP_ENDPOINTS.lock();
        let queue = endpoints.entry(dst_port).or_insert_with(VecDeque::new);
        if queue.len() >= crate::config::KernelConfig::network_udp_queue_limit() {
            match policy_from_u64(UDP_BACKPRESSURE_POLICY.load(Ordering::Relaxed)) {
                BackpressurePolicy::Drop => {
                    BACKPRESSURE_DROP_ACTIONS.fetch_add(1, Ordering::Relaxed);
                    UDP_SEND_DROPS.fetch_add(1, Ordering::Relaxed);
                    record_latency_bucket(
                        current_latency_tick().saturating_sub(start_tick),
                        &UDP_SEND_LAT_BUCKET_0,
                        &UDP_SEND_LAT_BUCKET_1,
                        &UDP_SEND_LAT_BUCKET_2_3,
                        &UDP_SEND_LAT_BUCKET_4_7,
                        &UDP_SEND_LAT_BUCKET_GE8,
                    );
                    return Err("udp queue full");
                }
                BackpressurePolicy::Defer => {
                    BACKPRESSURE_DEFER_ACTIONS.fetch_add(1, Ordering::Relaxed);
                    record_latency_bucket(
                        current_latency_tick().saturating_sub(start_tick),
                        &UDP_SEND_LAT_BUCKET_0,
                        &UDP_SEND_LAT_BUCKET_1,
                        &UDP_SEND_LAT_BUCKET_2_3,
                        &UDP_SEND_LAT_BUCKET_4_7,
                        &UDP_SEND_LAT_BUCKET_GE8,
                    );
                    return Err("udp queue deferred");
                }
                BackpressurePolicy::ForcePoll => {
                    BACKPRESSURE_FORCE_POLL_ACTIONS.fetch_add(1, Ordering::Relaxed);
                    drop(endpoints);
                    let _ = force_poll_once();
                    let mut endpoints_retry = UDP_ENDPOINTS.lock();
                    let queue_retry = endpoints_retry
                        .entry(dst_port)
                        .or_insert_with(VecDeque::new);
                    if queue_retry.len() >= crate::config::KernelConfig::network_udp_queue_limit() {
                        BACKPRESSURE_DROP_ACTIONS.fetch_add(1, Ordering::Relaxed);
                        UDP_SEND_DROPS.fetch_add(1, Ordering::Relaxed);
                        record_latency_bucket(
                            current_latency_tick().saturating_sub(start_tick),
                            &UDP_SEND_LAT_BUCKET_0,
                            &UDP_SEND_LAT_BUCKET_1,
                            &UDP_SEND_LAT_BUCKET_2_3,
                            &UDP_SEND_LAT_BUCKET_4_7,
                            &UDP_SEND_LAT_BUCKET_GE8,
                        );
                        return Err("udp queue full after force poll");
                    }
                    queue_retry.push_back(UdpDatagram {
                        src_port: self.local_port,
                        dst_port,
                        payload: payload.to_vec(),
                    });
                    update_udp_high_water(queue_retry.len());
                    record_latency_bucket(
                        current_latency_tick().saturating_sub(start_tick),
                        &UDP_SEND_LAT_BUCKET_0,
                        &UDP_SEND_LAT_BUCKET_1,
                        &UDP_SEND_LAT_BUCKET_2_3,
                        &UDP_SEND_LAT_BUCKET_4_7,
                        &UDP_SEND_LAT_BUCKET_GE8,
                    );
                    return Ok(payload.len());
                }
            }
        }
        queue.push_back(UdpDatagram {
            src_port: self.local_port,
            dst_port,
            payload: payload.to_vec(),
        });
        update_udp_high_water(queue.len());
        record_latency_bucket(
            current_latency_tick().saturating_sub(start_tick),
            &UDP_SEND_LAT_BUCKET_0,
            &UDP_SEND_LAT_BUCKET_1,
            &UDP_SEND_LAT_BUCKET_2_3,
            &UDP_SEND_LAT_BUCKET_4_7,
            &UDP_SEND_LAT_BUCKET_GE8,
        );
        Ok(payload.len())
    }

    pub fn recv(&self) -> Option<UdpDatagram> {
        UDP_RECV_CALLS.fetch_add(1, Ordering::Relaxed);
        let start_tick = current_latency_tick();
        let mut endpoints = UDP_ENDPOINTS.lock();
        let Some(queue) = endpoints.get_mut(&self.local_port) else {
            record_latency_bucket(
                current_latency_tick().saturating_sub(start_tick),
                &UDP_RECV_LAT_BUCKET_0,
                &UDP_RECV_LAT_BUCKET_1,
                &UDP_RECV_LAT_BUCKET_2_3,
                &UDP_RECV_LAT_BUCKET_4_7,
                &UDP_RECV_LAT_BUCKET_GE8,
            );
            return None;
        };
        let packet = queue.pop_front();
        if packet.is_some() {
            UDP_RECV_HITS.fetch_add(1, Ordering::Relaxed);
        }
        record_latency_bucket(
            current_latency_tick().saturating_sub(start_tick),
            &UDP_RECV_LAT_BUCKET_0,
            &UDP_RECV_LAT_BUCKET_1,
            &UDP_RECV_LAT_BUCKET_2_3,
            &UDP_RECV_LAT_BUCKET_4_7,
            &UDP_RECV_LAT_BUCKET_GE8,
        );
        packet
    }
}