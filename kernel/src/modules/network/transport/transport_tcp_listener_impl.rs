use super::*;
use alloc::collections::VecDeque;
use super::metrics_ops::current_latency_tick;

#[cfg(feature = "network_transport")]
impl TcpListener {
    pub fn local_port(&self) -> u16 {
        self.local_port
    }

    pub fn accept(&self) -> Option<TcpStream> {
        TCP_ACCEPT_CALLS.fetch_add(1, Ordering::Relaxed);
        let start_tick = current_latency_tick();
        let mut pending = TCP_PENDING_ACCEPT.lock();
        let queue = pending.entry(self.local_port).or_insert_with(VecDeque::new);
        let stream = queue.pop_front().map(|peer_port| TcpStream {
            local_port: self.local_port,
            peer_port,
        });
        if stream.is_some() {
            TCP_ACCEPT_HITS.fetch_add(1, Ordering::Relaxed);
        }
        record_latency_bucket(
            current_latency_tick().saturating_sub(start_tick),
            &TCP_RECV_LAT_BUCKET_0,
            &TCP_RECV_LAT_BUCKET_1,
            &TCP_RECV_LAT_BUCKET_2_3,
            &TCP_RECV_LAT_BUCKET_4_7,
            &TCP_RECV_LAT_BUCKET_GE8,
        );
        stream
    }
}