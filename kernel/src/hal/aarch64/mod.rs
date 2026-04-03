pub use crate::hal::common::boot::{acpi_rsdp_addr, dtb_addr, framebuffer, hhdm_offset, mem_map};
use crate::interfaces::hardware::InterruptController;
use crate::interfaces::HardwareAbstraction;
use core::arch::naked_asm;

pub mod acpi;
pub mod cpu;
pub mod exception;
pub mod gic;
pub mod paging;
pub mod pci;
pub mod pl011;
pub mod platform;
pub mod serial;
pub mod smp;
pub mod timer;
pub mod virt;

pub struct HAL;

impl HAL {
    pub fn early_init() {
        use crate::interfaces::SerialDevice;
        serial::SERIAL1.lock().init();

        exception::init();

        use crate::interfaces::task::CpuId;
        use crate::kernel::cpu_local::CpuLocal;
        use crate::kernel::sync::IrqSafeMutex;
        use alloc::boxed::Box;

        let bsp_local = Box::leak(Box::new(CpuLocal {
            cpu_id: CpuId(0),
            #[cfg(feature = "ring_protection")]
            scratch: 0,
            #[cfg(feature = "ring_protection")]
            kernel_stack_top: core::sync::atomic::AtomicUsize::new(smp::allocate_kernel_stack_top()),
            current_task: core::sync::atomic::AtomicUsize::new(0),
            heartbeat_tick: core::sync::atomic::AtomicU64::new(0),
            idle_stack_pointer: core::sync::atomic::AtomicUsize::new(0),
            scheduler: IrqSafeMutex::new(
                crate::modules::selector::ActiveScheduler::new(),
            ),
        }));

        unsafe {
            bsp_local.init();
        }

        smp::register_cpu(bsp_local);
    }

    pub fn init_interrupts() {
        let mut gic = gic::GIC.lock();
        unsafe {
            gic.init();
        }
        platform::irq::enable_platform_irq_lines(&mut *gic);
    }

    pub fn init_timer() {
        timer::GenericTimer::init();
    }

    /// Bring-up hook for secondary cores on AArch64.
    pub fn init_smp() {
        smp::init();
    }

    pub unsafe fn context_switch(prev: *mut usize, next: usize) {
        unsafe {
            context_switch(prev, next);
        }
    }

    pub fn read_per_cpu_base() -> usize {
        let ptr: u64;
        unsafe {
            core::arch::asm!("mrs {}, tpidr_el1", out(reg) ptr);
        }
        ptr as usize
    }
}

impl HardwareAbstraction for HAL {
    fn enable_interrupts() {
        unsafe {
            core::arch::asm!("msr daifclr, #2", options(nomem, nostack));
        }
    }

    fn disable_interrupts() {
        unsafe {
            core::arch::asm!("msr daifset, #2", options(nomem, nostack));
        }
    }

    #[inline(always)]
    fn irq_save() -> usize {
        let flags: usize;
        unsafe {
            core::arch::asm!(
                "mrs {}, daif",
                "msr daifset, #2",
                out(reg) flags,
                options(nomem, nostack)
            );
        }
        flags
    }

    #[inline(always)]
    fn irq_restore(flags: usize) {
        unsafe {
            core::arch::asm!(
                "msr daif, {}",
                in(reg) flags,
                options(nomem, nostack)
            );
        }
    }

    fn halt() {
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack));
        }
    }

    fn early_init() {
        HAL::early_init();
    }

    fn init_interrupts() {
        HAL::init_interrupts();
    }

    fn init_timer() {
        HAL::init_timer();
    }

    fn init_smp() {
        HAL::init_smp();
    }

    fn init_cpu_local(ptr: usize) {
        unsafe {
            core::arch::asm!("msr tpidr_el1, {}", in(reg) ptr);
        }
    }

    fn set_performance_profile(_profile: crate::interfaces::PerformanceProfile) {
        // ARM frequency scaling is SoC-specific.
    }

    fn serial_write_raw(s: &str) {
        serial::write_raw(s);
    }
}

/// Context Switch Logic for AArch64
#[cfg(target_os = "none")]
#[unsafe(naked)]
pub unsafe extern "C" fn context_switch(current_stack: *mut usize, next_stack: usize) {
    // AArch64 procedure call standard (AAPCS64):
    // x0 = current_stack, x1 = next_stack
    naked_asm!(
        // Save SIMD/FPU callee-saved registers (q8-q15)
        "stp q8, q9, [sp, #-32]!",
        "stp q10, q11, [sp, #-32]!",
        "stp q12, q13, [sp, #-32]!",
        "stp q14, q15, [sp, #-32]!",
        // Save callee-saved registers (x19-x29, lr)
        "stp x19, x20, [sp, #-16]!",
        "stp x21, x22, [sp, #-16]!",
        "stp x23, x24, [sp, #-16]!",
        "stp x25, x26, [sp, #-16]!",
        "stp x27, x28, [sp, #-16]!",
        "stp x29, x30, [sp, #-16]!", // FP & LR
        // Switch stacks
        "mov x9, sp",
        "str x9, [x0]", // *current_stack = sp
        "mov sp, x1",   // sp = next_stack
        // Restore callee-saved registers
        "ldp x29, x30, [sp], #16",
        "ldp x27, x28, [sp], #16",
        "ldp x25, x26, [sp], #16",
        "ldp x23, x24, [sp], #16",
        "ldp x21, x22, [sp], #16",
        "ldp x19, x20, [sp], #16",
        // Restore SIMD/FPU callee-saved registers
        "ldp q14, q15, [sp], #32",
        "ldp q12, q13, [sp], #32",
        "ldp q10, q11, [sp], #32",
        "ldp q8, q9, [sp], #32",
        "ret"
    );
}

#[cfg(not(target_os = "none"))]
pub unsafe extern "C" fn context_switch(_current_stack: *mut usize, _next_stack: usize) {
    panic!("aarch64 context_switch is only available on bare-metal targets");
}
