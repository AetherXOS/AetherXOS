use alloc::collections::VecDeque;
use alloc::vec;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use smoltcp::iface::{Config, Interface, SocketSet};
use smoltcp::phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken};
use smoltcp::time::Instant;
use smoltcp::wire::EthernetAddress;
use smoltcp::wire::{HardwareAddress, IpAddress, IpCidr};
use spin::Mutex;

use super::{MacAddress, NetworkInterface};
use super::metrics_ops::update_loopback_high_water;

pub(super) fn smoltcp_ethernet_address(mac: MacAddress) -> EthernetAddress {
    let MacAddress::Ethernet(bytes) = mac;
    EthernetAddress(bytes)
}

pub(super) fn init_smoltcp_bridge(nic: &dyn NetworkInterface) -> EthernetAddress {
    super::SMOLTCP_BRIDGE_INITS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    smoltcp_ethernet_address(nic.mac())
}

struct LoopbackSmolDevice {
    queue: VecDeque<Vec<u8>>,
    mtu: usize,
}

impl LoopbackSmolDevice {
    fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            mtu: 1536,
        }
    }
}

struct LoopbackRxToken {
    frame: Vec<u8>,
}

struct LoopbackTxToken<'a> {
    queue: &'a mut VecDeque<Vec<u8>>,
    mtu: usize,
}

impl RxToken for LoopbackRxToken {
    fn consume<R, F>(self, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut frame = self.frame;
        super::SMOLTCP_RX_FRAMES.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        f(&mut frame)
    }
}

impl TxToken for LoopbackTxToken<'_> {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let alloc_len = core::cmp::min(len, self.mtu);
        let mut buffer = vec![0u8; alloc_len];
        let out = f(&mut buffer);
        if self.queue.len() >= crate::config::KernelConfig::network_loopback_queue_limit() {
            super::LOOPBACK_SEND_DROPS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            return out;
        }
        self.queue.push_back(buffer);
        update_loopback_high_water(self.queue.len());
        super::SMOLTCP_TX_FRAMES.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        out
    }
}

impl Device for LoopbackSmolDevice {
    type RxToken<'a>
        = LoopbackRxToken
    where
        Self: 'a;

    type TxToken<'a>
        = LoopbackTxToken<'a>
    where
        Self: 'a;

    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let frame = self.queue.pop_front()?;
        Some((
            LoopbackRxToken { frame },
            LoopbackTxToken {
                queue: &mut self.queue,
                mtu: self.mtu,
            },
        ))
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        Some(LoopbackTxToken {
            queue: &mut self.queue,
            mtu: self.mtu,
        })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = self.mtu;
        caps.medium = Medium::Ethernet;
        caps
    }
}

struct SmoltcpRuntime {
    device: LoopbackSmolDevice,
    iface: Interface,
}

lazy_static! {
    static ref SMOLTCP_RUNTIME: Mutex<Option<SmoltcpRuntime>> = Mutex::new(None);
}

pub(super) fn runtime_ready() -> bool {
    SMOLTCP_RUNTIME.lock().is_some()
}

pub(super) fn init_smoltcp_runtime(nic: &dyn NetworkInterface) -> Result<(), &'static str> {
    if SMOLTCP_RUNTIME.lock().is_some() {
        return Ok(());
    }

    let mac = init_smoltcp_bridge(nic);
    let hw = HardwareAddress::Ethernet(mac);

    let mut config = Config::new(hw);
    config.random_seed = 0xC0DEC0DE;

    let mut device = LoopbackSmolDevice::new();
    let mut iface = Interface::new(config, &mut device, Instant::from_millis(0));
    iface.update_ip_addrs(|addrs| {
        // Standard IPv4 setup
        if addrs
            .push(IpCidr::new(IpAddress::v4(10, 0, 2, 15), 24))
            .is_err()
        {
            super::SMOLTCP_INIT_ERRORS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        }

        // Link-Local IPv6 based on MAC (EUI-64) for Neighbor Discovery / ICMPv6
        let mut eui64 = [0u8; 8];
        eui64[0] = mac.0[0] ^ 0x02;
        eui64[1] = mac.0[1];
        eui64[2] = mac.0[2];
        eui64[3] = 0xFF;
        eui64[4] = 0xFE;
        eui64[5] = mac.0[3];
        eui64[6] = mac.0[4];
        eui64[7] = mac.0[5];

        let ll_addr = smoltcp::wire::Ipv6Address([
            0xfe, 0x80, 0, 0, 0, 0, 0, 0, eui64[0], eui64[1], eui64[2], eui64[3], eui64[4],
            eui64[5], eui64[6], eui64[7],
        ]);

        if addrs
            .push(IpCidr::new(IpAddress::Ipv6(ll_addr), 64))
            .is_err()
        {
            super::SMOLTCP_INIT_ERRORS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        }
    });

    *SMOLTCP_RUNTIME.lock() = Some(SmoltcpRuntime { device, iface });
    Ok(())
}

pub(super) fn reinitialize_smoltcp_runtime(nic: &dyn NetworkInterface) -> Result<(), &'static str> {
    super::SMOLTCP_REINITIALIZE_CALLS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    *SMOLTCP_RUNTIME.lock() = None;
    super::reset_runtime_stats();
    init_smoltcp_runtime(nic)
}

pub(super) fn poll_smoltcp_runtime() -> bool {
    if !super::runtime_polling_enabled() {
        super::SMOLTCP_POLL_SKIPS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        return false;
    }

    let interval = super::runtime_poll_interval_ticks();
    let tick = super::SMOLTCP_TICKS.fetch_add(1, core::sync::atomic::Ordering::Relaxed) + 1;
    if interval > 1 && tick % interval != 0 {
        super::SMOLTCP_POLL_SKIPS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        return false;
    }

    let mut runtime = SMOLTCP_RUNTIME.lock();
    let Some(rt) = runtime.as_mut() else {
        super::SMOLTCP_POLL_ERRORS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        return false;
    };

    let now = Instant::from_millis(tick as i64);
    super::SMOLTCP_POLLS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);

    let mut sockets = SocketSet::new(vec![]);
    rt.iface.poll(now, &mut rt.device, &mut sockets)
}

pub(super) fn force_poll_once() -> bool {
    super::SMOLTCP_FORCE_POLLS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    let mut runtime = SMOLTCP_RUNTIME.lock();
    let Some(rt) = runtime.as_mut() else {
        super::SMOLTCP_POLL_ERRORS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        return false;
    };

    let tick = super::SMOLTCP_TICKS.fetch_add(1, core::sync::atomic::Ordering::Relaxed) as i64;
    let now = Instant::from_millis(tick);
    super::SMOLTCP_POLLS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);

    let mut sockets = SocketSet::new(vec![]);
    rt.iface.poll(now, &mut rt.device, &mut sockets)
}

pub(super) fn ingest_raw_ethernet_frame(frame: Vec<u8>) -> Result<(), &'static str> {
    let mut runtime = SMOLTCP_RUNTIME.lock();
    let Some(rt) = runtime.as_mut() else {
        super::SMOLTCP_POLL_ERRORS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        return Err("smoltcp runtime unavailable");
    };

    if rt.device.queue.len() >= crate::config::KernelConfig::network_loopback_queue_limit() {
        super::LOOPBACK_SEND_DROPS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        return Err("runtime frame queue full");
    }

    rt.device.queue.push_back(frame);
    update_loopback_high_water(rt.device.queue.len());
    Ok(())
}

pub(super) fn ingest_raw_ethernet_frames(frames: Vec<Vec<u8>>) -> usize {
    if frames.is_empty() {
        return 0;
    }
    let total = frames.len();

    let mut runtime = SMOLTCP_RUNTIME.lock();
    let Some(rt) = runtime.as_mut() else {
        super::SMOLTCP_POLL_ERRORS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        return 0;
    };

    let available = crate::config::KernelConfig::network_loopback_queue_limit()
        .saturating_sub(rt.device.queue.len());
    let mut accepted = 0usize;
    for frame in frames.into_iter().take(available) {
        rt.device.queue.push_back(frame);
        accepted += 1;
    }
    let dropped = total.saturating_sub(accepted);
    if dropped > 0 {
        super::LOOPBACK_SEND_DROPS.fetch_add(dropped as u64, core::sync::atomic::Ordering::Relaxed);
    }
    update_loopback_high_water(rt.device.queue.len());
    accepted
}
