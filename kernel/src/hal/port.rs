use core::marker::PhantomData;

use crate::interfaces::PortIo;

#[cfg(target_arch = "x86_64")]
type Backend = crate::hal::x86_64::port::X86PortIo;

#[cfg(not(target_arch = "x86_64"))]
struct Backend;

#[cfg(not(target_arch = "x86_64"))]
impl PortIo for Backend {
    unsafe fn outb(_port: u16, _value: u8) {}
    unsafe fn inb(_port: u16) -> u8 { 0 }
    unsafe fn outw(_port: u16, _value: u16) {}
    unsafe fn inw(_port: u16) -> u16 { 0 }
    unsafe fn outd(_port: u16, _value: u32) {}
    unsafe fn ind(_port: u16) -> u32 { 0 }
}

pub struct Port<T> {
    port: u16,
    _phantom: PhantomData<T>,
}

impl<T> Port<T> {
    pub const fn new(port: u16) -> Self {
        Self { port, _phantom: PhantomData }
    }
}

impl Port<u8> {
    pub unsafe fn read(&mut self) -> u8 {
        unsafe { Backend::inb(self.port) }
    }

    pub unsafe fn write(&mut self, value: u8) {
        unsafe { Backend::outb(self.port, value) }
    }
}

impl Port<u16> {
    pub unsafe fn read(&mut self) -> u16 {
        unsafe { Backend::inw(self.port) }
    }

    pub unsafe fn write(&mut self, value: u16) {
        unsafe { Backend::outw(self.port, value) }
    }
}

impl Port<u32> {
    pub unsafe fn read(&mut self) -> u32 {
        unsafe { Backend::ind(self.port) }
    }

    pub unsafe fn write(&mut self, value: u32) {
        unsafe { Backend::outd(self.port, value) }
    }
}