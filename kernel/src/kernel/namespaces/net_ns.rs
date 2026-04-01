/// Network Namespace — isolated network stack.
///
/// Each network namespace has its own set of network interfaces, routing
/// table, firewall rules, and socket tables.
use super::{alloc_ns_id, NsId};
use alloc::string::String;
use alloc::vec::Vec;

/// A virtual network interface entry within a namespace.
#[derive(Debug, Clone)]
pub struct NsNetInterface {
    pub name: String,
    pub index: u32,
    pub up: bool,
}

/// A single network namespace.
pub struct NetNamespace {
    pub id: NsId,
    /// Network interfaces visible in this namespace.
    pub interfaces: Vec<NsNetInterface>,
    /// Whether the loopback interface has been brought up.
    pub loopback_up: bool,
}

impl NetNamespace {
    /// Create the root (init) network namespace.
    pub fn root() -> Self {
        Self {
            id: alloc_ns_id(),
            interfaces: Vec::new(),
            loopback_up: true,
        }
    }

    /// Create a new empty network namespace (only loopback).
    pub fn new() -> Self {
        Self {
            id: alloc_ns_id(),
            interfaces: Vec::new(),
            loopback_up: false,
        }
    }

    /// Add an interface to this namespace.
    pub fn add_interface(&mut self, name: String) -> u32 {
        let index = self.interfaces.len() as u32 + 1;
        self.interfaces.push(NsNetInterface {
            name,
            index,
            up: false,
        });
        index
    }

    /// Bring an interface up.
    pub fn set_interface_up(&mut self, index: u32) -> bool {
        for iface in &mut self.interfaces {
            if iface.index == index {
                iface.up = true;
                return true;
            }
        }
        false
    }

    /// Bring an interface down.
    pub fn set_interface_down(&mut self, index: u32) -> bool {
        for iface in &mut self.interfaces {
            if iface.index == index {
                iface.up = false;
                return true;
            }
        }
        false
    }

    /// Move an interface from this namespace into `target`.
    pub fn move_interface_to(&mut self, index: u32, target: &mut Self) -> bool {
        if let Some(pos) = self.interfaces.iter().position(|i| i.index == index) {
            let iface = self.interfaces.remove(pos);
            target.interfaces.push(iface);
            true
        } else {
            false
        }
    }

    /// Bring up the loopback interface.
    pub fn bring_up_loopback(&mut self) {
        self.loopback_up = true;
    }
}
