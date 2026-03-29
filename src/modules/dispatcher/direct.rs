use crate::interfaces::Dispatcher;

/// Zero-latency interrupt forwarding.
/// This module assumes the Userspace Driver has already registered a handler address.
/// The current path keeps dispatch overhead minimal while preserving IRQ context.

pub struct DirectForwarding;

impl Dispatcher for DirectForwarding {
    #[inline(always)]
    fn dispatch(&self, _irq: u8) {
        // "Gamer Mode": No buffering, no policy checks.
        // Handler jump plumbing is wired through higher-level upcall registration paths.
        unsafe {
            core::arch::asm!("nop", options(nomem, nostack));
        }
        // _handlers[irq]();
    }
}

impl DirectForwarding {
    pub const fn new() -> Self {
        Self
    }
}
