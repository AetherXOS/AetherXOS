use crate::config::KernelConfig;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{AtomicU64, Ordering};

use crate::hal::pci::PciDevice;

use super::lifecycle::{
    DriverClass, DriverErrorKind, DriverIoGate, DriverStateMachine, PciProbeDriver,
};
use super::probe::{device_matches_any_pci_id, pci_bar0_mmio_base, pci_id, PciId};
use crate::impl_lifecycle_adapter;

const E1000_CTRL: usize = 0x0000;
const E1000_ICR: usize = 0x00C0;
const E1000_IMS: usize = 0x00D0;
const E1000_RCTL: usize = 0x0100;
const E1000_TCTL: usize = 0x0400;

const E1000_RDBAL: usize = 0x2800;
const E1000_RDBAH: usize = 0x2804;
const E1000_RDLEN: usize = 0x2808;
const E1000_RDH: usize = 0x2810;
const E1000_RDT: usize = 0x2818;

const E1000_TDBAL: usize = 0x3800;
const E1000_TDBAH: usize = 0x3804;
const E1000_TDLEN: usize = 0x3808;
const E1000_TDH: usize = 0x3810;
const E1000_TDT: usize = 0x3818;

const E1000_CTRL_RST: u32 = 1 << 26;
const E1000_CTRL_SLU: u32 = 1 << 6; // Set Link Up

const RCTL_EN: u32 = 1 << 1; // Receiver Enable
const RCTL_BSIZE_2048: u32 = 0; // 2048 Byte Buffer Size
const RCTL_SBP: u32 = 1 << 2; // Store Bad Packets
const RCTL_UPE: u32 = 1 << 3; // Unicast Promiscuous Enabled
const RCTL_MPE: u32 = 1 << 4; // Multicast Promiscuous Enabled
const RCTL_BAM: u32 = 1 << 15; // Broadcast Accept Mode
const RCTL_SECRC: u32 = 1 << 26; // Strip Ethernet CRC

const TCTL_EN: u32 = 1 << 1; // Transmit Enable
const TCTL_PSP: u32 = 1 << 3; // Pad Short Packets

const INTEL_VENDOR_ID: u16 = 0x8086;
const E1000_PCI_IDS: &[PciId] = &[
    pci_id(INTEL_VENDOR_ID, 0x100E), // 82540EM (QEMU common)
    pci_id(INTEL_VENDOR_ID, 0x100F), // 82545EM
    pci_id(INTEL_VENDOR_ID, 0x10D3), // 82574L
    pci_id(INTEL_VENDOR_ID, 0x153A), // I217-LM
];

const E1000_TX_DESC_DONE: u8 = 1;

#[derive(Debug, Clone, Copy)]
pub struct E1000DataplaneStats {
    pub io_calls: u64,
    pub rx_frames: u64,
    pub rx_bytes: u64,
    pub rx_invalid_len: u64,
    pub rx_delivery_drops: u64,
    pub tx_frames: u64,
    pub tx_bytes: u64,
    pub tx_truncated_frames: u64,
    pub tx_desc_busy_events: u64,
    pub tx_lock_contention_events: u64,
    pub io_errors: u64,
}

static E1000_IO_CALLS: AtomicU64 = AtomicU64::new(0);
static E1000_RX_FRAMES: AtomicU64 = AtomicU64::new(0);
static E1000_RX_BYTES: AtomicU64 = AtomicU64::new(0);
static E1000_RX_INVALID_LEN: AtomicU64 = AtomicU64::new(0);
static E1000_RX_DELIVERY_DROPS: AtomicU64 = AtomicU64::new(0);
static E1000_TX_FRAMES: AtomicU64 = AtomicU64::new(0);
static E1000_TX_BYTES: AtomicU64 = AtomicU64::new(0);
static E1000_TX_TRUNCATED_FRAMES: AtomicU64 = AtomicU64::new(0);
static E1000_TX_DESC_BUSY_EVENTS: AtomicU64 = AtomicU64::new(0);
static E1000_TX_LOCK_CONTENTION_EVENTS: AtomicU64 = AtomicU64::new(0);
static E1000_IO_ERRORS: AtomicU64 = AtomicU64::new(0);
static E1000_RESET_TIMEOUTS: AtomicU64 = AtomicU64::new(0);

pub fn dataplane_stats() -> E1000DataplaneStats {
    E1000DataplaneStats {
        io_calls: E1000_IO_CALLS.load(Ordering::Relaxed),
        rx_frames: E1000_RX_FRAMES.load(Ordering::Relaxed),
        rx_bytes: E1000_RX_BYTES.load(Ordering::Relaxed),
        rx_invalid_len: E1000_RX_INVALID_LEN.load(Ordering::Relaxed),
        rx_delivery_drops: E1000_RX_DELIVERY_DROPS.load(Ordering::Relaxed),
        tx_frames: E1000_TX_FRAMES.load(Ordering::Relaxed),
        tx_bytes: E1000_TX_BYTES.load(Ordering::Relaxed),
        tx_truncated_frames: E1000_TX_TRUNCATED_FRAMES.load(Ordering::Relaxed),
        tx_desc_busy_events: E1000_TX_DESC_BUSY_EVENTS.load(Ordering::Relaxed),
        tx_lock_contention_events: E1000_TX_LOCK_CONTENTION_EVENTS.load(Ordering::Relaxed),
        io_errors: E1000_IO_ERRORS.load(Ordering::Relaxed),
    }
}

pub fn reset_dataplane_stats() {
    E1000_IO_CALLS.store(0, Ordering::Relaxed);
    E1000_RX_FRAMES.store(0, Ordering::Relaxed);
    E1000_RX_BYTES.store(0, Ordering::Relaxed);
    E1000_RX_INVALID_LEN.store(0, Ordering::Relaxed);
    E1000_RX_DELIVERY_DROPS.store(0, Ordering::Relaxed);
    E1000_TX_FRAMES.store(0, Ordering::Relaxed);
    E1000_TX_BYTES.store(0, Ordering::Relaxed);
    E1000_TX_TRUNCATED_FRAMES.store(0, Ordering::Relaxed);
    E1000_TX_DESC_BUSY_EVENTS.store(0, Ordering::Relaxed);
    E1000_TX_LOCK_CONTENTION_EVENTS.store(0, Ordering::Relaxed);
    E1000_IO_ERRORS.store(0, Ordering::Relaxed);
}

#[derive(Debug, Clone, Copy)]
pub struct E1000WaitStats {
    pub reset_timeout_spins: usize,
    pub reset_timeouts: u64,
}

pub fn wait_stats() -> E1000WaitStats {
    E1000WaitStats {
        reset_timeout_spins: KernelConfig::e1000_reset_timeout_spins(),
        reset_timeouts: E1000_RESET_TIMEOUTS.load(Ordering::Relaxed),
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct E1000RxDesc {
    pub addr: u64,
    pub len: u16,
    pub csum: u16,
    pub status: u8,
    pub errors: u8,
    pub special: u16,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct E1000TxDesc {
    pub addr: u64,
    pub len: u16,
    pub cso: u8,
    pub cmd: u8,
    pub status: u8,
    pub css: u8,
    pub special: u16,
}

pub struct E1000 {
    pub mmio_base: u64,
    pub irq: u8,
    pub device_id: u16,
    lifecycle: DriverStateMachine,
    rx_descs: Box<[E1000RxDesc]>,
    tx_descs: Box<[E1000TxDesc]>,
    rx_bufs: Vec<Vec<u8>>,
    tx_bufs: Vec<Vec<u8>>,
    rx_cur: usize,
    tx_cur: usize,
}

impl E1000 {
    pub fn probe(devices: &[PciDevice]) -> Option<Self> {
        for dev in devices {
            if !device_matches_any_pci_id(dev, E1000_PCI_IDS) {
                continue;
            }
            let Some(mmio_base) = pci_bar0_mmio_base(*dev) else {
                continue;
            };

            return Some(Self {
                // Descriptor counts are runtime-tunable through KernelConfig.
                mmio_base,
                irq: dev.interrupt_line,
                device_id: dev.device_id,
                lifecycle: DriverStateMachine::new_discovered(),
                rx_descs: alloc::vec![
                    E1000RxDesc { addr: 0, len: 0, csum: 0, status: 0, errors: 0, special: 0 };
                    KernelConfig::e1000_rx_desc_count()
                ]
                .into_boxed_slice(),
                tx_descs: alloc::vec![
                    E1000TxDesc { addr: 0, len: 0, cso: 0, cmd: 0, status: 0, css: 0, special: 0 };
                    KernelConfig::e1000_tx_desc_count()
                ]
                .into_boxed_slice(),
                rx_bufs: Vec::new(),
                tx_bufs: Vec::new(),
                rx_cur: 0,
                tx_cur: 0,
            });
        }
        None
    }

    fn write_reg(&self, offset: usize, value: u32) {
        if let Some(hhdm) = crate::hal::hhdm_offset() {
            let addr = (self.mmio_base + hhdm + (offset as u64)) as *mut u32;
            unsafe { write_volatile(addr, value) };
        }
    }

    fn read_reg(&self, offset: usize) -> u32 {
        if let Some(hhdm) = crate::hal::hhdm_offset() {
            let addr = (self.mmio_base + hhdm + (offset as u64)) as *const u32;
            unsafe { read_volatile(addr) }
        } else {
            0
        }
    }

    pub fn init(&mut self) -> Result<(), &'static str> {
        self.lifecycle.on_init_start();

        let hhdm = crate::hal::hhdm_offset().ok_or("HHDM not available")?;

        // 1. Reset
        let ctrl = self.read_reg(E1000_CTRL);
        self.write_reg(E1000_CTRL, ctrl | E1000_CTRL_RST);
        let mut waited = 0;
        loop {
            if (self.read_reg(E1000_CTRL) & E1000_CTRL_RST) == 0 {
                break;
            }
            waited += 1;
            if waited > KernelConfig::e1000_reset_timeout_spins() {
                E1000_RESET_TIMEOUTS.fetch_add(1, Ordering::Relaxed);
                self.lifecycle.on_init_failure(DriverErrorKind::Timeout);
                return Err("E1000 reset timeout");
            }
            core::hint::spin_loop();
        }

        self.write_reg(E1000_CTRL, self.read_reg(E1000_CTRL) | E1000_CTRL_SLU); // Set Link Up
        let buf_size = KernelConfig::e1000_buffer_size_bytes();
        let rx_desc_count = self.rx_descs.len();
        let tx_desc_count = self.tx_descs.len();

        // Setup RX
        for i in 0..rx_desc_count {
            let buf = alloc::vec![0u8; buf_size];
            let phys = buf.as_ptr() as u64 - hhdm;
            self.rx_bufs.push(buf);
            self.rx_descs[i] = E1000RxDesc {
                addr: phys,
                len: 0,
                csum: 0,
                status: 0,
                errors: 0,
                special: 0,
            };
        }

        // Setup TX
        for i in 0..tx_desc_count {
            let buf = alloc::vec![0u8; buf_size];
            self.tx_bufs.push(buf);
            self.tx_descs[i] = E1000TxDesc {
                addr: 0,
                len: 0,
                cso: 0,
                cmd: 0,
                status: E1000_TX_DESC_DONE,
                css: 0,
                special: 0,
            };
        }

        let rx_phys = self.rx_descs.as_ptr() as u64 - hhdm;
        self.write_reg(E1000_RDBAL, rx_phys as u32);
        self.write_reg(E1000_RDBAH, (rx_phys >> 32) as u32);
        self.write_reg(
            E1000_RDLEN,
            (rx_desc_count * core::mem::size_of::<E1000RxDesc>()) as u32,
        );
        self.write_reg(E1000_RDH, 0);
        self.write_reg(E1000_RDT, (rx_desc_count - 1) as u32);

        let tx_phys = self.tx_descs.as_ptr() as u64 - hhdm;
        self.write_reg(E1000_TDBAL, tx_phys as u32);
        self.write_reg(E1000_TDBAH, (tx_phys >> 32) as u32);
        self.write_reg(
            E1000_TDLEN,
            (tx_desc_count * core::mem::size_of::<E1000TxDesc>()) as u32,
        );
        self.write_reg(E1000_TDH, 0);
        self.write_reg(E1000_TDT, 0);

        // Enables
        self.write_reg(
            E1000_RCTL,
            RCTL_EN | RCTL_SBP | RCTL_UPE | RCTL_MPE | RCTL_BAM | RCTL_SECRC | RCTL_BSIZE_2048,
        );
        self.write_reg(E1000_TCTL, TCTL_EN | TCTL_PSP | (15 << 4) | (0x40 << 12));

        self.write_reg(E1000_IMS, 0x1F6DC); // Enable interrupts

        self.lifecycle.on_init_success();
        Ok(())
    }

    fn lifecycle_init(&mut self) -> Result<(), &'static str> {
        self.init()
    }

    fn lifecycle_service(&mut self) -> Result<(), &'static str> {
        E1000_IO_CALLS.fetch_add(1, Ordering::Relaxed);
        match self.lifecycle.io_gate() {
            DriverIoGate::Open => {}
            _ => return Err("e1000 unhealthy"),
        }

        let budget = crate::modules::drivers::network::get_config()
            .irq_service_budget
            .max(1);
        let Some(hhdm) = crate::hal::hhdm_offset() else {
            self.lifecycle.on_io_failure(DriverErrorKind::InvalidConfig);
            E1000_IO_ERRORS.fetch_add(1, Ordering::Relaxed);
            return Err("hhdm not available");
        };
        let mut had_error = false;
        let buf_size = KernelConfig::e1000_buffer_size_bytes();

        let _icr = self.read_reg(E1000_ICR); // Read and clear Interrupt Cause

        // RX Service
        let mut rx_processed = 0usize;
        while rx_processed < budget && (self.rx_descs[self.rx_cur].status & 1) == 1 {
            // 1 == DD (Descriptor Done) bit
            let len = self.rx_descs[self.rx_cur].len as usize;
            if len == 0 || len > buf_size {
                had_error = true;
                E1000_RX_INVALID_LEN.fetch_add(1, Ordering::Relaxed);
            }

            // Extract packet
            if len > 0 && len <= buf_size {
                let packet = self.rx_bufs[self.rx_cur][..len].to_vec();
                if crate::modules::drivers::inject_network_rx_frame(packet).is_err() {
                    had_error = true;
                    E1000_RX_DELIVERY_DROPS.fetch_add(1, Ordering::Relaxed);
                } else {
                    E1000_RX_FRAMES.fetch_add(1, Ordering::Relaxed);
                    E1000_RX_BYTES.fetch_add(len as u64, Ordering::Relaxed);
                }
            }

            // Restore the descriptor
            self.rx_descs[self.rx_cur].status = 0;
            let next = (self.rx_cur + 1) % self.rx_descs.len();
            self.write_reg(E1000_RDT, self.rx_cur as u32);
            self.rx_cur = next;
            rx_processed += 1;
        }

        // TX Service
        if let Some(mut tx_ring) = crate::modules::drivers::network::E1000_TX_RING.try_lock() {
            let mut tx_processed = 0usize;
            while tx_processed < budget {
                if (self.tx_descs[self.tx_cur].status & E1000_TX_DESC_DONE) == 0 {
                    E1000_TX_DESC_BUSY_EVENTS.fetch_add(1, Ordering::Relaxed);
                    break;
                }

                let Some(pkt) = tx_ring.pop_front() else {
                    break;
                };

                let mut copied_len = pkt.len();
                if copied_len > buf_size {
                    copied_len = buf_size;
                    had_error = true;
                    E1000_TX_TRUNCATED_FRAMES.fetch_add(1, Ordering::Relaxed);
                }
                self.tx_bufs[self.tx_cur][..copied_len].copy_from_slice(&pkt[..copied_len]);

                self.tx_descs[self.tx_cur].addr = self.tx_bufs[self.tx_cur].as_ptr() as u64 - hhdm;
                self.tx_descs[self.tx_cur].len = copied_len as u16;
                self.tx_descs[self.tx_cur].cmd = (1 << 3) | (1 << 1) | 1; // EOP, IFCS, RS
                self.tx_descs[self.tx_cur].status = 0;

                self.tx_cur = (self.tx_cur + 1) % self.tx_descs.len();
                self.write_reg(E1000_TDT, self.tx_cur as u32);
                tx_processed += 1;
                E1000_TX_FRAMES.fetch_add(1, Ordering::Relaxed);
                E1000_TX_BYTES.fetch_add(copied_len as u64, Ordering::Relaxed);
            }
        } else {
            E1000_TX_LOCK_CONTENTION_EVENTS.fetch_add(1, Ordering::Relaxed);
        }

        crate::modules::drivers::service_network_irq(
            crate::modules::drivers::ActiveNetworkDriver::E1000,
        );
        if had_error {
            self.lifecycle.on_io_failure(DriverErrorKind::Io);
            E1000_IO_ERRORS.fetch_add(1, Ordering::Relaxed);
        } else {
            self.lifecycle.on_io_success();
        }
        Ok(())
    }

    fn lifecycle_teardown(&mut self) -> Result<(), &'static str> {
        self.lifecycle.on_teardown();
        Ok(())
    }
}

impl PciProbeDriver for E1000 {
    fn probe_pci(devices: &[PciDevice]) -> Option<Self> {
        Self::probe(devices)
    }
}

impl_lifecycle_adapter!(
    for E1000,
    class: DriverClass::Network,
    name: "e1000",
    lifecycle: lifecycle,
    init: lifecycle_init,
    service: lifecycle_service,
    teardown: lifecycle_teardown,
);
