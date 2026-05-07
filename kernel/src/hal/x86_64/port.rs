use crate::interfaces::PortIo;
// Use fully-qualified `core::arch::asm!` in this module to avoid an unused import warning.

pub struct X86PortIo;

impl PortIo for X86PortIo {
    #[inline(always)]
    unsafe fn outb(port: u16, value: u8) {
        #[cfg(target_os = "none")]
        {
            // Safety: caller guarantees the port/value pair is valid for an OUTB operation.
            unsafe {
                core::arch::asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
            }
        }
        #[cfg(not(target_os = "none"))]
        {
            let _ = (port, value);
        }
    }

    #[inline(always)]
    unsafe fn inb(port: u16) -> u8 {
        #[cfg(target_os = "none")]
        {
            let value: u8;
            // Safety: caller guarantees the port is valid for an INB operation.
            unsafe {
                core::arch::asm!("in al, dx", out("al") value, in("dx") port, options(nomem, nostack, preserves_flags));
            }
            value
        }
        #[cfg(not(target_os = "none"))]
        {
            let _ = port;
            0
        }
    }

    #[inline(always)]
    unsafe fn outw(port: u16, value: u16) {
        #[cfg(target_os = "none")]
        {
            // Safety: caller guarantees the port/value pair is valid for an OUTW operation.
            unsafe {
                core::arch::asm!("out dx, ax", in("dx") port, in("ax") value, options(nomem, nostack, preserves_flags));
            }
        }
        #[cfg(not(target_os = "none"))]
        {
            let _ = (port, value);
        }
    }

    #[inline(always)]
    unsafe fn inw(port: u16) -> u16 {
        #[cfg(target_os = "none")]
        {
            let value: u16;
            // Safety: caller guarantees the port is valid for an INW operation.
            unsafe {
                core::arch::asm!("in ax, dx", out("ax") value, in("dx") port, options(nomem, nostack, preserves_flags));
            }
            value
        }
        #[cfg(not(target_os = "none"))]
        {
            let _ = port;
            0
        }
    }

    #[inline(always)]
    unsafe fn outd(port: u16, value: u32) {
        #[cfg(target_os = "none")]
        {
            // Safety: caller guarantees the port/value pair is valid for an OUTD operation.
            unsafe {
                core::arch::asm!("out dx, eax", in("dx") port, in("eax") value, options(nomem, nostack, preserves_flags));
            }
        }
        #[cfg(not(target_os = "none"))]
        {
            let _ = (port, value);
        }
    }

    #[inline(always)]
    unsafe fn ind(port: u16) -> u32 {
        #[cfg(target_os = "none")]
        {
            let value: u32;
            // Safety: caller guarantees the port is valid for an IND operation.
            unsafe {
                core::arch::asm!("in eax, dx", out("eax") value, in("dx") port, options(nomem, nostack, preserves_flags));
            }
            value
        }
        #[cfg(not(target_os = "none"))]
        {
            let _ = port;
            0
        }
    }
}
