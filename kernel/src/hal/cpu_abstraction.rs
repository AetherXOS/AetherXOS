//! CPU abstraction layer - unified across x86_64 and aarch64
//!
//! This module provides platform-independent CPU abstractions.
//! Architecture-specific context representations are handled internally.

use super::abstractions::{CpuContext, X86_64Context, InitResult};

/// CPU ID type (platform-independent)
pub type CpuId = u32;

/// CPU context manager - handles platform-specific context operations
pub trait CpuContextManager: Send + Sync {
    /// Get current CPU ID
    fn current_cpu_id(&self) -> CpuId;

    /// Get CPU context (current state of all registers)
    fn get_context(&self) -> CpuContext;

    /// Set CPU context (restore register state)
    fn set_context(&self, context: CpuContext) -> InitResult;

    /// Initialize per-CPU local storage
    fn init_cpu_local(&self, local_ptr: usize) -> InitResult;

    /// Number of CPUs available
    fn cpu_count(&self) -> u32;

    /// Check if CPU is online
    fn is_cpu_online(&self, cpu_id: CpuId) -> bool;

    /// Bring CPU online
    fn bring_cpu_online(&self, cpu_id: CpuId) -> InitResult;
}

/// Platform-independent CPU context conversions
impl CpuContext {
    /// Create x86_64 context from raw register values
    #[cfg(target_arch = "x86_64")]
    pub fn from_x86_64_regs(
        rax: u64, rbx: u64, rcx: u64, rdx: u64, rsi: u64, rdi: u64,
        rbp: u64, rsp: u64, r8: u64, r9: u64, r10: u64, r11: u64,
        r12: u64, r13: u64, r14: u64, r15: u64,
        rip: u64, rflags: u64, cr3: u64,
    ) -> Self {
        CpuContext::X86_64(X86_64Context {
            rax, rbx, rcx, rdx, rsi, rdi, rbp, rsp,
            r8, r9, r10, r11, r12, r13, r14, r15,
            rip, rflags, cr3,
        })
    }

    /// Create aarch64 context
    #[cfg(target_arch = "aarch64")]
    pub fn from_aarch64_regs(
        x: [u64; 31],
        sp: u64,
        pc: u64,
        pstate: u64,
        ttbr0_el1: u64,
    ) -> Self {
        CpuContext::AArch64(AArch64Context {
            x, sp, pc, pstate, ttbr0_el1,
        })
    }

    /// Get instruction pointer (program counter)
    pub fn instruction_pointer(&self) -> u64 {
        match self {
            CpuContext::X86_64(ctx) => ctx.rip,
            CpuContext::AArch64(ctx) => ctx.pc,
        }
    }

    /// Get stack pointer
    pub fn stack_pointer(&self) -> u64 {
        match self {
            CpuContext::X86_64(ctx) => ctx.rsp,
            CpuContext::AArch64(ctx) => ctx.sp,
        }
    }

    /// Get frame pointer / base pointer
    pub fn frame_pointer(&self) -> u64 {
        match self {
            CpuContext::X86_64(ctx) => ctx.rbp,
            CpuContext::AArch64(ctx) => ctx.x[29], // FP is x29 on aarch64
        }
    }

    /// Get first argument register
    pub fn arg_register_0(&self) -> u64 {
        match self {
            CpuContext::X86_64(ctx) => ctx.rdi,
            CpuContext::AArch64(ctx) => ctx.x[0],
        }
    }

    /// Get second argument register
    pub fn arg_register_1(&self) -> u64 {
        match self {
            CpuContext::X86_64(ctx) => ctx.rsi,
            CpuContext::AArch64(ctx) => ctx.x[1],
        }
    }

    /// Get return value register
    pub fn return_register(&self) -> u64 {
        match self {
            CpuContext::X86_64(ctx) => ctx.rax,
            CpuContext::AArch64(ctx) => ctx.x[0],
        }
    }

    /// Set instruction pointer
    pub fn set_instruction_pointer(&mut self, ip: u64) {
        match self {
            CpuContext::X86_64(ctx) => ctx.rip = ip,
            CpuContext::AArch64(ctx) => ctx.pc = ip,
        }
    }

    /// Set stack pointer
    pub fn set_stack_pointer(&mut self, sp: u64) {
        match self {
            CpuContext::X86_64(ctx) => ctx.rsp = sp,
            CpuContext::AArch64(ctx) => ctx.sp = sp,
        }
    }

    /// Set first argument register (RDI / X0)
    pub fn set_arg_register_0(&mut self, val: u64) {
        match self {
            CpuContext::X86_64(ctx) => ctx.rdi = val,
            CpuContext::AArch64(ctx) => ctx.x[0] = val,
        }
    }

    /// Set second argument register (RSI / X1)
    pub fn set_arg_register_1(&mut self, val: u64) {
        match self {
            CpuContext::X86_64(ctx) => ctx.rsi = val,
            CpuContext::AArch64(ctx) => ctx.x[1] = val,
        }
    }

    /// Set third argument register (RDX / X2)
    pub fn set_arg_register_2(&mut self, val: u64) {
        match self {
            CpuContext::X86_64(ctx) => ctx.rdx = val,
            CpuContext::AArch64(ctx) => ctx.x[2] = val,
        }
    }

    /// Set return-value register (RAX / X0)
    pub fn set_return_register(&mut self, val: u64) {
        match self {
            CpuContext::X86_64(ctx) => ctx.rax = val,
            CpuContext::AArch64(ctx) => ctx.x[0] = val,
        }
    }

    /// Set flags register (RFLAGS / SPSR)
    pub fn set_flags(&mut self, flags: u64) {
        match self {
            CpuContext::X86_64(ctx) => ctx.rflags = flags,
            CpuContext::AArch64(ctx) => ctx.pstate = flags,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_x86_context_from_regs() {
        let ctx = CpuContext::from_x86_64_regs(
            0x1, 0x2, 0x3, 0x4, 0x5, 0x6,
            0x7, 0x8, 0x9, 0xa, 0xb, 0xc,
            0xd, 0xe, 0xf, 0x10,
            0x1000, 0x200, 0x3000,
        );
        assert_eq!(ctx.instruction_pointer(), 0x1000);
        assert_eq!(ctx.stack_pointer(), 0x8);
        assert_eq!(ctx.arg_register_0(), 0x6);
    }

    #[test]
    fn test_context_conversion() {
        // This would test platform-specific conversions
        #[cfg(target_arch = "x86_64")]
        {
            let mut ctx = CpuContext::from_x86_64_regs(
                0x1, 0x2, 0x3, 0x4, 0x5, 0x6,
                0x7, 0x8, 0x9, 0xa, 0xb, 0xc,
                0xd, 0xe, 0xf, 0x10,
                0x1000, 0x200, 0x3000,
            );
            ctx.set_instruction_pointer(0x2000);
            assert_eq!(ctx.instruction_pointer(), 0x2000);
        }
    }
}
