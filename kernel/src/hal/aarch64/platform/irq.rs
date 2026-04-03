use crate::hal::common::irq_catalog::{IrqLineDescriptor, IrqLineId, IrqLineKind};
use crate::hal::common::irq_registration::enable_interrupt_lines;
use crate::interfaces::InterruptController;

pub const TIMER_IRQ_CNTPNSIRQ: u32 = 27;
pub const TIMER_IRQ_CNTVIRQ: u32 = 30;
pub const UART_IRQ_QEMU_VIRT_SPI1: u32 = 33;

pub const TIMER_IRQ_CNTPNSIRQ_ID: IrqLineId = IrqLineId::new(TIMER_IRQ_CNTPNSIRQ);
pub const TIMER_IRQ_CNTVIRQ_ID: IrqLineId = IrqLineId::new(TIMER_IRQ_CNTVIRQ);
pub const UART_IRQ_QEMU_VIRT_SPI1_ID: IrqLineId = IrqLineId::new(UART_IRQ_QEMU_VIRT_SPI1);

pub const IRQ_LINE_DESCRIPTORS: [IrqLineDescriptor<u32>; 3] = [
    IrqLineDescriptor::new(TIMER_IRQ_CNTPNSIRQ, IrqLineKind::Timer, "timer-phys-ns"),
    IrqLineDescriptor::new(TIMER_IRQ_CNTVIRQ, IrqLineKind::Timer, "timer-virt"),
    IrqLineDescriptor::new(UART_IRQ_QEMU_VIRT_SPI1, IrqLineKind::Serial, "uart-pl011"),
];

pub const ENABLED_IRQ_LINES: [u32; 3] = [
    TIMER_IRQ_CNTPNSIRQ,
    TIMER_IRQ_CNTVIRQ,
    UART_IRQ_QEMU_VIRT_SPI1,
];

const _AARCH64_IRQ_DESCRIPTOR_COUNT_ASSERT: [(); ENABLED_IRQ_LINES.len()] =
    [(); IRQ_LINE_DESCRIPTORS.len()];

pub fn enable_platform_irq_lines(controller: &mut impl InterruptController) {
    enable_interrupt_lines(controller, &ENABLED_IRQ_LINES);
}
