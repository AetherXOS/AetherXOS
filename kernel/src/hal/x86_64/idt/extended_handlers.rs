#[cfg(target_os = "none")]
use crate::kernel::syscalls::syscalls_consts::x86;
#[cfg(target_os = "none")]
use crate::hal::common::irq_registration::IrqRoute;
#[cfg(target_os = "none")]
use x86_64::structures::idt::InterruptStackFrame;
#[cfg(target_os = "none")]
use super::metadata;

#[cfg(target_os = "none")]
macro_rules! define_irq_handlers {
    ($(($name:ident, $vector:expr)),+ $(,)?) => {
        $(
            extern "x86-interrupt" fn $name(_stack_frame: InterruptStackFrame) {
                super::dispatch::dispatch_irq_vector($vector);
            }
        )+

        pub(super) const EXTENDED_IRQ_ROUTES:
            [IrqRoute<u8, extern "x86-interrupt" fn(InterruptStackFrame)>; metadata::EXTENDED_IRQ_COUNT as usize] = [
            $(IrqRoute::new($vector, $name),)+
        ];
    };
}

#[cfg(target_os = "none")]
define_irq_handlers!(
    (irq_33_handler, x86::IRQ_VECTOR_BASE + 1),
    (irq_34_handler, x86::IRQ_VECTOR_BASE + 2),
    (irq_35_handler, x86::IRQ_VECTOR_BASE + 3),
    (irq_36_handler, x86::IRQ_VECTOR_BASE + 4),
    (irq_37_handler, x86::IRQ_VECTOR_BASE + 5),
    (irq_38_handler, x86::IRQ_VECTOR_BASE + 6),
    (irq_39_handler, x86::IRQ_VECTOR_BASE + 7),
    (irq_40_handler, x86::IRQ_VECTOR_BASE + 8),
    (irq_41_handler, x86::IRQ_VECTOR_BASE + 9),
    (irq_42_handler, x86::IRQ_VECTOR_BASE + 10),
    (irq_43_handler, x86::IRQ_VECTOR_BASE + 11),
    (irq_44_handler, x86::IRQ_VECTOR_BASE + 12),
    (irq_45_handler, x86::IRQ_VECTOR_BASE + 13),
    (irq_46_handler, x86::IRQ_VECTOR_BASE + 14),
    (irq_47_handler, x86::IRQ_VECTOR_BASE + 15),
);

#[cfg(target_os = "none")]
const _EXTENDED_IRQ_ROUTE_COUNT_ASSERT: [(); metadata::EXTENDED_IRQ_COUNT as usize] =
    [(); EXTENDED_IRQ_ROUTES.len()];
