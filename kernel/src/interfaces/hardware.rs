/// Hardware Abstraction Layer Interface (DIP)
pub trait HardwareAbstraction {
    fn enable_interrupts();
    fn disable_interrupts();
    /// Disables interrupts and returns previous state (flags).
    fn irq_save() -> usize;
    /// Restores interrupt state from flags.
    fn irq_restore(flags: usize);
    fn halt();

    // Lifecycle hooks
    fn early_init();
    fn init_interrupts();
    fn init_timer();
    fn init_smp();
    fn init_cpu_local(ptr: usize);

    // Power Management
    fn set_performance_profile(profile: PerformanceProfile);

    // Diagnostics
    fn serial_write_raw(s: &str);
    fn panic_with_report(info: &core::panic::PanicInfo, report: &crate::kernel::CrashReport) -> !;
    fn fatal_halt(reason: &str) -> !;
    fn idle_once();
    
    // Time
    fn get_time_ns() -> u64;

    // Sub-component Accessors (Pillar II: HAL Bridge)
    fn interrupt_controller() -> &'static dyn InterruptController;
    fn memory_manager() -> &'static dyn MemoryManager;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerformanceProfile {
    HighPerformance,
    Balanced,
    PowerSaving,
}

/// Port I/O Abstraction (for x86 in/out instructions)
pub trait PortIo {
    unsafe fn outb(port: u16, value: u8);
    unsafe fn inb(port: u16) -> u8;
    unsafe fn outw(port: u16, value: u16);
    unsafe fn inw(port: u16) -> u16;
    unsafe fn outd(port: u16, value: u32);
    unsafe fn ind(port: u16) -> u32;
}

/// Serial Device Abstraction
pub trait SerialDevice: core::fmt::Write {
    fn init(&mut self);
    fn send(&mut self, data: u8);
}

/// Interrupt Controller Abstraction (PIC, APIC, GIC)
pub trait InterruptController: Send + Sync {
    unsafe fn init(&self);
    unsafe fn enable_interrupt(&self, irq: u32);
    unsafe fn disable_interrupt(&self, irq: u32);
    unsafe fn end_of_interrupt(&self, irq: u32);
    fn is_spurious(&self, vector: u8) -> bool;
}

/// Memory Management Abstraction (Paging, TLB, Cache)
pub trait MemoryManager: Send + Sync {
    unsafe fn map_page(&self, virt: usize, phys: usize, flags: u64) -> Result<(), &'static str>;
    unsafe fn unmap_page(&self, virt: usize) -> Result<(), &'static str>;
    fn virtual_to_physical(&self, virt: usize) -> Option<usize>;
    fn flush_tlb(&self);
    fn current_table_base(&self) -> usize;
}

/// System Timer Abstraction
pub trait Timer {
    fn init(&mut self);
    fn set_oneshot_ns(&mut self, ns: u64);
    fn uptime_ns(&self) -> u64;
    fn frequency_hz(&self) -> u64;
}

/// PCI Controller Abstraction
pub trait PciController {
    unsafe fn read_config_byte(&self, bus: u8, slot: u8, func: u8, offset: u8) -> u8;
    unsafe fn read_config_word(&self, bus: u8, slot: u8, func: u8, offset: u8) -> u16;
    unsafe fn read_config_dword(&self, bus: u8, slot: u8, func: u8, offset: u8) -> u32;
    unsafe fn write_config_byte(&self, bus: u8, slot: u8, func: u8, offset: u8, value: u8);
    unsafe fn write_config_word(&self, bus: u8, slot: u8, func: u8, offset: u8, value: u16);
    unsafe fn write_config_dword(&self, bus: u8, slot: u8, func: u8, offset: u8, value: u32);
}
