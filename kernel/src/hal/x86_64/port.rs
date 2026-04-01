use crate::interfaces::PortIo;
use core::arch::asm;

pub struct X86PortIo;

impl PortIo for X86PortIo {
    #[inline(always)]
    unsafe fn outb(port: u16, value: u8) {
        // Safety: caller guarantees the port/value pair is valid for an OUTB operation.
        unsafe {
            asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
        }
    }

    #[inline(always)]
    unsafe fn inb(port: u16) -> u8 {
        let value: u8;
        // Safety: caller guarantees the port is valid for an INB operation.
        unsafe {
            asm!("in al, dx", out("al") value, in("dx") port, options(nomem, nostack, preserves_flags));
        }
        value
    }

    #[inline(always)]
    unsafe fn outw(port: u16, value: u16) {
        // Safety: caller guarantees the port/value pair is valid for an OUTW operation.
        unsafe {
            asm!("out dx, ax", in("dx") port, in("ax") value, options(nomem, nostack, preserves_flags));
        }
    }

    #[inline(always)]
    unsafe fn inw(port: u16) -> u16 {
        let value: u16;
        // Safety: caller guarantees the port is valid for an INW operation.
        unsafe {
            asm!("in ax, dx", out("ax") value, in("dx") port, options(nomem, nostack, preserves_flags));
        }
        value
    }

    #[inline(always)]
    unsafe fn outd(port: u16, value: u32) {
        // Safety: caller guarantees the port/value pair is valid for an OUTD operation.
        unsafe {
            asm!("out dx, eax", in("dx") port, in("eax") value, options(nomem, nostack, preserves_flags));
        }
    }

    #[inline(always)]
    unsafe fn ind(port: u16) -> u32 {
        let value: u32;
        // Safety: caller guarantees the port is valid for an IND operation.
        unsafe {
            asm!("in eax, dx", out("eax") value, in("dx") port, options(nomem, nostack, preserves_flags));
        }
        value
    }
}
