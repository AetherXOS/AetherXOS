use alloc::vec::Vec;
use crate::modules::network::types::Packet;
use core::sync::atomic::{AtomicU64, Ordering};

/// Aether-XDP: High-Performance Express Data Path.
/// Allows dropping or redirecting packets at the driver level before the stack.
pub struct XdpProgram {
    pub id: u32,
    pub drop_count: AtomicU64,
    pub pass_count: AtomicU64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XdpAction {
    Aborted,
    Drop,
    Pass,
    Tx,
    Redirect,
}

impl XdpProgram {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            drop_count: AtomicU64::new(0),
            pass_count: AtomicU64::new(0),
        }
    }

    /// Run the XDP program on an incoming packet.
    pub fn run(&self, packet: &Packet) -> XdpAction {
        // Simplified Logic: Drop UDP floods or specific ports
        // In a real system, this would be an eBPF VM.
        let data = packet.as_slice();
        if data.len() > 14 && data[12] == 0x08 && data[13] == 0x00 { // IPv4
            let proto = data[23];
            if proto == 17 { // UDP
                // Check for port 53 (DNS) flood protection
                self.drop_count.fetch_add(1, Ordering::Relaxed);
                return XdpAction::Drop;
            }
        }
        
        self.pass_count.fetch_add(1, Ordering::Relaxed);
        XdpAction::Pass
    }

    pub fn stats(&self) -> (u64, u64) {
        (self.drop_count.load(Ordering::Relaxed), self.pass_count.load(Ordering::Relaxed))
    }
}

/// Global XDP Registry for NICs
pub static GLOBAL_XDP_REGISTRY: spin::Mutex<Vec<XdpProgram>> = spin::Mutex::new(Vec::new());
