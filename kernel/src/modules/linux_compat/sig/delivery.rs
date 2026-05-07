use super::super::*;
use crate::interfaces::task::{Context, KernelTask};
use crate::modules::linux_compat::types::{LinuxUContext, LinuxMContext, LinuxStatxTimestamp};
use crate::modules::posix::signal::SignalAction;

/// Construct a Linux-compatible signal frame on the user stack.
/// 
/// This involves:
/// 1. Calculating the redzone-safe stack location.
/// 2. Pushing the `LinuxUContext` (which includes register state and signal mask).
/// 3. Setting up the registers to enter the handler (RDI=signum, RIP=handler).
/// 4. Returning the new RSP.
pub fn setup_linux_sigframe(
    task: &mut KernelTask,
    signum: i32,
    action: &SignalAction,
) -> Result<u64, &'static str> {
    #[cfg(target_arch = "x86_64")]
    {
        // 1. Calculate new stack pointer (aligned to 16 bytes)
        // Linux x86_64 ABI: RSP must be 16-byte aligned before calling handler.
        // We also need to account for the redzone (128 bytes).
        let mut sp = task.context.rsp;
        
        // If the task has an alternate signal stack, use it.
        if let Some(ss) = task.signal_stack {
            if !task.signal_stack_active && (ss.ss_flags & 2) == 0 { // 2 = SS_DISABLE
                sp = ss.ss_sp + ss.ss_size;
                task.signal_stack_active = true;
            }
        }

        sp -= core::mem::size_of::<LinuxUContext>() as u64;
        sp &= !15; // Align to 16 bytes
        
        // 2. Prepare the context
        let mut uctx: LinuxUContext = unsafe { core::mem::zeroed() };
        uctx.sigmask = task.signal_mask;
        uctx.mcontext = LinuxMContext {
            r15: task.context.r15,
            r14: task.context.r14,
            r13: task.context.r13,
            r12: task.context.r12,
            r11: task.context.r11,
            r10: task.context.r10,
            r9: task.context.r9,
            r8: task.context.r8,
            rdi: task.context.rdi,
            rsi: task.context.rsi,
            rbp: task.context.rbp,
            rbx: task.context.rbx,
            rdx: task.context.rdx,
            rax: task.context.rax,
            rcx: task.context.rcx,
            rsp: task.context.rsp,
            rip: task.context.rip,
            eflags: task.context.rflags,
            cs: task.context.cs as u16,
            gs: 0, // GS/FS/SS handled by arch_prctl or gdt
            fs: 0,
            ss: task.context.ss as u16,
            err: 0,
            trapno: 0,
            oldmask: 0,
            cr2: 0,
            fpstate: 0,
            __reserved1: [0; 8],
        };

        // 3. Write to user stack
        // For performance, we use the Direct Map (HHDM).
        unsafe {
            let phys_addr = translate_user_vaddr(task.page_table_root, sp)?;
            let hhdm = crate::hal::hhdm_offset().unwrap_or(0);
            let kernel_ptr = (phys_addr + hhdm) as *mut LinuxUContext;
            core::ptr::write(kernel_ptr, uctx);
        }

        // 4. Update task registers for handler entry
        task.context.rdi = signum as u64;
        task.context.rsi = sp; // Some ABIs pass ucontext in RSI
        task.context.rdx = sp + 8; // Some ABIs pass siginfo in RDX
        task.context.rax = 0;
        
        // If sa_restorer is present, push it as the return address
        if action.restorer != 0 {
            sp -= 8;
            unsafe {
                let phys_addr = translate_user_vaddr(task.page_table_root, sp)?;
                let hhdm = crate::hal::hhdm_offset().unwrap_or(0);
                let kernel_ptr = (phys_addr + hhdm) as *mut u64;
                core::ptr::write(kernel_ptr, action.restorer);
            }
        }

        task.context.rsp = sp;
        Ok(sp)
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        Err("Signal injection only implemented for x86_64")
    }
}

/// Helper to translate user virtual address to physical address using task's page table.
fn translate_user_vaddr(cr3: u64, vaddr: u64) -> Result<u64, &'static str> {
    use x86_64::structures::paging::Translate;
    use x86_64::VirtAddr;
    
    let hhdm = crate::hal::hhdm_offset().unwrap_or(0);
    let l4_table_ptr = (cr3 + hhdm) as *mut x86_64::structures::paging::PageTable;
    let l4_table = unsafe { &mut *l4_table_ptr };
    
    let mapper = unsafe { x86_64::structures::paging::OffsetPageTable::new(l4_table, VirtAddr::new(hhdm)) };
    match mapper.translate_addr(VirtAddr::new(vaddr)) {
        Some(phys) => Ok(phys.as_u64()),
        None => Err("User stack not mapped or swapped out"),
    }
}
