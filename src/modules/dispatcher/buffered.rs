use crate::interfaces::Dispatcher;
use crate::kernel::sync::IrqSafeMutex;
use alloc::collections::VecDeque;

/// Buffered Interrupt Dispatcher.
/// Stores interrupts in a ring buffer for the LibOS to poll.
/// Safe, decoupled, but higher latency than DirectForwarding.

pub struct Buffered {
    queue: IrqSafeMutex<VecDeque<u8>>,
    capacity: usize,
}

impl Buffered {
    pub fn new() -> Self {
        Self {
            queue: IrqSafeMutex::new(VecDeque::new()),
            capacity: crate::generated_consts::DISPATCHER_BUFFER_SIZE, // Fixed size buffer limit
        }
    }
}

impl Dispatcher for Buffered {
    fn dispatch(&self, irq: u8) {
        let mut q = self.queue.lock();
        if q.len() < self.capacity {
            q.push_back(irq);
        } else {
            // Buffer overflow policy: Drop oldest or Drop newest.
            // Here: Drop oldest
            q.pop_front();
            q.push_back(irq);
        }
    }
}

// Method for LibOS to poll
impl Buffered {
    pub fn poll(&self) -> Option<u8> {
        self.queue.lock().pop_front()
    }
}
