mod network;
mod timer;

#[cfg(feature = "drivers")]
pub(super) use network::{e1000_irq_handler, virtio_irq_handler};
pub(crate) use timer::timer_tick_handler;
