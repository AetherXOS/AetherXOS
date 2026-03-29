use crate::config::KernelConfig;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::cmp;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

mod metrics;

pub use metrics::{
    clear_active_driver_queues, clear_driver_queues, slo_report, stats, NetworkDataplaneStats,
    NetworkDriverSloReport, NetworkQueueResetSummary,
};

const DEFAULT_MAX_DROP_RATE_PER_MILLE: u64 = 25;
const DEFAULT_MAX_TX_RING_UTIL_PERCENT: u64 = 90;
const DEFAULT_MAX_RX_RING_UTIL_PERCENT: u64 = 90;
const DEFAULT_MAX_DRIVER_IO_ERRORS: u64 = 0;

#[derive(Debug, Clone, Copy)]
pub struct NetworkDriverConfig {
    pub irq_service_budget: usize,
    pub loop_service_budget: usize,
    pub virtio_ring_limit: usize,
    pub e1000_ring_limit: usize,
}

impl NetworkDriverConfig {
    pub fn from_kernel_config() -> Self {
        Self {
            irq_service_budget: KernelConfig::driver_network_irq_service_budget(),
            loop_service_budget: KernelConfig::driver_network_loop_service_budget(),
            virtio_ring_limit: KernelConfig::driver_network_ring_limit(),
            e1000_ring_limit: KernelConfig::driver_network_ring_limit(),
        }
    }

    pub const fn sanitized(self) -> Self {
        Self {
            irq_service_budget: if self.irq_service_budget == 0 {
                1
            } else {
                self.irq_service_budget
            },
            loop_service_budget: if self.loop_service_budget == 0 {
                1
            } else {
                self.loop_service_budget
            },
            virtio_ring_limit: if self.virtio_ring_limit == 0 {
                1
            } else {
                self.virtio_ring_limit
            },
            e1000_ring_limit: if self.e1000_ring_limit == 0 {
                1
            } else {
                self.e1000_ring_limit
            },
        }
    }
}

impl Default for NetworkDriverConfig {
    fn default() -> Self {
        Self::from_kernel_config()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkPollProfile {
    LowLatency,
    Balanced,
    Throughput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveNetworkDriver {
    None,
    VirtIo,
    E1000,
}

#[derive(Debug, Clone, Copy)]
pub struct NetworkDriverSloThresholds {
    pub max_drop_rate_per_mille: u64,
    pub max_tx_ring_utilization_percent: u64,
    pub max_rx_ring_utilization_percent: u64,
    pub max_driver_io_errors: u64,
}

pub use super::network_io_health::{
    evaluate_network_io_health_action, NetworkIoHealthAction, NetworkIoHealthHarness,
};

static ACTIVE_DRIVER: AtomicU64 = AtomicU64::new(0);
static POLL_PROFILE: AtomicU64 = AtomicU64::new(profile_to_u64(NetworkPollProfile::Balanced));
static DRIVER_IO_OWNED: AtomicBool = AtomicBool::new(false);
static REGISTER_VIRTIO_CALLS: AtomicU64 = AtomicU64::new(0);
static REGISTER_E1000_CALLS: AtomicU64 = AtomicU64::new(0);
static SERVICE_CALLS: AtomicU64 = AtomicU64::new(0);
static IRQ_SERVICE_CALLS: AtomicU64 = AtomicU64::new(0);
static TX_TO_NIC_FRAMES: AtomicU64 = AtomicU64::new(0);
static TX_TO_NIC_DROPS: AtomicU64 = AtomicU64::new(0);
static RX_TO_CORE_FRAMES: AtomicU64 = AtomicU64::new(0);
static RX_TO_CORE_DROPS: AtomicU64 = AtomicU64::new(0);
static MAX_DROP_RATE_PER_MILLE: AtomicU64 = AtomicU64::new(DEFAULT_MAX_DROP_RATE_PER_MILLE);
static MAX_TX_RING_UTIL_PERCENT: AtomicU64 = AtomicU64::new(DEFAULT_MAX_TX_RING_UTIL_PERCENT);
static MAX_RX_RING_UTIL_PERCENT: AtomicU64 = AtomicU64::new(DEFAULT_MAX_RX_RING_UTIL_PERCENT);
static MAX_DRIVER_IO_ERRORS: AtomicU64 = AtomicU64::new(DEFAULT_MAX_DRIVER_IO_ERRORS);

lazy_static! {
    pub static ref VIRTIO_RX_RING: Mutex<VecDeque<Vec<u8>>> = Mutex::new(VecDeque::new());
    pub static ref VIRTIO_TX_RING: Mutex<VecDeque<Vec<u8>>> = Mutex::new(VecDeque::new());
    pub static ref E1000_RX_RING: Mutex<VecDeque<Vec<u8>>> = Mutex::new(VecDeque::new());
    pub static ref E1000_TX_RING: Mutex<VecDeque<Vec<u8>>> = Mutex::new(VecDeque::new());
    static ref NETWORK_DRIVER_CONFIG: Mutex<NetworkDriverConfig> =
        Mutex::new(NetworkDriverConfig::default());
}

#[inline(always)]
fn config() -> NetworkDriverConfig {
    *NETWORK_DRIVER_CONFIG.lock()
}

pub fn set_config(config: NetworkDriverConfig) {
    *NETWORK_DRIVER_CONFIG.lock() = config.sanitized();
}

pub fn apply_runtime_config_from_kernel_config() {
    set_config(NetworkDriverConfig::from_kernel_config());
    set_slo_thresholds(NetworkDriverSloThresholds {
        max_drop_rate_per_mille: KernelConfig::driver_network_slo_max_drop_rate_per_mille(),
        max_tx_ring_utilization_percent:
            KernelConfig::driver_network_slo_max_tx_ring_utilization_percent(),
        max_rx_ring_utilization_percent:
            KernelConfig::driver_network_slo_max_rx_ring_utilization_percent(),
        max_driver_io_errors: KernelConfig::driver_network_slo_max_io_errors(),
    });
}

pub fn get_config() -> NetworkDriverConfig {
    config()
}

pub fn configure_service_budgets(loop_budget: usize, irq_budget: usize) {
    let mut cfg = NETWORK_DRIVER_CONFIG.lock();
    cfg.loop_service_budget = loop_budget.max(1);
    cfg.irq_service_budget = irq_budget.max(1);
}

pub fn configure_ring_limit(limit: usize) {
    let mut cfg = NETWORK_DRIVER_CONFIG.lock();
    let limit = limit.max(1);
    cfg.virtio_ring_limit = limit;
    cfg.e1000_ring_limit = limit;
}

#[inline(always)]
fn driver_to_u64(driver: ActiveNetworkDriver) -> u64 {
    match driver {
        ActiveNetworkDriver::None => 0,
        ActiveNetworkDriver::VirtIo => 1,
        ActiveNetworkDriver::E1000 => 2,
    }
}

#[inline(always)]
fn driver_from_u64(raw: u64) -> ActiveNetworkDriver {
    match raw {
        1 => ActiveNetworkDriver::VirtIo,
        2 => ActiveNetworkDriver::E1000,
        _ => ActiveNetworkDriver::None,
    }
}

#[inline(always)]
const fn profile_to_u64(profile: NetworkPollProfile) -> u64 {
    match profile {
        NetworkPollProfile::LowLatency => 0,
        NetworkPollProfile::Balanced => 1,
        NetworkPollProfile::Throughput => 2,
    }
}

#[inline(always)]
fn profile_from_u64(raw: u64) -> NetworkPollProfile {
    match raw {
        0 => NetworkPollProfile::LowLatency,
        2 => NetworkPollProfile::Throughput,
        _ => NetworkPollProfile::Balanced,
    }
}

pub fn poll_profile() -> NetworkPollProfile {
    profile_from_u64(POLL_PROFILE.load(Ordering::Relaxed))
}

pub fn set_poll_profile(profile: NetworkPollProfile) {
    POLL_PROFILE.store(profile_to_u64(profile), Ordering::Relaxed);
}

pub fn slo_thresholds() -> NetworkDriverSloThresholds {
    NetworkDriverSloThresholds {
        max_drop_rate_per_mille: MAX_DROP_RATE_PER_MILLE.load(Ordering::Relaxed),
        max_tx_ring_utilization_percent: MAX_TX_RING_UTIL_PERCENT.load(Ordering::Relaxed),
        max_rx_ring_utilization_percent: MAX_RX_RING_UTIL_PERCENT.load(Ordering::Relaxed),
        max_driver_io_errors: MAX_DRIVER_IO_ERRORS.load(Ordering::Relaxed),
    }
}

pub fn set_slo_thresholds(thresholds: NetworkDriverSloThresholds) {
    MAX_DROP_RATE_PER_MILLE.store(
        thresholds.max_drop_rate_per_mille.min(1000),
        Ordering::Relaxed,
    );
    MAX_TX_RING_UTIL_PERCENT.store(
        thresholds.max_tx_ring_utilization_percent.min(100),
        Ordering::Relaxed,
    );
    MAX_RX_RING_UTIL_PERCENT.store(
        thresholds.max_rx_ring_utilization_percent.min(100),
        Ordering::Relaxed,
    );
    MAX_DRIVER_IO_ERRORS.store(thresholds.max_driver_io_errors, Ordering::Relaxed);
}

pub fn apply_poll_profile(profile: NetworkPollProfile) {
    let defaults = NetworkDriverConfig::from_kernel_config();
    let ll_irq_divisor = KernelConfig::driver_network_low_latency_irq_budget_divisor();
    let ll_loop_divisor = KernelConfig::driver_network_low_latency_loop_budget_divisor();
    let ll_ring_divisor = KernelConfig::driver_network_low_latency_ring_limit_divisor();
    let tp_irq_multiplier = KernelConfig::driver_network_throughput_irq_budget_multiplier();
    let tp_loop_multiplier = KernelConfig::driver_network_throughput_loop_budget_multiplier();
    let tp_ring_multiplier = KernelConfig::driver_network_throughput_ring_limit_multiplier();
    let mut cfg = NETWORK_DRIVER_CONFIG.lock();
    match profile {
        NetworkPollProfile::LowLatency => {
            cfg.irq_service_budget = cmp::max(defaults.irq_service_budget / ll_irq_divisor, 1);
            cfg.loop_service_budget = cmp::max(defaults.loop_service_budget / ll_loop_divisor, 1);
            cfg.virtio_ring_limit = cmp::max(defaults.virtio_ring_limit / ll_ring_divisor, 1);
            cfg.e1000_ring_limit = cmp::max(defaults.e1000_ring_limit / ll_ring_divisor, 1);
        }
        NetworkPollProfile::Balanced => {
            *cfg = defaults;
        }
        NetworkPollProfile::Throughput => {
            cfg.irq_service_budget = defaults
                .irq_service_budget
                .saturating_mul(tp_irq_multiplier);
            cfg.loop_service_budget = defaults
                .loop_service_budget
                .saturating_mul(tp_loop_multiplier);
            cfg.virtio_ring_limit = defaults
                .virtio_ring_limit
                .saturating_mul(tp_ring_multiplier);
            cfg.e1000_ring_limit = defaults.e1000_ring_limit.saturating_mul(tp_ring_multiplier);
        }
    }
    *cfg = cfg.sanitized();
    set_poll_profile(profile);
}

pub fn register_virtio() {
    REGISTER_VIRTIO_CALLS.fetch_add(1, Ordering::Relaxed);
    ACTIVE_DRIVER.store(
        driver_to_u64(ActiveNetworkDriver::VirtIo),
        Ordering::Relaxed,
    );
}

pub fn register_e1000() {
    REGISTER_E1000_CALLS.fetch_add(1, Ordering::Relaxed);
    ACTIVE_DRIVER.store(driver_to_u64(ActiveNetworkDriver::E1000), Ordering::Relaxed);
}

pub fn active_driver() -> ActiveNetworkDriver {
    driver_from_u64(ACTIVE_DRIVER.load(Ordering::Relaxed))
}

pub fn clear_active_driver() {
    ACTIVE_DRIVER.store(driver_to_u64(ActiveNetworkDriver::None), Ordering::Relaxed);
    DRIVER_IO_OWNED.store(false, Ordering::Relaxed);
}

pub fn set_driver_io_owned(enabled: bool) {
    DRIVER_IO_OWNED.store(enabled, Ordering::Relaxed);
}

pub fn driver_io_owned() -> bool {
    DRIVER_IO_OWNED.load(Ordering::Relaxed)
}

pub fn has_active_driver() -> bool {
    active_driver() != ActiveNetworkDriver::None
}

pub fn service_queues() {
    service_queues_with_budget(config().loop_service_budget);
}

pub fn service_irq(driver: ActiveNetworkDriver) {
    IRQ_SERVICE_CALLS.fetch_add(1, Ordering::Relaxed);
    if active_driver() == driver {
        service_queues_with_budget(config().irq_service_budget);
    }
}

pub fn inject_rx_frame(frame: Vec<u8>) -> Result<(), &'static str> {
    let cfg = config();
    match active_driver() {
        ActiveNetworkDriver::VirtIo => {
            let mut rx = VIRTIO_RX_RING.lock();
            if rx.len() >= cfg.virtio_ring_limit {
                RX_TO_CORE_DROPS.fetch_add(1, Ordering::Relaxed);
                return Err("virtio rx ring full");
            }
            rx.push_back(frame);
            Ok(())
        }
        ActiveNetworkDriver::E1000 => {
            let mut rx = E1000_RX_RING.lock();
            if rx.len() >= cfg.e1000_ring_limit {
                RX_TO_CORE_DROPS.fetch_add(1, Ordering::Relaxed);
                return Err("e1000 rx ring full");
            }
            rx.push_back(frame);
            Ok(())
        }
        ActiveNetworkDriver::None => Err("no active network driver"),
    }
}

fn service_queues_with_budget(budget: usize) {
    let budget = cmp::max(budget, 1);
    SERVICE_CALLS.fetch_add(1, Ordering::Relaxed);

    let tx_frames = crate::kernel::net_core::drain_tx_frames(budget);
    if !tx_frames.is_empty() {
        push_tx_frames_to_active_nic(tx_frames);
    }

    if !driver_io_owned() {
        simulate_driver_loopback(active_driver(), budget);
    }
    pull_rx_frames_into_core(budget);
}

fn push_tx_frames_to_active_nic(tx_frames: Vec<Vec<u8>>) {
    let cfg = config();
    match active_driver() {
        ActiveNetworkDriver::VirtIo => {
            let mut ring = VIRTIO_TX_RING.lock();
            for frame in tx_frames {
                if ring.len() >= cfg.virtio_ring_limit {
                    TX_TO_NIC_DROPS.fetch_add(1, Ordering::Relaxed);
                    continue;
                }
                ring.push_back(frame);
                TX_TO_NIC_FRAMES.fetch_add(1, Ordering::Relaxed);
            }
        }
        ActiveNetworkDriver::E1000 => {
            let mut ring = E1000_TX_RING.lock();
            for frame in tx_frames {
                if ring.len() >= cfg.e1000_ring_limit {
                    TX_TO_NIC_DROPS.fetch_add(1, Ordering::Relaxed);
                    continue;
                }
                ring.push_back(frame);
                TX_TO_NIC_FRAMES.fetch_add(1, Ordering::Relaxed);
            }
        }
        ActiveNetworkDriver::None => {
            TX_TO_NIC_DROPS.fetch_add(tx_frames.len() as u64, Ordering::Relaxed);
        }
    }
}

fn simulate_driver_loopback(driver: ActiveNetworkDriver, budget: usize) {
    let cfg = config();
    for _ in 0..budget {
        let frame = match driver {
            ActiveNetworkDriver::VirtIo => VIRTIO_TX_RING.lock().pop_front(),
            ActiveNetworkDriver::E1000 => E1000_TX_RING.lock().pop_front(),
            ActiveNetworkDriver::None => None,
        };

        let Some(frame) = frame else {
            break;
        };

        match driver {
            ActiveNetworkDriver::VirtIo => {
                let mut rx = VIRTIO_RX_RING.lock();
                if rx.len() >= cfg.virtio_ring_limit {
                    RX_TO_CORE_DROPS.fetch_add(1, Ordering::Relaxed);
                    continue;
                }
                rx.push_back(frame);
            }
            ActiveNetworkDriver::E1000 => {
                let mut rx = E1000_RX_RING.lock();
                if rx.len() >= cfg.e1000_ring_limit {
                    RX_TO_CORE_DROPS.fetch_add(1, Ordering::Relaxed);
                    continue;
                }
                rx.push_back(frame);
            }
            ActiveNetworkDriver::None => {}
        }
    }
}

fn pull_rx_frames_into_core(budget: usize) {
    let Some(mut rx) = active_rx_ring(active_driver()) else {
        return;
    };

    for _ in 0..budget {
        let Some(frame) = rx.pop_front() else {
            break;
        };
        if crate::kernel::net_core::submit_rx_frame(frame).is_ok() {
            RX_TO_CORE_FRAMES.fetch_add(1, Ordering::Relaxed);
        } else {
            RX_TO_CORE_DROPS.fetch_add(1, Ordering::Relaxed);
        }
    }
}

fn active_rx_ring(
    driver: ActiveNetworkDriver,
) -> Option<spin::MutexGuard<'static, VecDeque<Vec<u8>>>> {
    match driver {
        ActiveNetworkDriver::VirtIo => Some(VIRTIO_RX_RING.lock()),
        ActiveNetworkDriver::E1000 => Some(E1000_RX_RING.lock()),
        ActiveNetworkDriver::None => None,
    }
}

#[cfg(test)]
#[path = "network/tests.rs"]
mod tests;
