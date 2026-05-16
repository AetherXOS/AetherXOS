use super::super::*;
use crate::interfaces::task::{KernelTask};
use crate::modules::linux_compat::types::{LinuxUContext, LinuxMContext, LinuxSiginfo, LinuxStackT};
use crate::modules::posix::signal::SignalAction;

/// Linux x86_64 rt_sigframe layout (mirrors kernel's arch/x86/include/asm/sigframe.h)
/// Stack layout when entering handler:
///   [RSP+0]   = return address (sa_restorer or vDSO sigreturn trampoline)
///   [RSP+8]   = rt_sigframe::pretcode (same as above, for compatibility)
///   rt_sigframe {
///       pretcode: *const u8,    // pointer to restorer
///       sig:      i32,
///       _pad:     i32,
///       pinfo:    *LinuxSiginfo,
///       puc:      *LinuxUContext,
///       info:     LinuxSiginfo,
///       uc:       LinuxUContext,
///   }
#[repr(C)]
struct RtSigFrame {
    pretcode: u64,       // sa_restorer address
    sig: i32,
    _pad: i32,
    pinfo: u64,          // pointer to info field within frame
    puc: u64,            // pointer to uc field within frame
    info: LinuxSiginfo,
    uc: LinuxUContext,
}

/// Construct a Linux x86_64 RT signal frame on the user stack.
///
/// Follows the exact layout expected by glibc/musl rt_sigreturn.
/// Steps:
///   1. Select stack (alternate or main, subtract 128-byte redzone)
///   2. Allocate and align RtSigFrame
///   3. Fill siginfo_t and ucontext with saved register state
///   4. Push restorer address as fake return address
///   5. Redirect RIP to handler, set RDI/RSI/RDX for SA_SIGINFO calling convention
pub fn setup_linux_sigframe(
    task: &mut KernelTask,
    signum: i32,
    action: &SignalAction,
) -> Result<u64, &'static str> {
    #[cfg(target_arch = "x86_64")]
    {
        // --- Step 1: Choose stack ---
        let mut sp: u64 = task.context.stack_pointer();

        // Use alternate signal stack if configured and not already active
        if let Some(ss) = task.signal_stack {
            const SS_DISABLE: i32 = 2;
            const SS_ONSTACK: i32 = 1;
            let on_altstack = (ss.ss_flags & SS_ONSTACK) != 0;
            let disabled = (ss.ss_flags & SS_DISABLE) != 0;
            if !on_altstack && !disabled && action.flags & (1 << 3) != 0 { // SA_ONSTACK = bit 3 on x86_64
                sp = ss.ss_sp + ss.ss_size;
                task.signal_stack_active = true;
            }
        }

        // Subtract 128-byte redzone (x86_64 ABI)
        sp = sp.saturating_sub(128);

        // --- Step 2: Allocate RtSigFrame ---
        let frame_size = core::mem::size_of::<RtSigFrame>() as u64;
        sp = sp.saturating_sub(frame_size);
        sp &= !15u64; // 16-byte align
        // Extra alignment: Linux aligns so that (sp + 8) % 16 == 0 for CALL instruction
        // CALL pushes 8-byte return address, so before CALL sp must be 16-byte aligned
        // meaning at handler entry sp is 16n-8 → we need sp to be 16n-8 here
        if sp & 8 == 0 {
            sp = sp.wrapping_sub(8);
        }

        let frame_vaddr = sp;
        let info_vaddr = frame_vaddr + core::mem::offset_of!(RtSigFrame, info) as u64;
        let uc_vaddr   = frame_vaddr + core::mem::offset_of!(RtSigFrame, uc)   as u64;

        // --- Step 3: Build siginfo ---
        let mut info: LinuxSiginfo = unsafe { core::mem::zeroed() };
        info.si_signo = signum;
        info.si_code  = 0; // SI_USER (sent from kernel/userspace)
        info.si_errno = 0;
        info.si_pid   = task.process_id
            .map(|p| p.0 as i32)
            .unwrap_or(0);
        info.si_uid   = 0;

        // --- Step 4: Build ucontext ---
        let saved_mask = task.signal_mask;
        let ctx = match &task.context {
            crate::hal::abstractions::CpuContext::X86_64(c) => c,
            _ => return Err("signal: context arch mismatch"),
        };
        let mctx = LinuxMContext {
            r8:      ctx.r8,
            r9:      ctx.r9,
            r10:     ctx.r10,
            r11:     ctx.r11,
            r12:     ctx.r12,
            r13:     ctx.r13,
            r14:     ctx.r14,
            r15:     ctx.r15,
            rdi:     ctx.rdi,
            rsi:     ctx.rsi,
            rbp:     ctx.rbp,
            rbx:     ctx.rbx,
            rdx:     ctx.rdx,
            rax:     ctx.rax,
            rcx:     ctx.rcx,
            rsp:     ctx.rsp,
            rip:     ctx.rip,
            eflags:  ctx.rflags,
            cs:      ctx.cs as u16,
            gs:      0,
            fs:      0,
            ss:      ctx.ss as u16,
            err:     0,
            trapno:  0,
            oldmask: saved_mask,
            cr2:     0,
            fpstate: 0,
            __reserved1: [0; 8],
        };

        let alt_ss = task.signal_stack.map(|ss| LinuxStackT {
            ss_sp:    ss.ss_sp,
            ss_flags: ss.ss_flags,
            ss_size:  ss.ss_size,
        }).unwrap_or_default();

        let uctx = LinuxUContext {
            flags:    0,
            link:     0,
            stack:    alt_ss,
            mcontext: mctx,
            sigmask:  saved_mask,
        };

        let restorer = action.restorer;
        if restorer == 0 {
            // Without a restorer, signal return is impossible - reject
            return Err("signal: no sa_restorer set; cannot inject frame safely");
        }

        // --- Step 5: Assemble and write frame ---
        let frame = RtSigFrame {
            pretcode: restorer,
            sig: signum,
            _pad: 0,
            pinfo: info_vaddr,
            puc:  uc_vaddr,
            info,
            uc: uctx,
        };

        // Write frame to user stack via physical address translation
        unsafe {
            let phys = translate_user_vaddr(task.page_table_root, frame_vaddr)?;
            let hhdm  = crate::hal::hhdm_offset().unwrap_or(0);
            let kptr  = (phys + hhdm) as *mut RtSigFrame;
            core::ptr::write_unaligned(kptr, frame);
        }

        // --- Step 6: Update task registers ---
        // Calling convention: handler(signum, *siginfo, *ucontext)
        task.context.set_arg_register_0(signum as u64);
        task.context.set_arg_register_1(info_vaddr);
        task.context.set_arg_register_2(uc_vaddr); // set rdx/third arg
        task.context.set_return_register(0);
        task.context.set_stack_pointer(sp);

        // Set instruction pointer to handler
        task.context.set_instruction_pointer(action.handler.map(|h| h as u64).unwrap_or(0));

        // Block signal during handler execution (add signum to mask)
        task.signal_mask |= 1u64 << (signum as u64 - 1);
        // Also block sa_mask
        task.signal_mask |= action.mask;

        crate::klog_trace!(
            "signal: frame injected sig={} handler={:#x} restorer={:#x} sp={:#x} tid=?",
            signum,
            task.context.instruction_pointer(),
            restorer,
            sp,
        );

        Ok(sp)
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        Err("Signal injection only implemented for x86_64")
    }
}

/// Translate user virtual address → physical address via task's page table.
fn translate_user_vaddr(cr3: u64, vaddr: u64) -> Result<u64, &'static str> {
    use x86_64::structures::paging::Translate;
    use x86_64::VirtAddr;

    let hhdm = crate::hal::hhdm_offset().unwrap_or(0);
    let l4_ptr = (cr3 + hhdm) as *mut x86_64::structures::paging::PageTable;
    let l4 = unsafe { &mut *l4_ptr };
    let mapper = unsafe {
        x86_64::structures::paging::OffsetPageTable::new(l4, VirtAddr::new(hhdm))
    };
    mapper.translate_addr(VirtAddr::new(vaddr))
        .map(|p| p.as_u64())
        .ok_or("User stack page not mapped")
}
