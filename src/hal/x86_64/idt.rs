#[cfg(target_os = "none")]
use crate::kernel::syscalls::syscalls_consts::x86;
#[cfg(feature = "dispatcher")]
use crate::modules::dispatcher::selector::ActiveDispatcher;
#[cfg(target_os = "none")]
use spin::Lazy;
#[cfg(target_os = "none")]
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

/// IDT (Interrupt Descriptor Table).
/// Routes hardware interrupts and CPU exceptions to handlers.

#[cfg(feature = "dispatcher")]
static mut DISPATCHER: Option<ActiveDispatcher> = None;

// Initialize the dispatcher globally (called from main)
#[cfg(feature = "dispatcher")]
pub fn init_dispatcher(d: ActiveDispatcher) {
    unsafe {
        DISPATCHER = Some(d);
    }
}

#[cfg(not(feature = "dispatcher"))]
pub fn init_dispatcher(_d: ()) {}

#[cfg(target_os = "none")]
static IDT: Lazy<InterruptDescriptorTable> = Lazy::new(|| {
    let mut idt = InterruptDescriptorTable::new();
    idt.breakpoint.set_handler_fn(breakpoint_handler);
    unsafe {
        idt.double_fault
            .set_handler_fn(double_fault_handler)
            .set_stack_index(crate::hal::x86_64::gdt::DOUBLE_FAULT_IST_INDEX);
    }
    idt.page_fault.set_handler_fn(page_fault_handler);

    idt.general_protection_fault.set_handler_fn(gpf_handler);

    // Timer Interrupt (IRQ 0 = Vector 32)
    idt[x86::IRQ_TIMER as usize].set_handler_fn(timer_interrupt_handler);
    if crate::generated_consts::CORE_ENABLE_EXTENDED_IRQ_VECTORS {
        idt[(x86::IRQ_VECTOR_BASE + 1) as usize].set_handler_fn(irq_33_handler);
        idt[(x86::IRQ_VECTOR_BASE + 2) as usize].set_handler_fn(irq_34_handler);
        idt[(x86::IRQ_VECTOR_BASE + 3) as usize].set_handler_fn(irq_35_handler);
        idt[(x86::IRQ_VECTOR_BASE + 4) as usize].set_handler_fn(irq_36_handler);
        idt[(x86::IRQ_VECTOR_BASE + 5) as usize].set_handler_fn(irq_37_handler);
        idt[(x86::IRQ_VECTOR_BASE + 6) as usize].set_handler_fn(irq_38_handler);
        idt[(x86::IRQ_VECTOR_BASE + 7) as usize].set_handler_fn(irq_39_handler);
        idt[(x86::IRQ_VECTOR_BASE + 8) as usize].set_handler_fn(irq_40_handler);
        idt[(x86::IRQ_VECTOR_BASE + 9) as usize].set_handler_fn(irq_41_handler);
        idt[(x86::IRQ_VECTOR_BASE + 10) as usize].set_handler_fn(irq_42_handler);
        idt[(x86::IRQ_VECTOR_BASE + 11) as usize].set_handler_fn(irq_43_handler);
        idt[(x86::IRQ_VECTOR_BASE + 12) as usize].set_handler_fn(irq_44_handler);
        idt[(x86::IRQ_VECTOR_BASE + 13) as usize].set_handler_fn(irq_45_handler);
        idt[(x86::IRQ_VECTOR_BASE + 14) as usize].set_handler_fn(irq_46_handler);
        idt[(x86::IRQ_VECTOR_BASE + 15) as usize].set_handler_fn(irq_47_handler);
    }
    idt[253].set_handler_fn(tlb_shootdown_handler);

    idt
});

#[cfg(target_os = "none")]
pub fn init() {
    IDT.load();
}

#[cfg(not(target_os = "none"))]
pub fn init() {}

#[cfg(target_os = "none")]
extern "x86-interrupt" fn breakpoint_handler(_stack_frame: InterruptStackFrame) {
    #[cfg(feature = "dispatcher")]
    {
        use crate::interfaces::Dispatcher;
        if let Some(d) = unsafe { &*(&raw const DISPATCHER) } {
            d.dispatch(x86::EXCEPTION_BREAKPOINT); // INT 3
        }
    }
}

#[cfg(target_os = "none")]
extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    if crate::config::KernelConfig::is_advanced_debug_enabled() {
        let bytes = unsafe {
            core::slice::from_raw_parts(
                (&stack_frame as *const InterruptStackFrame).cast::<u8>(),
                core::mem::size_of::<InterruptStackFrame>(),
            )
        };
        crate::hal::x86_64::serial::write_dump_bytes("x86.double_fault.frame", bytes);
    }
    crate::kernel::debug_trace::record_register_snapshot(
        "x86.double_fault",
        stack_frame.instruction_pointer.as_u64(),
        stack_frame.stack_pointer.as_u64(),
        stack_frame.code_segment as u64,
        stack_frame.cpu_flags,
    );
    crate::klog_error!("EXCEPTION: DOUBLE FAULT {:#?}", stack_frame);
    crate::kernel::fatal_halt("double_fault");
}

#[cfg(target_os = "none")]
extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use crate::interfaces::cpu::CpuRegisters;

    #[cfg(feature = "dispatcher")]
    {
        use crate::interfaces::Dispatcher;
        // Dispatch to registered handlers (e.g. VMM)
        if let Some(d) = unsafe { &*(&raw const DISPATCHER) } {
            d.dispatch(x86::EXCEPTION_PAGE_FAULT); // #PF
            return;
        }
    }

    // Fallback panic if no dispatcher active
    let addr = crate::hal::cpu::X86CpuRegisters::read_page_fault_addr();
    if crate::config::KernelConfig::is_advanced_debug_enabled() {
        let bytes = unsafe {
            core::slice::from_raw_parts(
                (&stack_frame as *const InterruptStackFrame).cast::<u8>(),
                core::mem::size_of::<InterruptStackFrame>(),
            )
        };
        crate::hal::x86_64::serial::write_dump_bytes("x86.page_fault.frame", bytes);
    }
    crate::kernel::debug_trace::record_register_snapshot(
        "x86.page_fault",
        stack_frame.instruction_pointer.as_u64(),
        stack_frame.stack_pointer.as_u64(),
        addr,
        error_code.bits(),
    );
    crate::klog_error!(
        "EXCEPTION: PAGE FAULT at {:?} code: {:?} {:#?}",
        addr,
        error_code,
        stack_frame
    );
    crate::kernel::fatal_halt("page_fault");
}

#[cfg(target_os = "none")]
extern "x86-interrupt" fn gpf_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    #[cfg(feature = "dispatcher")]
    {
        use crate::interfaces::Dispatcher;
        if let Some(d) = unsafe { &*(&raw const DISPATCHER) } {
            d.dispatch(13); // #GP
            return;
        }
    }

    if crate::config::KernelConfig::is_advanced_debug_enabled() {
        let bytes = unsafe {
            core::slice::from_raw_parts(
                (&stack_frame as *const InterruptStackFrame).cast::<u8>(),
                core::mem::size_of::<InterruptStackFrame>(),
            )
        };
        crate::hal::x86_64::serial::write_dump_bytes("x86.gpf.frame", bytes);
    }
    crate::kernel::debug_trace::record_register_snapshot(
        "x86.gpf",
        stack_frame.instruction_pointer.as_u64(),
        stack_frame.stack_pointer.as_u64(),
        error_code,
        stack_frame.cpu_flags,
    );
    crate::klog_error!("EXCEPTION: GPF code: {:#x} {:#?}", error_code, stack_frame);
    crate::kernel::fatal_halt("general_protection_fault");
}

#[cfg(target_os = "none")]
extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    dispatch_irq_vector(x86::IRQ_TIMER);
}

#[cfg(target_os = "none")]
#[inline(always)]
fn dispatch_irq_vector(vector: u8) {
    let allow_dispatch = crate::kernel::interrupt_guard::on_irq(vector);
    let is_timer = vector == x86::IRQ_TIMER;

    if crate::generated_consts::CORE_ENABLE_IRQ_TRACE && !is_timer {
        crate::klog_trace!("IRQ vector {} dispatched", vector);
    }

    if allow_dispatch {
        #[cfg(feature = "dispatcher")]
        {
            use crate::interfaces::Dispatcher;
            if let Some(d) = unsafe { &*(&raw const DISPATCHER) } {
                d.dispatch(vector);
            }
        }
    } else if crate::generated_consts::CORE_ENABLE_IRQ_TRACE {
        crate::klog_trace!("IRQ vector {} dropped by storm protection", vector);
    }

    unsafe {
        crate::hal::x86_64::apic::eoi();
    }
}

#[cfg(target_os = "none")]
macro_rules! define_irq_handler {
    ($name:ident, $vector:expr) => {
        extern "x86-interrupt" fn $name(_stack_frame: InterruptStackFrame) {
            dispatch_irq_vector($vector);
        }
    };
}

#[cfg(target_os = "none")]
define_irq_handler!(irq_33_handler, x86::IRQ_VECTOR_BASE + 1);
#[cfg(target_os = "none")]
define_irq_handler!(irq_34_handler, x86::IRQ_VECTOR_BASE + 2);
#[cfg(target_os = "none")]
define_irq_handler!(irq_35_handler, x86::IRQ_VECTOR_BASE + 3);
#[cfg(target_os = "none")]
define_irq_handler!(irq_36_handler, x86::IRQ_VECTOR_BASE + 4);
#[cfg(target_os = "none")]
define_irq_handler!(irq_37_handler, x86::IRQ_VECTOR_BASE + 5);
#[cfg(target_os = "none")]
define_irq_handler!(irq_38_handler, x86::IRQ_VECTOR_BASE + 6);
#[cfg(target_os = "none")]
define_irq_handler!(irq_39_handler, x86::IRQ_VECTOR_BASE + 7);
#[cfg(target_os = "none")]
define_irq_handler!(irq_40_handler, x86::IRQ_VECTOR_BASE + 8);
#[cfg(target_os = "none")]
define_irq_handler!(irq_41_handler, x86::IRQ_VECTOR_BASE + 9);
#[cfg(target_os = "none")]
define_irq_handler!(irq_42_handler, x86::IRQ_VECTOR_BASE + 10);
#[cfg(target_os = "none")]
define_irq_handler!(irq_43_handler, x86::IRQ_VECTOR_BASE + 11);
#[cfg(target_os = "none")]
define_irq_handler!(irq_44_handler, x86::IRQ_VECTOR_BASE + 12);
#[cfg(target_os = "none")]
define_irq_handler!(irq_45_handler, x86::IRQ_VECTOR_BASE + 13);
#[cfg(target_os = "none")]
define_irq_handler!(irq_46_handler, x86::IRQ_VECTOR_BASE + 14);
#[cfg(target_os = "none")]
define_irq_handler!(irq_47_handler, x86::IRQ_VECTOR_BASE + 15);

#[cfg(target_os = "none")]
use crate::hal::x86_64::smp::tlb_shootdown_handler;
