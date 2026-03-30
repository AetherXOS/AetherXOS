use super::port::X86PortIo;
use crate::interfaces::PortIo;
use crate::kernel::bit_utils::pic as bits;
use crate::interfaces::InterruptController;

/// 8259 Programmable Interrupt Controller (PIC).
/// Used only for legacy x86 interrupt management or pre-APIC initialization.
pub struct Pic;

impl InterruptController for Pic {
    unsafe fn init(&mut self) {
        // Safety: PIC remap offsets (32, 40) are standard to avoid CPU exceptions.
        unsafe { Pic::remap(32, 40) };
    }

    unsafe fn enable_interrupt(&mut self, irq: u32) {
        let port = if irq < 8 { bits::MASTER_DATA } else { bits::SLAVE_DATA };
        let mask = unsafe { X86PortIo::inb(port) };
        let bit = if irq < 8 { irq } else { irq - 8 };
        unsafe { X86PortIo::outb(port, mask & !(1 << bit as u8)) };
    }

    unsafe fn disable_interrupt(&mut self, irq: u32) {
        let port = if irq < 8 { bits::MASTER_DATA } else { bits::SLAVE_DATA };
        let mask = unsafe { X86PortIo::inb(port) };
        let bit = if irq < 8 { irq } else { irq - 8 };
        unsafe { X86PortIo::outb(port, mask | (1 << bit as u8)) };
    }

    unsafe fn end_of_interrupt(&mut self, irq: u32) {
        unsafe { Pic::send_eoi(irq as u8) };
    }
}

impl Pic {
    /// Send End of Interrupt (EOI) to the PICs.
    pub unsafe fn send_eoi(irq_line: u8) {
        if irq_line >= 8 {
            unsafe { X86PortIo::outb(bits::SLAVE_CMD, bits::EOI) };
        }
        unsafe { X86PortIo::outb(bits::MASTER_CMD, bits::EOI) };
    }

    /// Remap the PIC interrupt vectors.
    pub unsafe fn remap(master_offset: u8, slave_offset: u8) {
        // ICW1: Start initialization
        unsafe {
            X86PortIo::outb(bits::MASTER_CMD, bits::ICW1_INIT | bits::ICW1_ICW4);
            X86PortIo::outb(bits::SLAVE_CMD, bits::ICW1_INIT | bits::ICW1_ICW4);

            // ICW2: Vector offsets
            X86PortIo::outb(bits::MASTER_DATA, master_offset);
            X86PortIo::outb(bits::SLAVE_DATA, slave_offset);

            // ICW3: Cascade configuration
            X86PortIo::outb(bits::MASTER_DATA, 4); // Slave at IRQ2
            X86PortIo::outb(bits::SLAVE_DATA, 2);  // Identity of slave

            // ICW4: Mode selection (8086 mode)
            X86PortIo::outb(bits::MASTER_DATA, bits::ICW4_8086);
            X86PortIo::outb(bits::SLAVE_DATA, bits::ICW4_8086);

            // OCW1: Mask all interrupts initially
            X86PortIo::outb(bits::MASTER_DATA, 0xFF);
            X86PortIo::outb(bits::SLAVE_DATA, 0xFF);
        }
    }

    /// Disable the legacy PIC entirely.
    pub unsafe fn disable() {
        unsafe {
            X86PortIo::outb(bits::MASTER_DATA, 0xFF);
            X86PortIo::outb(bits::SLAVE_DATA, 0xFF);
        }
    }
}
