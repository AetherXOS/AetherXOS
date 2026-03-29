use super::*;

pub(super) struct LatencySnapshot {
    pub udp_send_calls: u64,
    pub udp_send_p50: u64,
    pub udp_send_p95: u64,
    pub udp_send_p99: u64,
    pub udp_recv_calls: u64,
    pub udp_recv_p50: u64,
    pub udp_recv_p95: u64,
    pub udp_recv_p99: u64,
    pub tcp_send_calls: u64,
    pub tcp_send_p50: u64,
    pub tcp_send_p95: u64,
    pub tcp_send_p99: u64,
    pub tcp_recv_calls: u64,
    pub tcp_recv_p50: u64,
    pub tcp_recv_p95: u64,
    pub tcp_recv_p99: u64,
}

pub(super) fn collect_latency_snapshot() -> LatencySnapshot {
    let udp_send_calls = UDP_SEND_CALLS.load(Ordering::Relaxed);
    let udp_send_b0 = UDP_SEND_LAT_BUCKET_0.load(Ordering::Relaxed);
    let udp_send_b1 = UDP_SEND_LAT_BUCKET_1.load(Ordering::Relaxed);
    let udp_send_b2_3 = UDP_SEND_LAT_BUCKET_2_3.load(Ordering::Relaxed);
    let udp_send_b4_7 = UDP_SEND_LAT_BUCKET_4_7.load(Ordering::Relaxed);
    let udp_send_bge8 = UDP_SEND_LAT_BUCKET_GE8.load(Ordering::Relaxed);
    let (udp_send_p50, udp_send_p95, udp_send_p99) = latency_percentiles(
        udp_send_calls,
        udp_send_b0,
        udp_send_b1,
        udp_send_b2_3,
        udp_send_b4_7,
        udp_send_bge8,
    );

    let udp_recv_calls = UDP_RECV_CALLS.load(Ordering::Relaxed);
    let udp_recv_b0 = UDP_RECV_LAT_BUCKET_0.load(Ordering::Relaxed);
    let udp_recv_b1 = UDP_RECV_LAT_BUCKET_1.load(Ordering::Relaxed);
    let udp_recv_b2_3 = UDP_RECV_LAT_BUCKET_2_3.load(Ordering::Relaxed);
    let udp_recv_b4_7 = UDP_RECV_LAT_BUCKET_4_7.load(Ordering::Relaxed);
    let udp_recv_bge8 = UDP_RECV_LAT_BUCKET_GE8.load(Ordering::Relaxed);
    let (udp_recv_p50, udp_recv_p95, udp_recv_p99) = latency_percentiles(
        udp_recv_calls,
        udp_recv_b0,
        udp_recv_b1,
        udp_recv_b2_3,
        udp_recv_b4_7,
        udp_recv_bge8,
    );

    let tcp_send_calls = TCP_SEND_CALLS.load(Ordering::Relaxed);
    let tcp_send_b0 = TCP_SEND_LAT_BUCKET_0.load(Ordering::Relaxed);
    let tcp_send_b1 = TCP_SEND_LAT_BUCKET_1.load(Ordering::Relaxed);
    let tcp_send_b2_3 = TCP_SEND_LAT_BUCKET_2_3.load(Ordering::Relaxed);
    let tcp_send_b4_7 = TCP_SEND_LAT_BUCKET_4_7.load(Ordering::Relaxed);
    let tcp_send_bge8 = TCP_SEND_LAT_BUCKET_GE8.load(Ordering::Relaxed);
    let (tcp_send_p50, tcp_send_p95, tcp_send_p99) = latency_percentiles(
        tcp_send_calls,
        tcp_send_b0,
        tcp_send_b1,
        tcp_send_b2_3,
        tcp_send_b4_7,
        tcp_send_bge8,
    );

    let tcp_recv_calls = TCP_RECV_CALLS.load(Ordering::Relaxed);
    let tcp_recv_b0 = TCP_RECV_LAT_BUCKET_0.load(Ordering::Relaxed);
    let tcp_recv_b1 = TCP_RECV_LAT_BUCKET_1.load(Ordering::Relaxed);
    let tcp_recv_b2_3 = TCP_RECV_LAT_BUCKET_2_3.load(Ordering::Relaxed);
    let tcp_recv_b4_7 = TCP_RECV_LAT_BUCKET_4_7.load(Ordering::Relaxed);
    let tcp_recv_bge8 = TCP_RECV_LAT_BUCKET_GE8.load(Ordering::Relaxed);
    let (tcp_recv_p50, tcp_recv_p95, tcp_recv_p99) = latency_percentiles(
        tcp_recv_calls,
        tcp_recv_b0,
        tcp_recv_b1,
        tcp_recv_b2_3,
        tcp_recv_b4_7,
        tcp_recv_bge8,
    );

    LatencySnapshot {
        udp_send_calls,
        udp_send_p50,
        udp_send_p95,
        udp_send_p99,
        udp_recv_calls,
        udp_recv_p50,
        udp_recv_p95,
        udp_recv_p99,
        tcp_send_calls,
        tcp_send_p50,
        tcp_send_p95,
        tcp_send_p99,
        tcp_recv_calls,
        tcp_recv_p50,
        tcp_recv_p95,
        tcp_recv_p99,
    }
}