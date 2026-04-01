use super::*;

fn take_unmasked_pending(pid: usize, mask: SigSet) -> Option<i32> {
    let mut pending = SIGNAL_PENDING.lock();
    let set = pending.get_mut(&pid)?;
    let mut found = None;
    for signum in set.iter().copied() {
        let Some(bit) = sigbit(signum) else {
            continue;
        };
        if (mask & bit) == 0 {
            found = Some(signum);
            break;
        }
    }
    if let Some(signum) = found {
        set.remove(&signum);
        if set.is_empty() {
            pending.remove(&pid);
        }
        Some(signum)
    } else {
        None
    }
}

fn take_matching_pending(pid: usize, wanted: SigSet) -> Option<i32> {
    let mut pending = SIGNAL_PENDING.lock();
    let set = pending.get_mut(&pid)?;

    let mut found = None;
    for signum in set.iter().copied() {
        let Some(bit) = sigbit(signum) else {
            continue;
        };
        if (wanted & bit) != 0 {
            found = Some(signum);
            break;
        }
    }

    if let Some(signum) = found {
        set.remove(&signum);
        if set.is_empty() {
            pending.remove(&pid);
        }
        Some(signum)
    } else {
        None
    }
}

pub fn sigwaitinfo(waitset: SigSet) -> Result<i32, PosixErrno> {
    if waitset == 0 {
        return Err(PosixErrno::Invalid);
    }

    let pid = current_pid();
    if pid == 0 {
        return Err(PosixErrno::Invalid);
    }

    for _ in 0..SIGNAL_WAIT_SPIN_BUDGET {
        if let Some(signum) = take_matching_pending(pid, waitset) {
            return Ok(signum);
        }
        crate::kernel::rt_preemption::request_forced_reschedule();
    }

    Err(PosixErrno::TimedOut)
}

#[inline(always)]
pub fn sigwait(waitset: SigSet) -> Result<i32, PosixErrno> {
    sigwaitinfo(waitset)
}

#[cfg(feature = "posix_time")]
#[inline(always)]
fn spin_budget_from_timespec(
    timeout: crate::modules::posix::time::PosixTimespec,
) -> Result<usize, PosixErrno> {
    if timeout.sec < 0 || timeout.nsec < 0 || timeout.nsec >= 1_000_000_000 {
        return Err(PosixErrno::Invalid);
    }

    let total_ns = (timeout.sec as u128)
        .saturating_mul(1_000_000_000u128)
        .saturating_add(timeout.nsec as u128);
    if total_ns == 0 {
        return Ok(0);
    }

    let slice_ns = crate::generated_consts::TIME_SLICE_NS as u128;
    let ticks = if slice_ns == 0 {
        total_ns
    } else {
        (total_ns + slice_ns - 1) / slice_ns
    };

    Ok(core::cmp::min(ticks as usize, SIGNAL_WAIT_SPIN_BUDGET))
}

pub fn sigtimedwait(waitset: SigSet, spin_budget: usize) -> Result<Option<i32>, PosixErrno> {
    if waitset == 0 {
        return Err(PosixErrno::Invalid);
    }
    if spin_budget == 0 {
        return Ok(None);
    }

    let pid = current_pid();
    if pid == 0 {
        return Err(PosixErrno::Invalid);
    }

    for _ in 0..spin_budget {
        if let Some(signum) = take_matching_pending(pid, waitset) {
            return Ok(Some(signum));
        }
        crate::kernel::rt_preemption::request_forced_reschedule();
    }

    Ok(None)
}

#[cfg(feature = "posix_time")]
pub fn sigtimedwait_ts(
    waitset: SigSet,
    timeout: crate::modules::posix::time::PosixTimespec,
) -> Result<Option<i32>, PosixErrno> {
    let budget = spin_budget_from_timespec(timeout)?;
    sigtimedwait(waitset, budget)
}

pub fn sigqueue(pid: usize, signum: i32) -> Result<(), PosixErrno> {
    if pid == 0 {
        return Err(PosixErrno::Invalid);
    }
    let _ = sigbit(signum).ok_or(PosixErrno::Invalid)?;

    SIGNAL_PENDING
        .lock()
        .entry(pid)
        .or_insert_with(BTreeSet::new)
        .insert(signum);
    SIGNAL_PENDING_QUEUED.fetch_add(1, Ordering::Relaxed);
    Ok(())
}

pub fn sigsuspend(temp_mask: SigSet) -> Result<i32, PosixErrno> {
    let pid = current_pid();
    if pid == 0 {
        return Err(PosixErrno::Invalid);
    }

    let old = sigprocmask(SigmaskHow::SetMask, Some(temp_mask))?;
    for _ in 0..SIGNAL_WAIT_SPIN_BUDGET {
        let mask = read_mask(pid);
        if let Some(signum) = take_unmasked_pending(pid, mask) {
            let _ = sigprocmask(SigmaskHow::SetMask, Some(old));
            return Ok(signum);
        }
        crate::kernel::rt_preemption::request_forced_reschedule();
    }
    let _ = sigprocmask(SigmaskHow::SetMask, Some(old));
    Err(PosixErrno::TimedOut)
}

#[cfg(feature = "posix_time")]
pub fn sigsuspend_timeout(
    temp_mask: SigSet,
    timeout: crate::modules::posix::time::PosixTimespec,
) -> Result<Option<i32>, PosixErrno> {
    let pid = current_pid();
    if pid == 0 {
        return Err(PosixErrno::Invalid);
    }

    let old = sigprocmask(SigmaskHow::SetMask, Some(temp_mask))?;
    let budget = spin_budget_from_timespec(timeout)?;

    if budget == 0 {
        let _ = sigprocmask(SigmaskHow::SetMask, Some(old));
        return Ok(None);
    }

    for _ in 0..budget {
        let mask = read_mask(pid);
        if let Some(signum) = take_unmasked_pending(pid, mask) {
            let _ = sigprocmask(SigmaskHow::SetMask, Some(old));
            return Ok(Some(signum));
        }
        crate::kernel::rt_preemption::request_forced_reschedule();
    }

    let _ = sigprocmask(SigmaskHow::SetMask, Some(old));
    Ok(None)
}

pub fn pause() -> Result<i32, PosixErrno> {
    let pid = current_pid();
    if pid == 0 {
        return Err(PosixErrno::Invalid);
    }
    let current = read_mask(pid);
    sigsuspend(current)
}

pub fn sigaltstack(
    ss: Option<crate::interfaces::task::SignalStack>,
) -> Result<Option<crate::interfaces::task::SignalStack>, PosixErrno> {
    let task_arc = current_task_arc()?;
    let mut task = task_arc.lock();
    let old = task.signal_stack;

    if let Some(stack) = ss {
        let disabling = (stack.ss_flags & crate::modules::posix_consts::signal::SS_DISABLE) != 0;
        if !disabling && stack.ss_size < POSIX_MINSIGSTKSZ {
            return Err(PosixErrno::Invalid);
        }
        task.signal_stack = if disabling { None } else { Some(stack) };
    }

    Ok(old)
}

// ── SignalFD Implementation ─────────────────────────────────────────────────

#[cfg(all(feature = "posix_fs", feature = "vfs"))]
struct SignalFd {
    mask: SigSet,
    nonblock: bool,
}

#[cfg(all(feature = "posix_fs", feature = "vfs"))]
impl File for SignalFd {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        // struct signalfd_siginfo is about 128 bytes.
        if buf.len() < 128 {
            return Err("buffer too small");
        }

        let pid = current_pid();
        if pid == 0 {
            return Err("invalid pid");
        }

        if let Some(signum) = take_matching_pending(pid, self.mask) {
            // In a real implementation we'd fill a struct signalfd_siginfo here.
            // For now we'll just put the signum at the start (le-bytes).
            buf[..4].copy_from_slice(&(signum as u32).to_le_bytes());
            // Fill the rest with zero for now.
            buf[4..128].fill(0);
            return Ok(128);
        }

        if self.nonblock {
            Err("would block")
        } else {
            Ok(0)
        }
    }

    fn write(&mut self, _buf: &[u8]) -> Result<usize, &'static str> {
        Err("signalfd is read-only")
    }

    fn poll_events(&self) -> crate::modules::vfs::PollEvents {
        let pid = current_pid();
        if pid == 0 {
            return crate::modules::vfs::PollEvents::empty();
        }

        let pending = SIGNAL_PENDING.lock();
        if let Some(set) = pending.get(&pid) {
            for signum in set {
                if let Some(bit) = sigbit(*signum) {
                    if (self.mask & bit) != 0 {
                        return crate::modules::vfs::PollEvents::IN;
                    }
                }
            }
        }
        crate::modules::vfs::PollEvents::empty()
    }

    fn as_any(&self) -> &dyn core::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }
}

#[cfg(all(feature = "posix_fs", feature = "vfs"))]
pub fn signalfd_create_errno(mask: SigSet, flags: i32) -> Result<u32, PosixErrno> {
    let _cloexec = (flags & 0x80000) != 0; // O_CLOEXEC
    let nonblock = (flags & 0x800) != 0; // O_NONBLOCK

    let sigfd = SignalFd { mask, nonblock };

    let fd = crate::modules::posix::fs::register_handle(
        0,
        alloc::format!("signalfd:{:x}", mask),
        Arc::new(Mutex::new(sigfd)),
        true,
    );
    if nonblock {
        let _ = crate::modules::posix::fs::fcntl_set_status_flags(
            fd,
            0x2 | crate::modules::posix_consts::net::O_NONBLOCK as u32,
        );
    }
    Ok(fd)
}

#[cfg(not(all(feature = "posix_fs", feature = "vfs")))]
pub fn signalfd_create_errno(mask: SigSet, flags: i32) -> Result<u32, PosixErrno> {
    let _ = (mask, flags);
    Err(PosixErrno::NoSys)
}

#[cfg(all(feature = "posix_fs", feature = "vfs"))]
pub fn signalfd_set_nonblock(fd: u32, enabled: bool) -> Result<(), PosixErrno> {
    let table = crate::modules::posix::fs::FILE_TABLE.lock();
    let desc = table.get(&fd).ok_or(PosixErrno::BadFileDescriptor)?;
    let mut handle = desc.file.handle.lock();
    if let Some(sigfd) = handle.as_any_mut().downcast_mut::<SignalFd>() {
        sigfd.nonblock = enabled;
        Ok(())
    } else {
        Err(PosixErrno::BadFileDescriptor)
    }
}

#[cfg(all(feature = "posix_fs", feature = "vfs"))]
pub fn signalfd_reconfigure_errno(fd: u32, mask: SigSet, flags: i32) -> Result<u32, PosixErrno> {
    let nonblock = (flags & 0x800) != 0; // O_NONBLOCK
    let table = crate::modules::posix::fs::FILE_TABLE.lock();
    let desc = table.get(&fd).ok_or(PosixErrno::BadFileDescriptor)?;
    let mut handle = desc.file.handle.lock();
    if let Some(sigfd) = handle.as_any_mut().downcast_mut::<SignalFd>() {
        sigfd.mask = mask;
        sigfd.nonblock = nonblock;
        drop(handle);
        drop(table);
        let mut status_flags = 0x2u32;
        if nonblock {
            status_flags |= crate::modules::posix_consts::net::O_NONBLOCK as u32;
        }
        let _ = crate::modules::posix::fs::fcntl_set_status_flags(fd, status_flags);
        Ok(fd)
    } else {
        Err(PosixErrno::BadFileDescriptor)
    }
}

#[cfg(not(all(feature = "posix_fs", feature = "vfs")))]
pub fn signalfd_reconfigure_errno(fd: u32, mask: SigSet, flags: i32) -> Result<u32, PosixErrno> {
    let _ = (fd, mask, flags);
    Err(PosixErrno::NoSys)
}

#[cfg(not(all(feature = "posix_fs", feature = "vfs")))]
pub fn signalfd_set_nonblock(fd: u32, enabled: bool) -> Result<(), PosixErrno> {
    let _ = (fd, enabled);
    Err(PosixErrno::NoSys)
}
