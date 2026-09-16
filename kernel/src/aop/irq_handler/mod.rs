//! irq_handler module.

use aop_macros::irq_handler;
use alloc::format;

#[cfg(feature = "host_examples")]
#[irq_handler(priority = 5)]
pub fn example_irq() {
    crate::core::log::info("IRQ handler example running");
}

#[cfg(all(test, feature = "host_examples"))]
mod tests {
    use super::*;

    #[test_case]
    fn test_irq_handler() {
        example_irq();
    }
}
