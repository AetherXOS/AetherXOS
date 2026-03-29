use super::port::X86PortIo;
use crate::interfaces::PortIo;

// Standard PIC ports (Master: 0x20/0x21, Slave: 0xA0/0xA1)
const PIC1_CMD: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;
const PIC2_CMD: u16 = 0xA0;
const PIC2_DATA: u16 = 0xA1;

const PIC_EOI: u8 = 0x20;

use crate::interfaces::InterruptController;

/// 8259 Programmable Interrupt Controller
pub struct Pic {
    // Only needed if we tracked state, but PIC is stateless to us mostly
}

impl InterruptController for Pic {
    unsafe fn initialize(&mut self) {
        // Safety: PIC remap offsets are the standard legacy x86 IRQ vectors.
        unsafe { Pic::init(32, 40) };
    }

    unsafe fn enable_interrupt(&mut self, irq: u8) {
        let port = if irq < 8 { PIC1_DATA } else { PIC2_DATA };
        // Safety: PIC mask registers live on fixed x86 I/O ports.
        let current_mask = unsafe { X86PortIo::inb(port) };
        let shift = if irq < 8 { irq } else { irq - 8 };
        // Safety: PIC mask registers live on fixed x86 I/O ports.
        unsafe { X86PortIo::outb(port, current_mask & !(1 << shift)) };
    }

    unsafe fn disable_interrupt(&mut self, irq: u8) {
        let port = if irq < 8 { PIC1_DATA } else { PIC2_DATA };
        // Safety: PIC mask registers live on fixed x86 I/O ports.
        let current_mask = unsafe { X86PortIo::inb(port) };
        let shift = if irq < 8 { irq } else { irq - 8 };
        // Safety: PIC mask registers live on fixed x86 I/O ports.
        unsafe { X86PortIo::outb(port, current_mask | (1 << shift)) };
    }

    unsafe fn end_of_interrupt(&mut self, irq: u8) {
        // Safety: EOI is delivered only to the legacy PIC command ports.
        unsafe { Pic::send_eoi(irq) };
    }
}

impl Pic {
    /// Send End of Interrupt (EOI) to PICs.
    /// Standard procedure: If IRQ >= 8 (Slave), send to Slave AND Master.
    /// If IRQ < 8 (Master), send only to Master.
    /// However, raw IRQ from IDT (vector) is usually mapped to 32+IRQ.
    /// Assuming `irq_vector` - 32 = `irq_line`.
    pub unsafe fn send_eoi(irq_line: u8) {
        if irq_line >= 8 {
            // Safety: PIC2 command port accepts EOI commands.
            unsafe { X86PortIo::outb(PIC2_CMD, PIC_EOI) };
        }
        // Safety: PIC1 command port accepts EOI commands.
        unsafe { X86PortIo::outb(PIC1_CMD, PIC_EOI) };
    }

    /// Initialize PICs and remap IRQs.
    /// Standard is master_offset=32 (0x20), slave_offset=40 (0x28) to avoid CPU exceptions 0-31.
    pub unsafe fn init(master_offset: u8, slave_offset: u8) {
        // ICW1: Init command
        // Safety: PIC init sequence targets fixed legacy PIC ports.
        unsafe { X86PortIo::outb(PIC1_CMD, 0x11) };
        unsafe { X86PortIo::outb(PIC2_CMD, 0x11) };

        // ICW2: Remap vectors
        unsafe { X86PortIo::outb(PIC1_DATA, master_offset) };
        unsafe { X86PortIo::outb(PIC2_DATA, slave_offset) };

        // ICW3: Cascade
        unsafe { X86PortIo::outb(PIC1_DATA, 4) };
        unsafe { X86PortIo::outb(PIC2_DATA, 2) };

        // ICW4: 8086 mode
        unsafe { X86PortIo::outb(PIC1_DATA, 0x01) };
        unsafe { X86PortIo::outb(PIC2_DATA, 0x01) };

        // OCW1: Mask interrupts
        unsafe { X86PortIo::outb(PIC1_DATA, 0xFE) }; // Only IRQ 0 (Timer) -- No, mask ALL for disable logic

        // Actually, init enables. Let's add disable separately.
    }

    /// Disable the legacy PIC by masking all interrupts (0xFF).
    pub unsafe fn disable() {
        // Safety: masking all interrupts writes only the PIC data ports.
        unsafe { X86PortIo::outb(PIC1_DATA, 0xFF) };
        unsafe { X86PortIo::outb(PIC2_DATA, 0xFF) };
    }
}
