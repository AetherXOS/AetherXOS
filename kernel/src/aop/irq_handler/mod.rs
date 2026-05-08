use aop_macros::irq_handler;
use alloc::format;

#[irq_handler(priority = 5)]
pub fn example_irq() {
    crate::core::log::info("IRQ handler example running");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_irq_handler() {
        example_irq();
    }
}
