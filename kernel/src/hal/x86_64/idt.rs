#[cfg(target_os = "none")]
use crate::kernel::syscalls::syscalls_consts::x86;
#[cfg(target_os = "none")]
use spin::Lazy;
#[cfg(target_os = "none")]
use x86_64::structures::idt::InterruptDescriptorTable;

#[path = "idt/metadata.rs"]
mod metadata;
#[path = "idt/extended_handlers.rs"]
mod extended_handlers;
#[path = "idt/dispatch.rs"]
mod dispatch;
#[path = "idt/exception_handlers.rs"]
mod exception_handlers;

/// IDT (Interrupt Descriptor Table).
/// Routes hardware interrupts and CPU exceptions to handlers.

pub use dispatch::init_dispatcher;
#[cfg(target_os = "none")]
pub use dispatch::{irq_dispatch_metrics, IrqDispatchMetrics};

#[cfg(target_os = "none")]
static IDT: Lazy<InterruptDescriptorTable> = Lazy::new(|| {
    let mut idt = InterruptDescriptorTable::new();
    idt.breakpoint
        .set_handler_fn(exception_handlers::breakpoint_handler);
    unsafe {
        idt.double_fault
            .set_handler_fn(exception_handlers::double_fault_handler)
            .set_stack_index(crate::hal::gdt::DOUBLE_FAULT_IST_INDEX);
    }
    idt.page_fault
        .set_handler_fn(exception_handlers::page_fault_handler);
    idt.general_protection_fault
        .set_handler_fn(exception_handlers::gpf_handler);

    // Timer Interrupt (IRQ 0 = Vector 32)
    idt[x86::IRQ_TIMER as usize].set_handler_fn(exception_handlers::timer_interrupt_handler);
    if crate::generated_consts::CORE_ENABLE_EXTENDED_IRQ_VECTORS {
        debug_assert_eq!(
            extended_handlers::EXTENDED_IRQ_ROUTES.len(),
            metadata::EXTENDED_IRQ_COUNT as usize
        );
        crate::hal::common::irq_registration::register_irq_routes(
            &extended_handlers::EXTENDED_IRQ_ROUTES,
            |vector, handler| {
                idt[vector as usize].set_handler_fn(handler);
            },
        );
    }
    idt[x86::IRQ_TLB_SHOOTDOWN as usize].set_handler_fn(tlb_shootdown_handler);

    idt
});

#[cfg(target_os = "none")]
pub fn init() {
    IDT.load();
}

#[cfg(not(target_os = "none"))]
pub fn init() {}

#[cfg(target_os = "none")]
use crate::hal::smp::tlb_shootdown_handler;
