/// Architecture-specific CPU Context (Registers)
#[derive(Debug, Clone, Default, Copy)]
#[repr(C)]
#[cfg(target_arch = "x86_64")]
pub struct Context {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rbp: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

#[cfg(target_arch = "x86_64")]
impl Context {
    /// Return the instruction pointer (arch-agnostic accessor).
    #[inline(always)]
    pub fn rip(&self) -> u64 {
        self.rip
    }
}

/// Architecture-specific CPU Context (Registers) — AArch64
#[derive(Debug, Clone, Default, Copy)]
#[repr(C)]
#[cfg(target_arch = "aarch64")]
pub struct Context {
    pub x19: u64,
    pub x20: u64,
    pub x21: u64,
    pub x22: u64,
    pub x23: u64,
    pub x24: u64,
    pub x25: u64,
    pub x26: u64,
    pub x27: u64,
    pub x28: u64,
    pub x29: u64, // FP
    pub x30: u64, // LR (return address)
    pub sp: u64,
    pub elr: u64,  // Exception Link Register (PC to return to)
    pub spsr: u64, // Saved Program Status Register
}

#[cfg(target_arch = "aarch64")]
impl Context {
    #[inline(always)]
    pub fn rip(&self) -> u64 {
        self.elr
    }
}

/// Fallback for unsupported architectures
#[derive(Debug, Clone, Default, Copy)]
#[repr(C)]
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
pub struct Context {
    pub pc: u64,
    pub sp: u64,
    pub flags: u64,
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
impl Context {
    #[inline(always)]
    pub fn rip(&self) -> u64 {
        self.pc
    }
}
