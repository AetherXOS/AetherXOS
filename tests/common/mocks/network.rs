use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use alloc::vec::Vec;

pub struct MockNetworkInterface {
    pub connected: AtomicBool,
    pub tx_count: AtomicUsize,
    pub rx_count: AtomicUsize,
    pub mtu: AtomicUsize,
    pub mac_address: [u8; 6],
}

impl MockNetworkInterface {
    pub fn new(mac: [u8; 6]) -> Self {
        Self {
            connected: AtomicBool::new(false),
            tx_count: AtomicUsize::new(0),
            rx_count: AtomicUsize::new(0),
            mtu: AtomicUsize::new(1500),
            mac_address: mac,
        }
    }

    pub fn connect(&self) {
        self.connected.store(true, Ordering::SeqCst);
    }

    pub fn disconnect(&self) {
        self.connected.store(false, Ordering::SeqCst);
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    pub fn transmit(&self, _data: &[u8]) -> Result<usize, &'static str> {
        if !self.is_connected() {
            return Err("Not connected");
        }
        self.tx_count.fetch_add(1, Ordering::SeqCst);
        Ok(0)
    }

    pub fn receive(&self, _buffer: &mut [u8]) -> Result<usize, &'static str> {
        if !self.is_connected() {
            return Err("Not connected");
        }
        self.rx_count.fetch_add(1, Ordering::SeqCst);
        Ok(0)
    }

    pub fn get_tx_count(&self) -> usize {
        self.tx_count.load(Ordering::SeqCst)
    }

    pub fn get_rx_count(&self) -> usize {
        self.rx_count.load(Ordering::SeqCst)
    }

    pub fn get_mtu(&self) -> usize {
        self.mtu.load(Ordering::SeqCst)
    }

    pub fn get_mac(&self) -> [u8; 6] {
        self.mac_address
    }
}

pub struct MockNetworkStack {
    pub interfaces: Vec<MockNetworkInterface>,
}

impl MockNetworkStack {
    pub fn new() -> Self {
        Self {
            interfaces: Vec::new(),
        }
    }

    pub fn add_interface(&mut self, iface: MockNetworkInterface) {
        self.interfaces.push(iface);
    }

    pub fn interface_count(&self) -> usize {
        self.interfaces.len()
    }
}

impl Default for MockNetworkStack {
    fn default() -> Self {
        Self::new()
    }
}
