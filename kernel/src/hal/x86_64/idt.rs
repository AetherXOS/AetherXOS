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
            .set_stack_index(crate::hal::gdt::DOUBLE_FAULT_IST_INDEX);
    }
    idt.page_fault.set_handler_fn(page_fault_handler);
    idt.general_protection_fault.set_handler_fn(gpf_handler);

    // Timer Interrupt (IRQ 0 = Vector 32)
    idt[x86::IRQ_TIMER as usize].set_handler_fn(timer_interrupt_handler);
    if crate::generated_consts::CORE_ENABLE_EXTENDED_IRQ_VECTORS {
        for vector in 1..=15 {
             idt[(x86::IRQ_VECTOR_BASE + vector) as usize].set_handler_fn(get_irq_handler(vector));
        }
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
fn handle_common_exception(
    name: &str,
    vector: u8,
    stack_frame: InterruptStackFrame,
    error_code: Option<u64>,
    fault_addr: Option<u64>,
) {
    if crate::config::KernelConfig::is_advanced_debug_enabled() {
        let bytes = unsafe {
            core::slice::from_raw_parts(
                (&stack_frame as *const InterruptStackFrame).cast::<u8>(),
                core::mem::size_of::<InterruptStackFrame>(),
            )
        };
        crate::hal::serial::write_dump_bytes(name, bytes);
    }

    crate::kernel::debug_trace::record_register_snapshot(
        name,
        stack_frame.instruction_pointer.as_u64(),
        stack_frame.stack_pointer.as_u64(),
        fault_addr.unwrap_or(error_code.unwrap_or(0)),
        stack_frame.cpu_flags,
    );

    #[cfg(feature = "dispatcher")]
    {
        use crate::interfaces::Dispatcher;
        if let Some(d) = unsafe { &*(&raw const DISPATCHER) } {
            d.dispatch(vector);
            return;
        }
    }

    crate::klog_error!(
        "EXCEPTION: {} (vector {}) at {:#x} code: {:#x?} frame: {:#?}",
        name,
        vector,
        stack_frame.instruction_pointer.as_u64(),
        error_code,
        stack_frame
    );
    crate::kernel::fatal_halt(name);
}

#[cfg(target_os = "none")]
extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    handle_common_exception("breakpoint", x86::EXCEPTION_BREAKPOINT, stack_frame, None, None);
}

#[cfg(target_os = "none")]
extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    crate::klog_error!("FATAL EXCEPTION: DOUBLE FAULT {:#?}", stack_frame);
    handle_common_exception(
        "double_fault",
        x86::EXCEPTION_DOUBLE_FAULT,
        stack_frame,
        Some(error_code),
        None,
    );
    unreachable!()
}

#[cfg(target_os = "none")]
extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use crate::interfaces::cpu::CpuRegisters;
    let addr = crate::hal::cpu::X86CpuRegisters::read_page_fault_addr();
    handle_common_exception(
        "page_fault",
        x86::EXCEPTION_PAGE_FAULT,
        stack_frame,
        Some(error_code.bits()),
        Some(addr),
    );
}

#[cfg(target_os = "none")]
extern "x86-interrupt" fn gpf_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    handle_common_exception(
        "gpf",
        x86::EXCEPTION_GPF,
        stack_frame,
        Some(error_code),
        None,
    );
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
        crate::hal::apic::eoi();
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
fn get_irq_handler(vector_offset: u8) -> extern "x86-interrupt" fn(InterruptStackFrame) {
    match vector_offset {
        1 => irq_33_handler,
        2 => irq_34_handler,
        3 => irq_35_handler,
        4 => irq_36_handler,
        5 => irq_37_handler,
        6 => irq_38_handler,
        7 => irq_39_handler,
        8 => irq_40_handler,
        9 => irq_41_handler,
        10 => irq_42_handler,
        11 => irq_43_handler,
        12 => irq_44_handler,
        13 => irq_45_handler,
        14 => irq_46_handler,
        15 => irq_47_handler,
        _ => panic!("unsupported irq offset"),
    }
}

#[cfg(target_os = "none")]
use crate::hal::smp::tlb_shootdown_handler;
