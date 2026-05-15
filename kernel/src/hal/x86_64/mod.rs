pub use crate::hal::common::boot::{acpi_rsdp_addr, dtb_addr, framebuffer, hhdm_offset, mem_map};
use crate::core::log;
use alloc::format;
use crate::interfaces::{HardwareAbstraction, SerialDevice};
use crate::interfaces::hardware::{InterruptController, MemoryManager};
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

/// Boot-time call stack verification checkpoint.
///
/// Marked `#[inline(never)]` so it appears in the call graph and
/// stack traces during early-boot debugging.  **Only compiled in
/// debug builds** — release builds elide this entirely.
#[cfg(debug_assertions)]
#[inline(never)]
fn early_call_checkpoint() {
    serial::write_raw("[EARLY SERIAL] x86_64 early call checkpoint entered\n");
}

/// No-op stub for release builds — zero overhead.
#[cfg(not(debug_assertions))]
#[inline(always)]
fn early_call_checkpoint() {}

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
                current_process_id: core::sync::atomic::AtomicUsize::new(0),
                is_user_mode: core::sync::atomic::AtomicBool::new(false),
                heartbeat_tick: core::sync::atomic::AtomicU64::new(0),
                idle_stack_pointer: core::sync::atomic::AtomicUsize::new(0),
                scheduler,
                kernel_mode_depth: core::sync::atomic::AtomicU32::new(1),
            });
        }
        serial::write_raw("[EARLY SERIAL] x86_64 bsp cpu local write returned\n");
        BSP_CPU_LOCAL_READY.store(true, Ordering::Release);
        serial::write_raw("[EARLY SERIAL] x86_64 bsp cpu local bootstrap returned\n");
    }

    unsafe { &*BSP_CPU_LOCAL.as_ptr() }
}

impl HAL {
    /// Primary x86_64 early-boot initialisation.
    ///
    /// Sequence (must be kept in order — each stage depends on the previous):
    /// 1. Serial port  — enables debug output before anything else
    /// 2. GDT/TSS      — required for safe kernel stack and privilege levels
    /// 3. IDT          — required before any interrupt/exception can fire
    /// 4. APIC         — replaces legacy PIC, needed for timer + IPI
    /// 5. CpuLocal     — per-CPU data (scheduler, current task, etc.)
    /// 6. SYSCALL/RET  — Ring 3 → Ring 0 entry point (requires GDT + CpuLocal)
    pub fn early_init() {
        // ── 1. Serial port ────────────────────────────────────────────────────
        serial::SERIAL1.lock().init();
        // Always emit at least one marker so the user knows serial is live.
        serial::write_raw("[BOOT] x86_64 serial ready\n");

        // ── 2. GDT / TSS ──────────────────────────────────────────────────────
        #[cfg(debug_assertions)]
        serial::write_raw("[BOOT] gdt init begin\n");
        let bsp_gdt = unsafe { gdt::bootstrap_gdt_tss() };
        let selectors = bsp_gdt.selectors;
        unsafe { bsp_gdt.load(); }
        #[cfg(debug_assertions)]
        serial::write_raw("[BOOT] gdt loaded\n");

        // ── 3. IDT ────────────────────────────────────────────────────────────
        idt::init();
        #[cfg(debug_assertions)]
        serial::write_raw("[BOOT] idt ready\n");

        // ── 4. APIC (disable legacy PIC first) ────────────────────────────────
        unsafe {
            pic::Pic::disable();
            apic::init_local_apic();
        }
        #[cfg(debug_assertions)]
        serial::write_raw("[BOOT] apic ready\n");

        // ── 5. BSP CpuLocal ───────────────────────────────────────────────────
        let bsp_local = unsafe { bootstrap_bsp_cpu_local() };
        unsafe { bsp_local.init(); }
        #[cfg(debug_assertions)]
        serial::write_raw("[BOOT] cpu_local ready\n");

        // SMP registration is deferred to `init_smp()` because it requires
        // the heap allocator which isn't available yet at this point.

        // ── 6. SYSCALL/SYSRET entry point ────────────────────────────────────
        #[cfg(feature = "ring_protection")]
        syscalls::init(&selectors);
        // Call-graph checkpoint: validates linker resolved this call correctly.
        early_call_checkpoint();
        serial::write_raw("[BOOT] x86_64 early_init complete\n");
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

    pub fn get_time_ns() -> u64 {
        #[cfg(target_os = "none")]
        {
            if let Some(hz) = cpu::tsc_frequency_hz() {
                // Potential overflow for very long runtimes, but safe for boot/standard ops
                // (TSC * 1e9) / HZ
                return cpu::rdtsc().wrapping_mul(1_000_000_000) / hz;
            }
        }
        crate::kernel::watchdog::global_tick() * crate::config::KernelConfig::time_slice()
    }

    #[inline(always)]
    pub fn irq_save() -> usize {
        <Self as HardwareAbstraction>::irq_save()
    }

    #[inline(always)]
    pub fn irq_restore(flags: usize) {
        <Self as HardwareAbstraction>::irq_restore(flags)
    }

    pub fn create_frame_allocator() -> paging::PageAllocWrapper {
        paging::PageAllocWrapper::new()
    }

    #[inline(always)]
    pub fn cpu_relax() {
        unsafe {
            core::arch::asm!("pause", options(nomem, nostack));
        }
    }
}

impl HardwareAbstraction for HAL {
    fn enable_interrupts() {
        #[cfg(target_os = "none")]
        unsafe {
            core::arch::asm!("sti", options(nomem, nostack));
        }
    }

    fn disable_interrupts() {
        #[cfg(target_os = "none")]
        unsafe {
            core::arch::asm!("cli", options(nomem, nostack));
        }
    }

    #[inline(always)]
    fn irq_save() -> usize {
        #[cfg(target_os = "none")]
        {
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
        #[cfg(not(target_os = "none"))]
        {
            0
        }
    }

    #[inline(always)]
    fn irq_restore(flags: usize) {
        #[cfg(target_os = "none")]
        unsafe {
            core::arch::asm!(
                "push {}",
                "popf",
                in(reg) flags,
                options(nomem, nostack)
            );
        }
        #[cfg(not(target_os = "none"))]
        {
            let _ = flags;
        }
    }

    fn halt() {
        #[cfg(target_os = "none")]
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack));
        }
    }

    fn early_init() {
        // x86_64 early boot involves GDT and basic serial
        #[cfg(target_os = "none")]
        unsafe {
            gdt::bootstrap_gdt_tss().load();
            serial::init();
        }
    }

    fn init_interrupts() {
        #[cfg(target_os = "none")]
        idt::init();
    }

    fn init_timer() {
        #[cfg(target_os = "none")]
        apic::init();
    }

    fn init_smp() {
        #[cfg(target_os = "none")]
        Self::init_smp();
    }

    fn init_cpu_local(ptr: usize) {
        #[cfg(target_os = "none")]
        {
            use x86_64::registers::model_specific::GsBase;
            use x86_64::VirtAddr;
            GsBase::write(VirtAddr::new(ptr as u64));
        }
        #[cfg(not(target_os = "none"))]
        {
            let _ = ptr;
        }
    }

    fn set_performance_profile(profile: crate::interfaces::PerformanceProfile) {
        #[cfg(target_os = "none")]
        {
            use crate::interfaces::PerformanceProfile;
            use crate::kernel::bit_utils::perf;
            let ratio = match profile {
                PerformanceProfile::HighPerformance => perf::RATIO_HIGH,
                PerformanceProfile::Balanced        => perf::RATIO_BALANCED,
                PerformanceProfile::PowerSaving     => perf::RATIO_POWERSAVE,
            };
            unsafe {
                cpu::write_msr(perf::IA32_PERF_CTL, (ratio as u64) << 8);
            }
        }
        #[cfg(not(target_os = "none"))]
        {
            let _ = profile;
        }
    }

    fn serial_write_raw(s: &str) {
        serial::write_raw(s);
    }

    fn panic_with_report(info: &core::panic::PanicInfo, report: &crate::kernel::CrashReport) -> ! {
        // Minimal allocation-free reporting
        log::error("KERNEL PANIC");

        // If advanced debug is enabled, dump additional diagnostics to serial
        if crate::config::KernelConfig::is_advanced_debug_enabled() {
            crate::kernel::dump_diagnostics("panic", report);
        }

        if let Some(location) = info.location() {
            let file = location.file();
            let line = location.line();
            // Use klog to ensure persistence in ring buffer
            log::error(&format!("Location: {}:{}", file, line));
        }

        log::error("Panic Count: 1");

        #[cfg(target_os = "none")]
        loop {
            unsafe { core::arch::asm!("hlt"); }
        }
        #[cfg(not(target_os = "none"))]
        panic!("kernel panic in host test");
    }

    fn fatal_halt(reason: &str) -> ! {
        log::error(&format!("FATAL HALT: {}", reason));

        if crate::config::KernelConfig::is_advanced_debug_enabled() {
            let report = crate::kernel::crash_report();
            serial::write_trace("fatal", reason);
            crate::kernel::dump_diagnostics("fatal", &report);
        }

        #[cfg(target_os = "none")]
        loop {
            unsafe { core::arch::asm!("hlt"); }
        }
        #[cfg(not(target_os = "none"))]
        panic!("fatal halt: {}", reason);
    }

    fn idle_once() {
        #[cfg(target_os = "none")]
        unsafe { core::arch::asm!("hlt"); }
    }

    fn get_time_ns() -> u64 {
        Self::get_time_ns()
    }

    fn interrupt_controller() -> &'static dyn InterruptController {
        &X86_INTERRUPT_CONTROLLER
    }

    fn memory_manager() -> &'static dyn MemoryManager {
        &X86_MEMORY_MANAGER
    }
}

// ── HAL Sub-component Implementations ────────────────────────────────────────

struct X86InterruptController;
struct X86MemoryManager;

static X86_INTERRUPT_CONTROLLER: X86InterruptController = X86InterruptController;
static X86_MEMORY_MANAGER: X86MemoryManager = X86MemoryManager;

impl InterruptController for X86InterruptController {
    unsafe fn init(&self) {
        unsafe {
            pic::Pic::disable();
            apic::init_local_apic();
        }
    }

    unsafe fn enable_interrupt(&self, irq: u32) {
        let _ = irq;
    }

    unsafe fn disable_interrupt(&self, irq: u32) {
        let _ = irq;
    }

    unsafe fn end_of_interrupt(&self, irq: u32) {
        let _ = irq;
        unsafe { apic::eoi(); }
    }

    fn is_spurious(&self, vector: u8) -> bool {
        vector == 0xFF
    }
}

impl MemoryManager for X86MemoryManager {
    unsafe fn map_page(&self, _virt: usize, _phys: usize, _flags: u64) -> Result<(), &'static str> {
        Err("Global map_page requires active frame allocator context")
    }

    unsafe fn unmap_page(&self, _virt: usize) -> Result<(), &'static str> {
        Err("Global unmap_page requires active page table context")
    }

    fn virtual_to_physical(&self, _virt: usize) -> Option<usize> {
        None
    }

    fn flush_tlb(&self) {
        #[cfg(target_os = "none")]
        unsafe {
            use x86_64::registers::control::Cr3;
            let (frame, flags) = Cr3::read();
            Cr3::write(frame, flags);
        }
    }

    fn current_table_base(&self) -> usize {
        #[cfg(target_os = "none")]
        {
            use x86_64::registers::control::Cr3;
            Cr3::read().0.start_address().as_u64() as usize
        }
        #[cfg(not(target_os = "none"))]
        0
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
