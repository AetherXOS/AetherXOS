use super::*;

#[derive(Debug, Clone, Copy)]
pub struct NetworkDataplaneStats {
    pub active_driver: ActiveNetworkDriver,
    pub poll_profile: NetworkPollProfile,
    pub driver_io_owned: bool,
    pub register_virtio_calls: u64,
    pub register_e1000_calls: u64,
    pub service_calls: u64,
    pub irq_service_calls: u64,
    pub tx_to_nic_frames: u64,
    pub tx_to_nic_drops: u64,
    pub rx_to_core_frames: u64,
    pub rx_to_core_drops: u64,
    pub virtio_rx_depth: usize,
    pub virtio_tx_depth: usize,
    pub e1000_rx_depth: usize,
    pub e1000_tx_depth: usize,
    pub e1000_io_calls: u64,
    pub e1000_rx_frames: u64,
    pub e1000_rx_bytes: u64,
    pub e1000_rx_invalid_len: u64,
    pub e1000_rx_delivery_drops: u64,
    pub e1000_tx_frames: u64,
    pub e1000_tx_bytes: u64,
    pub e1000_tx_truncated_frames: u64,
    pub e1000_tx_desc_busy_events: u64,
    pub e1000_tx_lock_contention_events: u64,
    pub e1000_io_errors: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct NetworkDriverSloReport {
    pub driver: ActiveNetworkDriver,
    pub drop_rate_per_mille: u64,
    pub tx_ring_utilization_percent: u64,
    pub rx_ring_utilization_percent: u64,
    pub driver_io_errors: u64,
    pub drop_rate_breach: bool,
    pub tx_ring_breach: bool,
    pub rx_ring_breach: bool,
    pub io_error_breach: bool,
    pub breach_count: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct NetworkQueueResetSummary {
    pub driver: ActiveNetworkDriver,
    pub cleared_virtio_rx: usize,
    pub cleared_virtio_tx: usize,
    pub cleared_e1000_rx: usize,
    pub cleared_e1000_tx: usize,
}

pub fn stats() -> NetworkDataplaneStats {
    let e1000 = crate::modules::drivers::e1000::dataplane_stats();
    NetworkDataplaneStats {
        active_driver: active_driver(),
        poll_profile: poll_profile(),
        driver_io_owned: driver_io_owned(),
        register_virtio_calls: REGISTER_VIRTIO_CALLS.load(Ordering::Relaxed),
        register_e1000_calls: REGISTER_E1000_CALLS.load(Ordering::Relaxed),
        service_calls: SERVICE_CALLS.load(Ordering::Relaxed),
        irq_service_calls: IRQ_SERVICE_CALLS.load(Ordering::Relaxed),
        tx_to_nic_frames: TX_TO_NIC_FRAMES.load(Ordering::Relaxed),
        tx_to_nic_drops: TX_TO_NIC_DROPS.load(Ordering::Relaxed),
        rx_to_core_frames: RX_TO_CORE_FRAMES.load(Ordering::Relaxed),
        rx_to_core_drops: RX_TO_CORE_DROPS.load(Ordering::Relaxed),
        virtio_rx_depth: VIRTIO_RX_RING.lock().len(),
        virtio_tx_depth: VIRTIO_TX_RING.lock().len(),
        e1000_rx_depth: E1000_RX_RING.lock().len(),
        e1000_tx_depth: E1000_TX_RING.lock().len(),
        e1000_io_calls: e1000.io_calls,
        e1000_rx_frames: e1000.rx_frames,
        e1000_rx_bytes: e1000.rx_bytes,
        e1000_rx_invalid_len: e1000.rx_invalid_len,
        e1000_rx_delivery_drops: e1000.rx_delivery_drops,
        e1000_tx_frames: e1000.tx_frames,
        e1000_tx_bytes: e1000.tx_bytes,
        e1000_tx_truncated_frames: e1000.tx_truncated_frames,
        e1000_tx_desc_busy_events: e1000.tx_desc_busy_events,
        e1000_tx_lock_contention_events: e1000.tx_lock_contention_events,
        e1000_io_errors: e1000.io_errors,
    }
}

pub fn slo_report() -> NetworkDriverSloReport {
    let stats = stats();
    let tx_total = stats.tx_to_nic_frames.saturating_add(stats.tx_to_nic_drops);
    let rx_total = stats
        .rx_to_core_frames
        .saturating_add(stats.rx_to_core_drops);
    let drops_total = stats.tx_to_nic_drops.saturating_add(stats.rx_to_core_drops);
    let traffic_total = tx_total.saturating_add(rx_total);
    let drop_rate_per_mille = if traffic_total == 0 {
        0
    } else {
        drops_total.saturating_mul(1000) / traffic_total
    };

    let cfg = get_config();
    let (tx_depth, tx_limit, rx_depth, rx_limit) = match stats.active_driver {
        ActiveNetworkDriver::VirtIo => (
            stats.virtio_tx_depth,
            cfg.virtio_ring_limit,
            stats.virtio_rx_depth,
            cfg.virtio_ring_limit,
        ),
        ActiveNetworkDriver::E1000 => (
            stats.e1000_tx_depth,
            cfg.e1000_ring_limit,
            stats.e1000_rx_depth,
            cfg.e1000_ring_limit,
        ),
        ActiveNetworkDriver::None => (0, 1, 0, 1),
    };

    let tx_ring_utilization_percent =
        ((tx_depth as u64).saturating_mul(100)) / (tx_limit.max(1) as u64);
    let rx_ring_utilization_percent =
        ((rx_depth as u64).saturating_mul(100)) / (rx_limit.max(1) as u64);
    let driver_io_errors = stats.e1000_io_errors;
    let thresholds = slo_thresholds();

    let drop_rate_breach = drop_rate_per_mille > thresholds.max_drop_rate_per_mille;
    let tx_ring_breach = tx_ring_utilization_percent > thresholds.max_tx_ring_utilization_percent;
    let rx_ring_breach = rx_ring_utilization_percent > thresholds.max_rx_ring_utilization_percent;
    let io_error_breach = driver_io_errors > thresholds.max_driver_io_errors;

    let mut breach_count = 0u8;
    if drop_rate_breach {
        breach_count = breach_count.saturating_add(1);
    }
    if tx_ring_breach {
        breach_count = breach_count.saturating_add(1);
    }
    if rx_ring_breach {
        breach_count = breach_count.saturating_add(1);
    }
    if io_error_breach {
        breach_count = breach_count.saturating_add(1);
    }

    NetworkDriverSloReport {
        driver: stats.active_driver,
        drop_rate_per_mille,
        tx_ring_utilization_percent,
        rx_ring_utilization_percent,
        driver_io_errors,
        drop_rate_breach,
        tx_ring_breach,
        rx_ring_breach,
        io_error_breach,
        breach_count,
    }
}

pub fn clear_driver_queues(driver: ActiveNetworkDriver) -> NetworkQueueResetSummary {
    let mut summary = NetworkQueueResetSummary {
        driver,
        cleared_virtio_rx: 0,
        cleared_virtio_tx: 0,
        cleared_e1000_rx: 0,
        cleared_e1000_tx: 0,
    };

    match driver {
        ActiveNetworkDriver::VirtIo => {
            let mut rx = VIRTIO_RX_RING.lock();
            summary.cleared_virtio_rx = rx.len();
            rx.clear();
            let mut tx = VIRTIO_TX_RING.lock();
            summary.cleared_virtio_tx = tx.len();
            tx.clear();
        }
        ActiveNetworkDriver::E1000 => {
            let mut rx = E1000_RX_RING.lock();
            summary.cleared_e1000_rx = rx.len();
            rx.clear();
            let mut tx = E1000_TX_RING.lock();
            summary.cleared_e1000_tx = tx.len();
            tx.clear();
        }
        ActiveNetworkDriver::None => {}
    }

    summary
}

pub fn clear_active_driver_queues() -> NetworkQueueResetSummary {
    clear_driver_queues(active_driver())
}
