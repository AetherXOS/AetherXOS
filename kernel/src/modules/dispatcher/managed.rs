use crate::interfaces::Dispatcher;
use crate::kernel::sync::IrqSafeMutex;
use alloc::collections::BinaryHeap;

/// Managed Interrupt Dispatcher.
/// Uses a Priority Queue to handle high-priority IRQs first.
/// Deterministic and suitable for Real-Time systems.

pub struct Managed {
    queue: IrqSafeMutex<BinaryHeap<IrqEvent>>,
}

#[derive(Eq, PartialEq)]
struct IrqEvent {
    irq: u8,
    priority: u8,
}

// Order by priority (Higher is better)
impl Ord for IrqEvent {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.priority.cmp(&other.priority)
    }
}

impl PartialOrd for IrqEvent {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Default for Managed {
    fn default() -> Self {
        Self::new()
    }
}

impl Managed {
    pub const fn new() -> Self {
        Self {
            queue: IrqSafeMutex::new(BinaryHeap::new()),
        }
    }

    fn get_priority(&self, irq: u8) -> u8 {
        // Hardware Specific Priority Mapping
        // This could be loaded from a configuration table in the future.
        match irq {
            0..=31 => 255,  // Exceptions/Traps (Highest)
            32..=47 => 100, // Hardware IRQs
            _ => 10,
        }
    }
}

impl Dispatcher for Managed {
    fn dispatch(&self, irq: u8) {
        let priority = self.get_priority(irq);
        self.queue.lock().push(IrqEvent { irq, priority });
    }
}

impl Managed {
    pub fn poll(&self) -> Option<u8> {
        self.queue.lock().pop().map(|e| e.irq)
    }
}
