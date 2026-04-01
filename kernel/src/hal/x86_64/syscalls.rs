use crate::kernel::syscalls::syscalls_consts::x86;
use core::arch::naked_asm;
use x86_64::registers::model_specific::{Efer, EferFlags, LStar, SFMask, Star};
use x86_64::registers::rflags::RFlags;

/// Initialize Syscall MSRs
pub fn init(selectors: &super::gdt::Selectors) {
    unsafe {
        // 1. Enable SYSCALL/SYSRET instruction via EFER.SCE
        Efer::update(|flags| {
            flags.insert(EferFlags::SYSTEM_CALL_EXTENSIONS);
        });

        // 2. Set LSTAR to our handler function address
        LStar::write(x86_64::VirtAddr::new(syscall_handler as *const () as u64));

        // 3. Set SFMask (RFlags mask) - clear interrupts, trap bit, etc. on syscall entry
        // Usually we execute with interrupts disabled until we decide otherwise.
        SFMask::write(RFlags::INTERRUPT_FLAG | RFlags::TRAP_FLAG | RFlags::DIRECTION_FLAG);

        // 4. Set STAR register
        use x86_64::registers::segmentation::SegmentSelector;

        let kernel_code = selectors.kernel_code_selector;
        let kernel_data = selectors.kernel_data_selector;

        let user_data = SegmentSelector(kernel_data.0 + 8);
        let user_code = SegmentSelector(kernel_data.0 + 16);

        let _ = Star::write(user_code, user_data, kernel_code, kernel_data);
    }
}

/// Raw Syscall Entry Point (Ring 3 -> Ring 0)
#[unsafe(naked)]
pub unsafe extern "C" fn syscall_handler() {
    naked_asm!(
        "swapgs",                  // Switch to Kernel GS Base (CpuLocal)
        "mov gs:[{scratch}], rsp", // Save User Stack Pointer to CpuLocal.scratch
        "mov rsp, gs:[{kstack}]",  // Load Kernel Stack Pointer from CpuLocal.kernel_stack_top
        "push rbp",
        "push rbx",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        "push r11", // Saved RFLAGS
        "push rcx", // Saved RIP (User)
        "push rdi",
        "push rsi",
        "push rdx",
        "push rax", // original rax (often used for syscall result)

        // Pass a pointer to the Frame on the stack as arg 10 (System V ABI)
        // Frame starts at RSP: [rax, rdx, rsi, rdi, rcx, r11, r15, r14, r13, r12, rbx, rbp]
        "sub rsp, 32",
        "mov [rsp], rax",              // arg 7: syscall_id
        "mov rax, [rsp + 64]",         // [rsp+64] is rcx (user_rip)
        "mov [rsp + 8], rax",          // arg 8: user_rip
        "mov rax, [rsp + 72]",         // [rsp+72] is r11 (user_rflags)
        "mov [rsp + 16], rax",         // arg 9: user_rflags
        "lea rax, [rsp + 32]",         // Pointer to frame start (rax)
        "mov [rsp + 24], rax",         // arg 10: frame_ptr

        "call rust_syscall_handler",
        "add rsp, 32",

        // Return override/result
        "mov [rsp], rax", // Override the pushed RAX with the syscall result!
        "pop rax", // Restore RAX (The result)
        "pop rdx",
        "pop rsi",
        "pop rdi",
        "pop rcx", // Restore RIP
        "pop r11", // Restore RFLAGS
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbx",
        "pop rbp",

        "mov rsp, gs:[{scratch}]",
        "swapgs",
        "sysretq",
        scratch = const x86::CPU_LOCAL_SCRATCH,
        kstack = const x86::CPU_LOCAL_KSTACK_TOP,
    );
}
