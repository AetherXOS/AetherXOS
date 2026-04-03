pub use crate::hal::common::boot::{acpi_rsdp_addr, dtb_addr, framebuffer, hhdm_offset, mem_map};
use crate::interfaces::{HardwareAbstraction, SerialDevice};
#[cfg(target_os = "none")]
use core::arch::naked_asm;

pub mod acpi;
pub mod cpu;
pub mod gdt;
pub mod idt;
pub mod input;
pub mod platform;
#[cfg(all(feature = "ring_protection", target_os = "none"))]
pub mod syscalls;
pub mod virt;
#[cfg(all(feature = "ring_protection", not(target_os = "none")))]
pub mod syscalls {
    pub fn init(_selectors: &super::gdt::Selectors) {}
}

pub mod apic;
pub mod iommu;
pub mod pci;
pub mod pic;
pub mod port;
pub mod serial;
pub mod smp;
pub mod paging;

use core::mem::MaybeUninit;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};
use crate::kernel::bit_utils as bits;

pub struct HAL;

static BSP_CPU_LOCAL_READY: AtomicBool = AtomicBool::new(false);
struct StaticCell<T>(UnsafeCell<MaybeUninit<T>>);

unsafe impl<T> Sync for StaticCell<T> {}

impl<T> StaticCell<T> {
    const fn uninit() -> Self {
        Self(UnsafeCell::new(MaybeUninit::uninit()))
    }

    unsafe fn write(&self, value: T) {
        unsafe { (*self.0.get()).write(value) };
    }

    unsafe fn as_ptr(&self) -> *const T {
        unsafe { (*self.0.get()).as_ptr() }
    }
}

static BSP_CPU_LOCAL: StaticCell<crate::kernel::cpu_local::CpuLocal> = StaticCell::uninit();
#[cfg(feature = "ring_protection")]
struct StaticBytes<const N: usize>(UnsafeCell<[u8; N]>);

#[cfg(feature = "ring_protection")]
unsafe impl<const N: usize> Sync for StaticBytes<N> {}

#[cfg(feature = "ring_protection")]
impl<const N: usize> StaticBytes<N> {
    const fn zeroed() -> Self {
        Self(UnsafeCell::new([0u8; N]))
    }

    fn base_addr(&self) -> usize {
        self.0.get() as *const u8 as usize
    }
}

#[cfg(feature = "ring_protection")]
static BSP_KERNEL_STACK: StaticBytes<{ crate::generated_consts::STACK_SIZE_PAGES * 4096 }> =
    StaticBytes::zeroed();

#[inline(never)]
fn early_call_checkpoint() {
    serial::write_raw("[EARLY SERIAL] x86_64 early call checkpoint entered\n");
}

#[cfg(feature = "ring_protection")]
fn bootstrap_bsp_kernel_stack_top() -> usize {
    let top = BSP_KERNEL_STACK.base_addr() + crate::generated_consts::STACK_SIZE_PAGES * 4096;
    top & !0xF
}

unsafe fn bootstrap_bsp_cpu_local() -> &'static crate::kernel::cpu_local::CpuLocal {
    use crate::interfaces::task::CpuId;
    use crate::kernel::cpu_local::CpuLocal;

    if !BSP_CPU_LOCAL_READY.load(Ordering::Acquire) {
        serial::write_raw("[EARLY SERIAL] x86_64 bsp cpu local bootstrap begin\n");
        serial::write_raw("[EARLY SERIAL] x86_64 bsp scheduler create begin\n");
        early_call_checkpoint();
        serial::write_raw("[EARLY SERIAL] x86_64 early call checkpoint returned\n");
        let scheduler = crate::modules::selector::bootstrap_active_scheduler();
        serial::write_raw("[EARLY SERIAL] x86_64 bsp scheduler create returned\n");
        serial::write_raw("[EARLY SERIAL] x86_64 bsp scheduler mutex begin\n");
        let scheduler = crate::kernel::sync::IrqSafeMutex::new(scheduler);
        serial::write_raw("[EARLY SERIAL] x86_64 bsp scheduler mutex returned\n");
        serial::write_raw("[EARLY SERIAL] x86_64 bsp cpu local write begin\n");
        unsafe {
            BSP_CPU_LOCAL.write(CpuLocal {
                cpu_id: CpuId(0),
                #[cfg(feature = "ring_protection")]
                scratch: 0,
                #[cfg(feature = "ring_protection")]
                kernel_stack_top: core::sync::atomic::AtomicUsize::new(bootstrap_bsp_kernel_stack_top()),
                current_task: core::sync::atomic::AtomicUsize::new(0),
                heartbeat_tick: core::sync::atomic::AtomicU64::new(0),
                idle_stack_pointer: core::sync::atomic::AtomicUsize::new(0),
                scheduler,
            });
        }
        serial::write_raw("[EARLY SERIAL] x86_64 bsp cpu local write returned\n");
        BSP_CPU_LOCAL_READY.store(true, Ordering::Release);
        serial::write_raw("[EARLY SERIAL] x86_64 bsp cpu local bootstrap returned\n");
    }

    unsafe { &*BSP_CPU_LOCAL.as_ptr() }
}

impl HAL {
    pub fn early_init() {
        // 1. Initialize Serial Port for logging
        serial::SERIAL1.lock().init();
        serial::write_raw("[EARLY SERIAL] x86_64 serial initialized\n");

        // 2. Initialize BSP (Bootstrap Processor) GDT & TSS
        // Must leak to keep alive forever
        serial::write_raw("[EARLY SERIAL] x86_64 bootstrap gdt request begin\n");
        let bsp_gdt = unsafe { gdt::bootstrap_gdt_tss() };
        serial::write_raw("[EARLY SERIAL] x86_64 bootstrap gdt request returned\n");
        let selectors = bsp_gdt.selectors;
        serial::write_raw("[EARLY SERIAL] x86_64 gdt load call begin\n");
        unsafe {
            bsp_gdt.load();
        }
        serial::write_raw("[EARLY SERIAL] x86_64 gdt loaded\n");

        // 3. Initialize IDT
        idt::init();
        serial::write_raw("[EARLY SERIAL] x86_64 idt initialized\n");

        // 4. Initialize Local APIC (BSP)
        unsafe {
            pic::Pic::disable();
            apic::init_local_apic();
        }
        serial::write_raw("[EARLY SERIAL] x86_64 local apic initialized\n");

        // 5. Initialize BSP (CPU 0) CpuLocal structure
        serial::write_raw("[EARLY SERIAL] x86_64 bsp cpu local request begin\n");
        let bsp_local = unsafe { bootstrap_bsp_cpu_local() };
        serial::write_raw("[EARLY SERIAL] x86_64 bsp cpu local request returned\n");

        serial::write_raw("[EARLY SERIAL] x86_64 cpu local init begin\n");
        unsafe {
            bsp_local.init();
        }
        serial::write_raw("[EARLY SERIAL] x86_64 cpu local initialized\n");

        serial::write_raw("[EARLY SERIAL] x86_64 bsp register phase deferred to init_smp due to Heap dependencies.\n");

        // 6. Initialize Syscalls (Ring 3 -> 0) after CpuLocal/GS/kernel stack.
        #[cfg(feature = "ring_protection")]
        syscalls::init(&selectors);
        serial::write_raw("[EARLY SERIAL] x86_64 syscalls initialized\n");
        early_call_checkpoint();
        serial::write_raw("[EARLY SERIAL] x86_64 post-syscall checkpoint returned\n");
    }

    pub fn init_interrupts() {
        // IDT is already initialized in early_init
    }

    pub fn init_timer() {
        // Timer is often part of APIC initialization or separate pit/hpet
    }

    pub fn init_smp() {
        serial::write_raw("[EARLY SERIAL] x86_64 late bsp registration initialized\n");
        let bsp_local = unsafe { bootstrap_bsp_cpu_local() };
        smp::register_cpu(bsp_local);
        serial::write_raw("[EARLY SERIAL] x86_64 late bsp registered successfully\n");
        smp::init();
    }

    pub unsafe fn context_switch(prev: *mut usize, next: usize) {
        unsafe {
            context_switch(prev, next);
        }
    }

    pub fn read_per_cpu_base() -> usize {
        use crate::interfaces::cpu::CpuRegisters;
        cpu::X86CpuRegisters::read_per_cpu_base() as usize
    }
}

impl HardwareAbstraction for HAL {
    fn enable_interrupts() {
        unsafe {
            core::arch::asm!("sti", options(nomem, nostack));
        }
    }

    fn disable_interrupts() {
        unsafe {
            core::arch::asm!("cli", options(nomem, nostack));
        }
    }

    #[inline(always)]
    fn irq_save() -> usize {
        let flags: usize;
        unsafe {
            core::arch::asm!(
                "pushf",
                "pop {}",
                "cli",
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
                "push {}",
                "popf",
                in(reg) flags,
                options(nomem, nostack)
            );
        }
    }

    fn halt() {
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack));
        }
    }

    fn early_init() {
        // x86_64 early boot involves GDT and basic serial
        unsafe {
            gdt::bootstrap_gdt_tss().load();
            serial::init();
        }
    }

    fn init_interrupts() {
        idt::init();
    }

    fn init_timer() {
        apic::init();
    }

    fn init_smp() {
        Self::init_smp();
    }

    fn init_cpu_local(ptr: usize) {
        use x86_64::registers::model_specific::GsBase;
        use x86_64::VirtAddr;
        GsBase::write(VirtAddr::new(ptr as u64));
    }

    fn set_performance_profile(profile: crate::interfaces::PerformanceProfile) {
        use crate::interfaces::PerformanceProfile;
        let ratio = match profile {
            PerformanceProfile::HighPerformance => bits::perf::RATIO_HIGH,
            PerformanceProfile::Balanced        => bits::perf::RATIO_BALANCED,
            PerformanceProfile::PowerSaving     => bits::perf::RATIO_POWERSAVE,
        };
        unsafe {
            cpu::write_msr(bits::perf::IA32_PERF_CTL, (ratio as u64) << 8);
        }
    }

    fn serial_write_raw(s: &str) {
        serial::write_raw(s);
    }
}

/// Context Switch Logic for x86_64
#[cfg(target_os = "none")]
#[unsafe(naked)]
pub unsafe extern "C" fn context_switch(current_stack: *mut usize, next_stack: usize) {
    // fast call abi: rdi = current_stack, rsi = next_stack
    naked_asm!(
        // Save callee-saved registers
        "push rbp",
        "push rbx",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        // Switch stacks
        "mov [rdi], rsp", // Save old SP
        "mov rsp, rsi",   // Load new SP
        // Restore callee-saved registers
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbx",
        "pop rbp",
        "ret"
    );
}

#[cfg(not(target_os = "none"))]
pub unsafe extern "C" fn context_switch(_current_stack: *mut usize, _next_stack: usize) {
    panic!("x86_64 context_switch is only available on bare-metal targets");
}
