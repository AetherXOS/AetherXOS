use alloc::vec::Vec;
use core::cmp;

use crate::hal::pci::PciDevice;

use super::lifecycle::{
    DriverClass, DriverErrorKind, DriverIoGate, DriverStateMachine, PciProbeDriver,
};
use super::probe::{pci_bar0_io_base, pci_id, probe_first_pci_by_ids};
use crate::impl_lifecycle_adapter;

mod queue;
mod regs;

use queue::{VirtControlQueue, VirtQueue, VirtQueueRole};
use regs::*;

#[cfg(target_arch = "x86_64")]
use x86_64::instructions::port::Port;

#[cfg(not(target_arch = "x86_64"))]
struct Port<T> {
    _phantom: core::marker::PhantomData<T>,
}

#[cfg(not(target_arch = "x86_64"))]
impl<T: Default + Copy> Port<T> {
    fn new(_port: u16) -> Self {
        Self {
            _phantom: core::marker::PhantomData,
        }
    }

    unsafe fn read(&mut self) -> T {
        T::default()
    }

    unsafe fn write(&mut self, _value: T) {}
}

/// VirtIO Network Driver (Legacy)
///
/// Implements a baseline legacy virtio-net dataplane with RX/TX virtqueue support
/// and optional control queue commands when device features expose it.
pub struct VirtIoNet {
    pub io_base: u16,
    pub irq: u8,
    negotiated_features: u32,
    queue_size: u16,
    software_budget: usize,
    tx_submitted_frames: u64,
    tx_completed_frames: u64,
    rx_completed_frames: u64,
    dropped_frames: u64,
    control_queue_ops: u64,
    control_queue_failures: u64,
    mac_address: [u8; 6],
    rx_queue: Option<VirtQueue>,
    tx_queue: Option<VirtQueue>,
    control_queue: Option<VirtControlQueue>,
    lifecycle: DriverStateMachine,
}

impl VirtIoNet {
    pub fn probe(devices: &[PciDevice]) -> Option<Self> {
        let ids = [pci_id(
            crate::hal::pci::VENDOR_REDHAT,
            crate::hal::pci::VIRTIO_DEV_NET_LEGACY,
        )];
        let dev = probe_first_pci_by_ids(devices, &ids)?;
        let io_base = pci_bar0_io_base(dev)?;
        Some(Self {
            io_base,
            irq: dev.interrupt_line,
            negotiated_features: 0,
            queue_size: 0,
            software_budget: 64,
            tx_submitted_frames: 0,
            tx_completed_frames: 0,
            rx_completed_frames: 0,
            dropped_frames: 0,
            control_queue_ops: 0,
            control_queue_failures: 0,
            mac_address: [0u8; 6],
            rx_queue: None,
            tx_queue: None,
            control_queue: None,
            lifecycle: DriverStateMachine::new_discovered(),
        })
    }

    fn setup_virtqueue(
        &self,
        queue_index: u16,
        role: VirtQueueRole,
        hhdm: u64,
    ) -> Result<VirtQueue, &'static str> {
        let mut queue_select_port = Port::<u16>::new(self.io_base + VIRTIO_REG_QUEUE_SELECT);
        let mut queue_size_port = Port::<u16>::new(self.io_base + VIRTIO_REG_QUEUE_SIZE);
        let mut queue_address_port = Port::<u32>::new(self.io_base + VIRTIO_REG_QUEUE_ADDRESS);

        unsafe {
            queue_select_port.write(queue_index);
            let offered_size = queue_size_port.read();
            if offered_size == 0 {
                return Err("virtio queue size is zero");
            }
            let negotiated_size = cmp::min(offered_size, VIRTIO_QUEUE_MAX_SIZE);
            let queue = VirtQueue::new(queue_index, negotiated_size, role, hhdm)?;
            let queue_phys = queue.queue_phys_addr()?;
            let pfn = queue_phys >> 12;
            if pfn > u32::MAX as u64 {
                return Err("virtio queue address pfn overflow");
            }
            queue_address_port.write(pfn as u32);
            Ok(queue)
        }
    }

    #[inline(always)]
    fn has_feature(&self, feature_bit: u32) -> bool {
        (self.negotiated_features & feature_bit) != 0
    }

    fn setup_control_queue(&self, hhdm: u64) -> Result<VirtControlQueue, &'static str> {
        let mut queue_select_port = Port::<u16>::new(self.io_base + VIRTIO_REG_QUEUE_SELECT);
        let mut queue_size_port = Port::<u16>::new(self.io_base + VIRTIO_REG_QUEUE_SIZE);
        let mut queue_address_port = Port::<u32>::new(self.io_base + VIRTIO_REG_QUEUE_ADDRESS);

        unsafe {
            queue_select_port.write(VIRTIO_QUEUE_CTRL);
            let offered_size = queue_size_port.read();
            if offered_size < 2 {
                return Err("virtio control queue size is insufficient");
            }
            let negotiated_size = cmp::min(offered_size, VIRTIO_QUEUE_MAX_SIZE);
            let ctrl_queue = VirtControlQueue::new(VIRTIO_QUEUE_CTRL, negotiated_size, hhdm)?;
            let queue_phys = ctrl_queue.queue_phys_addr()?;
            let pfn = queue_phys >> 12;
            if pfn > u32::MAX as u64 {
                return Err("virtio control queue address pfn overflow");
            }
            queue_address_port.write(pfn as u32);
            Ok(ctrl_queue)
        }
    }

    fn read_config_mac(&self) -> [u8; 6] {
        let mut mac = [0u8; 6];
        for (i, byte) in mac.iter_mut().enumerate() {
            let mut port = Port::<u8>::new(self.io_base + VIRTIO_NET_CONFIG_MAC_OFFSET + i as u16);
            unsafe {
                *byte = port.read();
            }
        }
        mac
    }

    fn send_control_command(
        &mut self,
        class: u8,
        cmd: u8,
        payload: &[u8],
    ) -> Result<(), &'static str> {
        if payload.len() + 2 > VIRTIO_CTRL_MAX_CMD_BYTES {
            return Err("virtio control payload too large");
        }
        let queue_index = {
            let Some(ctrl) = self.control_queue.as_ref() else {
                return Err("virtio control queue unavailable");
            };
            ctrl.queue_index()
        };
        {
            let Some(ctrl) = self.control_queue.as_mut() else {
                return Err("virtio control queue unavailable");
            };
            let mut command = [0u8; VIRTIO_CTRL_MAX_CMD_BYTES];
            command[0] = class;
            command[1] = cmd;
            if !payload.is_empty() {
                command[2..2 + payload.len()].copy_from_slice(payload);
            }
            ctrl.prepare_command(&command[..2 + payload.len()])?;
        }

        self.queue_notify(queue_index);
        let status = {
            let Some(ctrl) = self.control_queue.as_mut() else {
                return Err("virtio control queue unavailable");
            };
            ctrl.wait_completion(VIRTIO_CTRL_TIMEOUT_SPINS)?
        };
        self.control_queue_ops = self.control_queue_ops.saturating_add(1);
        if status != VIRTIO_CTRL_STATUS_OK {
            self.control_queue_failures = self.control_queue_failures.saturating_add(1);
            return Err("virtio control command rejected");
        }
        Ok(())
    }

    pub fn control_queue_available(&self) -> bool {
        self.control_queue.is_some()
    }

    pub fn mac_address(&self) -> [u8; 6] {
        self.mac_address
    }

    pub fn control_queue_stats(&self) -> (u64, u64) {
        (self.control_queue_ops, self.control_queue_failures)
    }

    pub fn set_promiscuous_mode(&mut self, enabled: bool) -> Result<(), &'static str> {
        if !self.has_feature(VIRTIO_NET_F_CTRL_VQ) || !self.has_feature(VIRTIO_NET_F_CTRL_RX) {
            return Err("virtio control rx not supported");
        }
        self.send_control_command(
            VIRTIO_NET_CTRL_RX_CLASS,
            VIRTIO_NET_CTRL_RX_PROMISC_CMD,
            &[if enabled { 1 } else { 0 }],
        )
    }

    pub fn set_mac_address(&mut self, mac: [u8; 6]) -> Result<(), &'static str> {
        if !self.has_feature(VIRTIO_NET_F_CTRL_VQ) || !self.has_feature(VIRTIO_NET_F_MAC) {
            return Err("virtio mac control not supported");
        }
        self.send_control_command(
            VIRTIO_NET_CTRL_MAC_CLASS,
            VIRTIO_NET_CTRL_MAC_ADDR_SET_CMD,
            &mac,
        )?;
        self.mac_address = mac;
        Ok(())
    }

    fn queue_notify(&self, queue_index: u16) {
        let mut notify_port = Port::<u16>::new(self.io_base + VIRTIO_REG_QUEUE_NOTIFY);
        unsafe {
            notify_port.write(queue_index);
        }
    }

    fn read_isr_status(&self) -> u8 {
        let mut isr_port = Port::<u8>::new(self.io_base + VIRTIO_REG_ISR_STATUS);
        unsafe { isr_port.read() }
    }

    pub fn init(&mut self) -> Result<(), &'static str> {
        self.lifecycle.on_init_start();
        self.control_queue = None;
        self.control_queue_ops = 0;
        self.control_queue_failures = 0;

        let mut status_port = Port::<u8>::new(self.io_base + VIRTIO_REG_DEVICE_STATUS);
        let mut device_features_port = Port::<u32>::new(self.io_base + VIRTIO_REG_DEVICE_FEATURES);
        let mut guest_features_port = Port::<u32>::new(self.io_base + VIRTIO_REG_GUEST_FEATURES);
        let hhdm = crate::hal::hhdm_offset().ok_or("HHDM not available for virtio queues")?;
        let negotiated_features: u32;

        unsafe {
            status_port.write(0);
            let mut status = STATUS_ACKNOWLEDGE;
            status_port.write(status);
            status |= STATUS_DRIVER;
            status_port.write(status);

            let features = device_features_port.read();
            guest_features_port.write(features);
            negotiated_features = features;

            status |= STATUS_FEATURES_OK;
            status_port.write(status);
            let new_status = status_port.read();
            if new_status & STATUS_FEATURES_OK == 0 {
                status |= STATUS_FAILED;
                status_port.write(status);
                self.lifecycle.on_init_failure(DriverErrorKind::Init);
                return Err("VirtIO Features Negotiation Failed");
            }
        }
        self.negotiated_features = negotiated_features;

        let rx_queue = self
            .setup_virtqueue(VIRTIO_QUEUE_RX, VirtQueueRole::Rx, hhdm)
            .map_err(|e| {
                self.lifecycle.on_init_failure(DriverErrorKind::Init);
                e
            })?;
        let tx_queue = self
            .setup_virtqueue(VIRTIO_QUEUE_TX, VirtQueueRole::Tx, hhdm)
            .map_err(|e| {
                self.lifecycle.on_init_failure(DriverErrorKind::Init);
                e
            })?;
        self.queue_size = cmp::min(rx_queue.queue_size(), tx_queue.queue_size());
        self.rx_queue = Some(rx_queue);
        self.tx_queue = Some(tx_queue);
        if let Some(rx_queue) = self.rx_queue.as_ref() {
            self.queue_notify(rx_queue.queue_index());
        }

        if self.has_feature(VIRTIO_NET_F_MAC) {
            self.mac_address = self.read_config_mac();
        }

        if self.has_feature(VIRTIO_NET_F_CTRL_VQ) {
            let ctrl_queue = self.setup_control_queue(hhdm).map_err(|e| {
                self.lifecycle.on_init_failure(DriverErrorKind::Init);
                e
            })?;
            self.control_queue = Some(ctrl_queue);
            if self.has_feature(VIRTIO_NET_F_CTRL_RX) && self.set_promiscuous_mode(false).is_err() {
                self.lifecycle.on_init_failure(DriverErrorKind::Init);
                return Err("VirtIO control RX setup failed");
            }
        }

        unsafe {
            let mut status = status_port.read();
            status |= STATUS_DRIVER_OK;
            status_port.write(status);
        }

        self.software_budget = crate::modules::drivers::network::get_config().irq_service_budget;
        self.lifecycle.on_init_success();
        Ok(())
    }

    fn lifecycle_init(&mut self) -> Result<(), &'static str> {
        self.init()
    }

    fn lifecycle_service(&mut self) -> Result<(), &'static str> {
        match self.lifecycle.io_gate() {
            DriverIoGate::Open => {}
            DriverIoGate::Cooldown => return Err("virtio-net recovery cooldown active"),
            DriverIoGate::Closed => return Err("virtio-net driver unhealthy"),
        }

        if self.queue_size == 0 {
            self.lifecycle.on_io_failure(DriverErrorKind::InvalidConfig);
            return Err("virtio-net queue not initialized");
        }
        if self.rx_queue.is_none() || self.tx_queue.is_none() {
            self.lifecycle.on_io_failure(DriverErrorKind::InvalidConfig);
            return Err("virtio-net virtqueue state missing");
        }

        let runtime_budget = crate::modules::drivers::network::get_config()
            .irq_service_budget
            .max(1);
        let queue_budget = self.queue_size as usize;
        self.software_budget = runtime_budget
            .min(queue_budget.max(1))
            .min(MAX_SOFTWARE_BUDGET);

        crate::modules::drivers::service_network_irq(
            crate::modules::drivers::ActiveNetworkDriver::VirtIo,
        );

        let mut staged: Vec<Vec<u8>> = Vec::new();
        {
            let mut tx_ring = crate::modules::drivers::network::VIRTIO_TX_RING.lock();
            for _ in 0..self.software_budget.max(1) {
                let Some(frame) = tx_ring.pop_front() else {
                    break;
                };
                staged.push(frame);
            }
        }

        let mut notify_tx = false;
        let mut dropped = 0u64;
        {
            let Some(tx_queue) = self.tx_queue.as_mut() else {
                self.lifecycle.on_io_failure(DriverErrorKind::InvalidConfig);
                return Err("virtio tx queue unavailable");
            };
            for frame in staged {
                if tx_queue.submit_tx_frame(&frame).is_ok() {
                    self.tx_submitted_frames = self.tx_submitted_frames.saturating_add(1);
                    notify_tx = true;
                } else {
                    self.dropped_frames = self.dropped_frames.saturating_add(1);
                    dropped = dropped.saturating_add(1);
                }
            }
            let completed = tx_queue.poll_tx_completions(self.software_budget.max(1));
            self.tx_completed_frames = self.tx_completed_frames.saturating_add(completed as u64);
        }
        if notify_tx {
            self.queue_notify(VIRTIO_QUEUE_TX);
        }

        let notify_rx;
        let mut rx_frames = Vec::new();
        {
            let Some(rx_queue) = self.rx_queue.as_mut() else {
                self.lifecycle.on_io_failure(DriverErrorKind::InvalidConfig);
                return Err("virtio rx queue unavailable");
            };
            let (completed, rearmed) =
                rx_queue.poll_rx_frames(self.software_budget.max(1), &mut rx_frames);
            self.rx_completed_frames = self.rx_completed_frames.saturating_add(completed as u64);
            notify_rx = rearmed > 0;
        }
        if notify_rx {
            self.queue_notify(VIRTIO_QUEUE_RX);
        }

        for frame in rx_frames {
            if crate::modules::drivers::inject_network_rx_frame(frame).is_err() {
                self.dropped_frames = self.dropped_frames.saturating_add(1);
                dropped = dropped.saturating_add(1);
            }
        }

        let isr = self.read_isr_status();
        if (isr & 0x1) != 0 {
            if let Some(tx_queue) = self.tx_queue.as_mut() {
                let completed = tx_queue.poll_tx_completions(self.software_budget.max(1));
                self.tx_completed_frames =
                    self.tx_completed_frames.saturating_add(completed as u64);
            }
            let mut irq_frames = Vec::new();
            let mut irq_notify_rx = false;
            if let Some(rx_queue) = self.rx_queue.as_mut() {
                let (completed, rearmed) =
                    rx_queue.poll_rx_frames(self.software_budget.max(1), &mut irq_frames);
                self.rx_completed_frames =
                    self.rx_completed_frames.saturating_add(completed as u64);
                irq_notify_rx = rearmed > 0;
            }
            if irq_notify_rx {
                self.queue_notify(VIRTIO_QUEUE_RX);
            }
            for frame in irq_frames {
                if crate::modules::drivers::inject_network_rx_frame(frame).is_err() {
                    self.dropped_frames = self.dropped_frames.saturating_add(1);
                    dropped = dropped.saturating_add(1);
                }
            }
        }

        crate::modules::drivers::service_network_irq(
            crate::modules::drivers::ActiveNetworkDriver::VirtIo,
        );
        if dropped > 0 {
            self.lifecycle.on_io_failure(DriverErrorKind::Io);
        } else {
            self.lifecycle.on_io_success();
        }
        Ok(())
    }

    fn lifecycle_teardown(&mut self) -> Result<(), &'static str> {
        self.rx_queue = None;
        self.tx_queue = None;
        self.control_queue = None;
        self.lifecycle.on_teardown();
        Ok(())
    }
}

impl PciProbeDriver for VirtIoNet {
    fn probe_pci(devices: &[PciDevice]) -> Option<Self> {
        Self::probe(devices)
    }
}

impl_lifecycle_adapter!(
    for VirtIoNet,
    class: DriverClass::Network,
    name: "virtio-net",
    lifecycle: lifecycle,
    init: lifecycle_init,
    service: lifecycle_service,
    teardown: lifecycle_teardown,
);
