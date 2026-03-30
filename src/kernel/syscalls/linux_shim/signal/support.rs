#![allow(dead_code)]

use super::*;
#[cfg(not(feature = "linux_compat"))]
use crate::kernel::syscalls::linux_shim::util::{read_user_pod, write_user_pod};
#[cfg(not(feature = "linux_compat"))]
use crate::kernel::syscalls::with_user_write_bytes;

#[cfg(not(feature = "linux_compat"))]
#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(super) struct LinuxKSigActionCompat {
    pub(super) sa_handler: u64,
    pub(super) sa_flags: u64,
    pub(super) sa_restorer: u64,
    pub(super) sa_mask: u64,
}

#[cfg(not(feature = "linux_compat"))]
#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(super) struct LinuxSigaltstackCompat {
    pub(super) ss_sp: u64,
    pub(super) ss_flags: i32,
    pub(super) ss_size: u64,
}

#[cfg(all(not(feature = "linux_compat"), target_arch = "x86_64"))]
#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(super) struct LinuxMContextCompat {
    pub(super) r8: u64,
    pub(super) r9: u64,
    pub(super) r10: u64,
    pub(super) r11: u64,
    pub(super) r12: u64,
    pub(super) r13: u64,
    pub(super) r14: u64,
    pub(super) r15: u64,
    pub(super) rdi: u64,
    pub(super) rsi: u64,
    pub(super) rbp: u64,
    pub(super) rbx: u64,
    pub(super) rdx: u64,
    pub(super) rax: u64,
    pub(super) rcx: u64,
    pub(super) rsp: u64,
    pub(super) rip: u64,
    pub(super) eflags: u64,
    pub(super) cs: u16,
    pub(super) gs: u16,
    pub(super) fs: u16,
    pub(super) ss: u16,
    pub(super) err: u64,
    pub(super) trapno: u64,
    pub(super) oldmask: u64,
    pub(super) cr2: u64,
    pub(super) fpstate: u64,
    pub(super) __reserved1: [u64; 8],
}

#[cfg(all(not(feature = "linux_compat"), target_arch = "x86_64"))]
#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(super) struct LinuxUContextCompat {
    pub(super) flags: u64,
    pub(super) link: u64,
    pub(super) stack: LinuxSigaltstackCompat,
    pub(super) mcontext: LinuxMContextCompat,
    pub(super) sigmask: u64,
}

#[cfg(not(feature = "linux_compat"))]
const SIGNAL_SET_LEN: usize = core::mem::size_of::<u64>();
#[cfg(not(feature = "linux_compat"))]
const SIGNAL_WAIT_SPIN_BUDGET_COMPAT: usize = 4096;
#[cfg(not(feature = "linux_compat"))]
const LINUX_SIGINFO_LEN: usize = 128;
#[cfg(not(feature = "linux_compat"))]
const LINUX_SIGINFO_SIGNO_OFFSET: usize = 0;
#[cfg(not(feature = "linux_compat"))]
const LINUX_SIGINFO_PID_OFFSET: usize = 16;

#[cfg(not(feature = "linux_compat"))]
#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(super) struct LinuxTimespecCompat {
    pub(super) tv_sec: i64,
    pub(super) tv_nsec: i64,
}

#[cfg(not(feature = "linux_compat"))]
#[inline(always)]
pub(super) fn current_task_arc_for_signal_shim() -> Result<
    alloc::sync::Arc<crate::kernel::sync::IrqSafeMutex<crate::interfaces::task::KernelTask>>,
    usize,
> {
    let Some(cpu) = (unsafe { crate::kernel::cpu_local::CpuLocal::try_get() }) else {
        return Err(linux_errno(crate::modules::posix_consts::errno::ESRCH));
    };
    let current_tid = cpu.current_task.load(core::sync::atomic::Ordering::Relaxed);
    crate::kernel::task::get_task(crate::interfaces::task::TaskId(current_tid))
        .ok_or_else(|| linux_errno(crate::modules::posix_consts::errno::ESRCH))
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn validate_sigaction_args(signum: usize, sigsetsize: usize) -> Result<(), usize> {
    if sigsetsize != 8 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }
    if signum == 0 || signum > 64 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }
    Ok(())
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn decode_sigprocmask_how(how: usize) -> Result<i32, usize> {
    let how = how as i32;
    match how {
        crate::modules::posix_consts::signal::SIG_BLOCK
        | crate::modules::posix_consts::signal::SIG_UNBLOCK
        | crate::modules::posix_consts::signal::SIG_SETMASK => Ok(how),
        _ => Err(linux_errno(crate::modules::posix_consts::errno::EINVAL)),
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn validate_linux_sigset_size(sigsetsize: usize) -> Result<(), usize> {
    if sigsetsize != SIGNAL_SET_LEN {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }
    Ok(())
}

#[cfg(not(feature = "linux_compat"))]
#[inline]
pub(super) fn linux_unblockable_signal_mask() -> u64 {
    let sigkill_bit = 1u64 << ((linux::SIGKILL as u64).saturating_sub(1));
    let sigstop_bit = 1u64 << ((linux::SIGSTOP as u64).saturating_sub(1));
    sigkill_bit | sigstop_bit
}

#[cfg(not(feature = "linux_compat"))]
#[inline]
pub(super) fn sanitize_linux_sigmask(mask: u64) -> u64 {
    mask & !linux_unblockable_signal_mask()
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn should_write_signal_set(ptr: usize) -> bool {
    ptr != 0
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn write_signal_set(ptr: usize, value: u64) -> Result<(), usize> {
    write_user_pod(ptr, &value)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn read_signal_set(ptr: usize) -> Result<u64, usize> {
    read_user_pod(ptr)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn read_signal_wait_timeout_spin_budget(timeout_ptr: usize) -> Result<Option<usize>, usize> {
    if timeout_ptr == 0 {
        return Ok(None);
    }

    let timeout: LinuxTimespecCompat = read_user_pod(timeout_ptr)?;
    if timeout.tv_sec < 0 || timeout.tv_nsec < 0 || timeout.tv_nsec >= 1_000_000_000 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }

    let total_ns = (timeout.tv_sec as u128)
        .saturating_mul(1_000_000_000u128)
        .saturating_add(timeout.tv_nsec as u128);
    if total_ns == 0 {
        return Ok(Some(0));
    }

    let slice_ns = crate::generated_consts::TIME_SLICE_NS as u128;
    let ticks = if slice_ns == 0 {
        total_ns
    } else {
        (total_ns + slice_ns - 1) / slice_ns
    };

    Ok(Some(core::cmp::min(
        ticks as usize,
        SIGNAL_WAIT_SPIN_BUDGET_COMPAT,
    )))
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn write_signal_wait_siginfo(
    siginfo_ptr: usize,
    signum: i32,
    sender_pid: usize,
) -> Result<(), usize> {
    if siginfo_ptr == 0 {
        return Ok(());
    }

    with_user_write_bytes(siginfo_ptr, LINUX_SIGINFO_LEN, |dst| {
        dst.fill(0);
        dst[LINUX_SIGINFO_SIGNO_OFFSET..LINUX_SIGINFO_SIGNO_OFFSET + 4]
            .copy_from_slice(&(signum as u32).to_ne_bytes());
        dst[LINUX_SIGINFO_PID_OFFSET..LINUX_SIGINFO_PID_OFFSET + 4]
            .copy_from_slice(&(sender_pid as u32).to_ne_bytes());
        0
    })
    .map(|_| ())
    .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
}

#[cfg(not(feature = "linux_compat"))]
#[inline]
pub(super) fn first_signal_from_mask(mask: u64) -> Option<i32> {
    if mask == 0 {
        return None;
    }
    let signum = (mask.trailing_zeros() as i32) + 1;
    if (1..=64).contains(&signum) {
        Some(signum)
    } else {
        None
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn validate_sigaltstack_flags_and_size(
    value: &LinuxSigaltstackCompat,
) -> Result<(), usize> {
    let allowed_flags = linux::SS_DISABLE as i32;
    if (value.ss_flags & !allowed_flags) != 0 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }
    if (value.ss_flags & (linux::SS_DISABLE as i32)) == 0 && value.ss_size == 0 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }
    Ok(())
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn write_zeroed_sigaction(oldact: usize) -> Result<(), usize> {
    write_sigaction(oldact, &LinuxKSigActionCompat::default())
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn write_sigaction(ptr: usize, value: &LinuxKSigActionCompat) -> Result<(), usize> {
    write_user_pod(ptr, value)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn read_sigaction(ptr: usize) -> Result<LinuxKSigActionCompat, usize> {
    read_user_pod(ptr)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn write_sigaltstack(ptr: usize, value: &LinuxSigaltstackCompat) -> Result<(), usize> {
    write_user_pod(ptr, value)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn read_sigaltstack(ptr: usize) -> Result<LinuxSigaltstackCompat, usize> {
    read_user_pod(ptr)
}

#[cfg(all(not(feature = "linux_compat"), target_arch = "x86_64"))]
pub(super) fn read_ucontext(ptr: usize) -> Result<LinuxUContextCompat, usize> {
    read_user_pod(ptr)
}

#[cfg(all(test, not(feature = "linux_compat")))]
mod tests {
    use super::*;

    #[test_case]
    fn sigaction_arg_validator_accepts_valid_inputs() {
        assert_eq!(validate_sigaction_args(1, 8), Ok(()));
        assert_eq!(validate_sigaction_args(64, 8), Ok(()));
    }

    #[test_case]
    fn linux_sigset_size_validator_accepts_u64_and_rejects_other_sizes() {
        assert_eq!(
            validate_linux_sigset_size(core::mem::size_of::<u64>()),
            Ok(())
        );
        assert_eq!(
            validate_linux_sigset_size(core::mem::size_of::<u32>()),
            Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
        );
    }

    #[test_case]
    fn signal_set_writer_helper_matches_null_pointer_convention() {
        assert!(!should_write_signal_set(0));
        assert!(should_write_signal_set(8));
    }

    #[test_case]
    fn signal_set_helpers_roundtrip_mask_bytes() {
        let mut raw = [0u8; core::mem::size_of::<u64>()];
        assert_eq!(
            write_signal_set(raw.as_mut_ptr() as usize, 0x55AA_F0F0_1234_5678),
            Ok(())
        );
        assert_eq!(
            read_signal_set(raw.as_ptr() as usize),
            Ok(0x55AA_F0F0_1234_5678)
        );
    }

    #[test_case]
    fn signal_set_helpers_report_invalid_pointers() {
        assert_eq!(
            write_signal_set(0, 1),
            Err(linux_errno(crate::modules::posix_consts::errno::EFAULT))
        );
        assert_eq!(
            read_signal_set(0),
            Err(linux_errno(crate::modules::posix_consts::errno::EFAULT))
        );
    }

    #[test_case]
    fn sigprocmask_how_decoder_accepts_supported_modes() {
        assert_eq!(
            decode_sigprocmask_how(crate::modules::posix_consts::signal::SIG_BLOCK as usize),
            Ok(crate::modules::posix_consts::signal::SIG_BLOCK)
        );
        assert_eq!(
            decode_sigprocmask_how(crate::modules::posix_consts::signal::SIG_UNBLOCK as usize),
            Ok(crate::modules::posix_consts::signal::SIG_UNBLOCK)
        );
        assert_eq!(
            decode_sigprocmask_how(crate::modules::posix_consts::signal::SIG_SETMASK as usize),
            Ok(crate::modules::posix_consts::signal::SIG_SETMASK)
        );
    }

    #[test_case]
    fn sigmask_sanitizer_clears_unblockable_signals() {
        let kill_bit = 1u64 << ((linux::SIGKILL as u64).saturating_sub(1));
        let stop_bit = 1u64 << ((linux::SIGSTOP as u64).saturating_sub(1));
        let keep_bit = 1u64;

        let input = kill_bit | stop_bit | keep_bit;
        let sanitized = sanitize_linux_sigmask(input);

        assert_eq!(sanitized & kill_bit, 0);
        assert_eq!(sanitized & stop_bit, 0);
        assert_ne!(sanitized & keep_bit, 0);
    }

    #[test_case]
    fn sigaltstack_flag_validator_rejects_unknown_flags_and_zero_sized_enabled_stack() {
        let invalid_flags = LinuxSigaltstackCompat {
            ss_sp: 0x1000,
            ss_flags: 0x20,
            ss_size: 0x4000,
        };
        let invalid_size = LinuxSigaltstackCompat {
            ss_sp: 0x1000,
            ss_flags: 0,
            ss_size: 0,
        };
        let disabled = LinuxSigaltstackCompat {
            ss_sp: 0,
            ss_flags: linux::SS_DISABLE as i32,
            ss_size: 0,
        };
        assert_eq!(
            validate_sigaltstack_flags_and_size(&invalid_flags),
            Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
        );
        assert_eq!(
            validate_sigaltstack_flags_and_size(&invalid_size),
            Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
        );
        assert_eq!(validate_sigaltstack_flags_and_size(&disabled), Ok(()));
    }

    #[test_case]
    fn sigaltstack_helpers_roundtrip_bytes() {
        let expected = LinuxSigaltstackCompat {
            ss_sp: 0x1234,
            ss_flags: 0,
            ss_size: 0x4000,
        };
        let mut raw = [0u8; core::mem::size_of::<LinuxSigaltstackCompat>()];
        assert_eq!(
            write_sigaltstack(raw.as_mut_ptr() as usize, &expected),
            Ok(())
        );
        assert_eq!(read_sigaltstack(raw.as_ptr() as usize), Ok(expected));
    }

    #[test_case]
    fn sigaltstack_helpers_report_invalid_pointers() {
        let expected = LinuxSigaltstackCompat {
            ss_sp: 0,
            ss_flags: linux::SS_DISABLE as i32,
            ss_size: 0,
        };
        assert_eq!(
            write_sigaltstack(0, &expected),
            Err(linux_errno(crate::modules::posix_consts::errno::EFAULT))
        );
        assert_eq!(
            read_sigaltstack(0),
            Err(linux_errno(crate::modules::posix_consts::errno::EFAULT))
        );
    }

    #[test_case]
    fn signal_wait_timeout_budget_validator_rejects_invalid_timespec() {
        let invalid_nsec = LinuxTimespecCompat {
            tv_sec: 0,
            tv_nsec: 1_000_000_000,
        };
        assert_eq!(
            read_signal_wait_timeout_spin_budget((&invalid_nsec as *const LinuxTimespecCompat) as usize),
            Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
        );

        let invalid_sec = LinuxTimespecCompat {
            tv_sec: -1,
            tv_nsec: 0,
        };
        assert_eq!(
            read_signal_wait_timeout_spin_budget((&invalid_sec as *const LinuxTimespecCompat) as usize),
            Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
        );
    }

    #[test_case]
    fn signal_wait_timeout_budget_handles_null_and_zero_timeout() {
        assert_eq!(read_signal_wait_timeout_spin_budget(0), Ok(None));

        let zero = LinuxTimespecCompat {
            tv_sec: 0,
            tv_nsec: 0,
        };
        assert_eq!(
            read_signal_wait_timeout_spin_budget((&zero as *const LinuxTimespecCompat) as usize),
            Ok(Some(0))
        );
    }

    #[test_case]
    fn first_signal_from_mask_returns_lowest_pending_bit() {
        assert_eq!(first_signal_from_mask(0), None);
        assert_eq!(first_signal_from_mask(1u64 << 3), Some(4));
        assert_eq!(first_signal_from_mask((1u64 << 7) | (1u64 << 1)), Some(2));
    }

    #[test_case]
    fn signal_wait_siginfo_writer_encodes_expected_fields() {
        let mut raw = [0u8; LINUX_SIGINFO_LEN];
        assert_eq!(
            write_signal_wait_siginfo(raw.as_mut_ptr() as usize, 15, 1234),
            Ok(())
        );

        let signo = u32::from_ne_bytes(
            raw[LINUX_SIGINFO_SIGNO_OFFSET..LINUX_SIGINFO_SIGNO_OFFSET + 4]
                .try_into()
                .expect("signo bytes"),
        );
        let pid = u32::from_ne_bytes(
            raw[LINUX_SIGINFO_PID_OFFSET..LINUX_SIGINFO_PID_OFFSET + 4]
                .try_into()
                .expect("pid bytes"),
        );

        assert_eq!(signo, 15);
        assert_eq!(pid, 1234);
        assert!(raw[20..].iter().all(|byte| *byte == 0));
    }
}
