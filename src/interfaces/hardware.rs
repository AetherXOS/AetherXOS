/// Hardware Abstraction Layer Interface (DIP)
pub trait HardwareAbstraction {
    fn enable_interrupts();
    fn disable_interrupts();
    /// Disables interrupts and returns previous state (flags).
    fn irq_save() -> usize;
    /// Restores interrupt state from flags.
    fn irq_restore(flags: usize);
    fn halt();
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
pub trait InterruptController {
    unsafe fn initialize(&mut self);
    unsafe fn enable_interrupt(&mut self, irq: u8);
    unsafe fn disable_interrupt(&mut self, irq: u8);
    unsafe fn end_of_interrupt(&mut self, irq: u8);
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
