use super::super::*;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LinuxTimespecCompat {
    tv_sec: i64,
    tv_nsec: i64,
}

#[inline]
fn linux_unblockable_signal_mask() -> u64 {
    let sigkill_bit = 1u64 << ((linux::SIGKILL as u64).saturating_sub(1));
    let sigstop_bit = 1u64 << ((linux::SIGSTOP as u64).saturating_sub(1));
    sigkill_bit | sigstop_bit
}

#[inline]
fn sanitize_linux_sigmask(mask: u64) -> u64 {
    mask & !linux_unblockable_signal_mask()
}

fn timeout_spin_budget(timeout_ptr: usize) -> Result<Option<usize>, usize> {
    if timeout_ptr == 0 {
        return Ok(None);
    }

    let timeout = match UserPtr::<LinuxTimespecCompat>::new(timeout_ptr).read() {
        Ok(v) => v,
        Err(e) => return Err(e),
    };

    if timeout.tv_sec < 0 || timeout.tv_nsec < 0 || timeout.tv_nsec >= 1_000_000_000 {
        return Err(linux_inval());
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

    Ok(Some(core::cmp::min(ticks as usize, 4096)))
}

fn write_wait_siginfo(siginfo_ptr: usize, signum: i32, sender_pid: usize) -> Result<(), usize> {
    if siginfo_ptr == 0 {
        return Ok(());
    }

    crate::kernel::syscalls::with_user_write_bytes(siginfo_ptr, 128, |dst| {
        dst.fill(0);
        dst[0..4].copy_from_slice(&(signum as u32).to_ne_bytes());
        dst[16..20].copy_from_slice(&(sender_pid as u32).to_ne_bytes());
        0
    })
    .map(|_| ())
    .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
}

fn decode_required_waitset(set: UserPtr<u64>, sigsetsize: usize) -> Result<u64, usize> {
    if sigsetsize != linux::SIGSET_SIZE {
        return Err(linux_inval());
    }
    if set.is_null() {
        return Err(linux_errno(crate::modules::posix_consts::errno::EFAULT));
    }

    let waitset = sanitize_linux_sigmask(set.read()?);
    if waitset == 0 {
        return Err(linux_inval());
    }

    Ok(waitset)
}

#[inline]
fn complete_wait_signum(siginfo_ptr: usize, signum: i32) -> usize {
    match write_wait_siginfo(
        siginfo_ptr,
        signum,
        crate::modules::posix::signal::current_pid_pub(),
    ) {
        Ok(()) => signum as usize,
        Err(e) => e,
    }
}

#[inline]
fn map_wait_errno(errno: i32, timeout_errno: i32) -> usize {
    if errno == crate::modules::posix_consts::errno::ETIMEDOUT {
        linux_errno(timeout_errno)
    } else {
        linux_errno(errno)
    }
}

pub fn sys_linux_rt_sigprocmask(
    how: usize,
    set: UserPtr<u64>,
    oldset: UserPtr<u64>,
    sigsetsize: usize,
) -> usize {
    crate::require_posix_signal!((how, set, oldset, sigsetsize) => {
        use crate::modules::posix::signal::{self, SigmaskHow};

        if sigsetsize != linux::SIGSET_SIZE {
            return linux_inval();
        }

        let how_enum = match how as i32 {
            crate::modules::posix_consts::signal::SIG_BLOCK => SigmaskHow::Block,
            crate::modules::posix_consts::signal::SIG_SETMASK => SigmaskHow::SetMask,
            crate::modules::posix_consts::signal::SIG_UNBLOCK => SigmaskHow::Unblock,
            _ => return linux_inval(),
        };

        // Read new set from userspace
        let new_set = if !set.is_null() {
            match set.read() {
                Ok(v) => Some(v),
                Err(e) => return e,
            }
        } else {
            None
        };

        match signal::sigprocmask(how_enum, new_set) {
            Ok(old_mask) => {
                if !oldset.is_null() {
                    if let Err(e) = oldset.write(&old_mask) { return e; }
                }
                0
            }
            Err(_) => linux_inval(),
        }
    })
}
pub fn sys_linux_rt_sigpending(set: UserPtr<u64>, sigsetsize: usize) -> usize {
    crate::require_posix_signal!((set, sigsetsize) => {
        if sigsetsize != linux::SIGSET_SIZE {
                    return linux_inval();
                }
                if !set.is_null() {
                    let pending = crate::modules::posix::signal::sigpending();
                    if let Err(e) = set.write(&pending) { return e; }
                }
                0
    })
}

pub fn sys_linux_rt_sigsuspend(unmask: UserPtr<u64>, sigsetsize: usize) -> usize {
    crate::require_posix_signal!((unmask, sigsetsize) => {
        if sigsetsize != linux::SIGSET_SIZE {
                    return linux_inval();
                }
                let mask = if !unmask.is_null() {
                    match unmask.read() {
                        Ok(v) => sanitize_linux_sigmask(v),
                        Err(e) => return e,
                    }
                } else {
                    0
                };

                match crate::modules::posix::signal::sigsuspend(mask) {
                    Ok(_) => linux_errno(crate::modules::posix_consts::errno::EINTR),
                    Err(e) => map_wait_errno(
                        e.code(),
                        crate::modules::posix_consts::errno::EINTR,
                    ),
                }
    })
}

pub fn sys_linux_rt_sigwaitinfo(set: UserPtr<u64>, siginfo_ptr: usize, sigsetsize: usize) -> usize {
    crate::require_posix_signal!((set, siginfo_ptr, sigsetsize) => {
        let waitset = match decode_required_waitset(set, sigsetsize) {
            Ok(v) => v,
            Err(e) => return e,
        };

        match crate::modules::posix::signal::sigwaitinfo(waitset) {
            Ok(signum) => complete_wait_signum(siginfo_ptr, signum),
            Err(e) => map_wait_errno(
                e.code(),
                crate::modules::posix_consts::errno::EINTR,
            ),
        }
    })
}

pub fn sys_linux_rt_sigtimedwait(
    set: UserPtr<u64>,
    siginfo_ptr: usize,
    timeout_ptr: usize,
    sigsetsize: usize,
) -> usize {
    crate::require_posix_signal!((set, siginfo_ptr, timeout_ptr, sigsetsize) => {
        let waitset = match decode_required_waitset(set, sigsetsize) {
            Ok(v) => v,
            Err(e) => return e,
        };

        let budget = match timeout_spin_budget(timeout_ptr) {
            Ok(v) => v,
            Err(e) => return e,
        };

        if budget.is_none() {
            return sys_linux_rt_sigwaitinfo(set, siginfo_ptr, sigsetsize);
        }

        match crate::modules::posix::signal::sigtimedwait(waitset, budget.unwrap_or(0)) {
            Ok(Some(signum)) => complete_wait_signum(siginfo_ptr, signum),
            Ok(None) => linux_errno(crate::modules::posix_consts::errno::EAGAIN),
            Err(e) => map_wait_errno(
                e.code(),
                crate::modules::posix_consts::errno::EAGAIN,
            ),
        }
    })
}

pub fn sys_linux_pause() -> usize {
    crate::require_posix_signal!(() => {
        match crate::modules::posix::signal::pause() {
                    Ok(_) => linux_errno(crate::modules::posix_consts::errno::EINTR),
                    Err(e) => map_wait_errno(
                        e.code(),
                        crate::modules::posix_consts::errno::EINTR,
                    ),
                }
    })
}
