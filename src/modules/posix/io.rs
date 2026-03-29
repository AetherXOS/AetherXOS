use crate::modules::vfs::File;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;

use crate::modules::posix::PosixErrno;

#[derive(Debug, Clone, Copy)]
pub struct PosixPollFd {
    pub fd: u32,
    pub events: u16,
    pub revents: u16,
}

impl PosixPollFd {
    pub const fn new(fd: u32, events: u16) -> Self {
        Self {
            fd,
            events,
            revents: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PosixSelectResult {
    pub readable: Vec<u32>,
    pub writable: Vec<u32>,
    pub exceptional: Vec<u32>,
}

fn poll_one(fd: u32, events: u16) -> Result<u16, PosixErrno> {
    let table = crate::modules::posix::fs::FILE_TABLE.lock();
    let desc = table.get(&fd).ok_or(PosixErrno::BadFileDescriptor)?;
    let handle = desc.file.handle.lock();
    let revents = handle.poll_events();
    Ok((revents.bits() as u16) & events)
}

pub fn poll_mixed(fds: &mut [PosixPollFd], retries: usize) -> Result<usize, PosixErrno> {
    for _ in 0..=retries {
        let mut ready = 0usize;
        for fd in fds.iter_mut() {
            fd.revents = poll_one(fd.fd, fd.events)?;
            if fd.revents != 0 {
                ready += 1;
            }
        }

        if ready > 0 {
            return Ok(ready);
        }
        crate::kernel::rt_preemption::request_forced_reschedule();
    }
    Ok(0)
}

pub fn poll_mixed_timespec(
    fds: &mut [PosixPollFd],
    timeout: crate::modules::posix::time::PosixTimespec,
) -> Result<usize, PosixErrno> {
    if timeout.sec < 0 || timeout.nsec < 0 || timeout.nsec >= 1_000_000_000 {
        return Err(PosixErrno::Invalid);
    }

    let total_ns = (timeout.sec as u128)
        .saturating_mul(1_000_000_000u128)
        .saturating_add(timeout.nsec as u128);
    let retries = if total_ns == 0 {
        0
    } else {
        let slice_ns = crate::generated_consts::TIME_SLICE_NS as u128;
        if slice_ns == 0 {
            total_ns as usize
        } else {
            ((total_ns + slice_ns - 1) / slice_ns) as usize
        }
    };

    poll_mixed(fds, retries)
}

pub fn select_mixed(
    read_fds: &[u32],
    write_fds: &[u32],
    except_fds: &[u32],
    retries: usize,
) -> Result<PosixSelectResult, PosixErrno> {
    let mut merged: BTreeMap<u32, u16> = BTreeMap::new();
    for fd in read_fds {
        merged
            .entry(*fd)
            .and_modify(|ev| *ev |= crate::modules::posix_consts::net::POLLIN)
            .or_insert(crate::modules::posix_consts::net::POLLIN);
    }
    for fd in write_fds {
        merged
            .entry(*fd)
            .and_modify(|ev| *ev |= crate::modules::posix_consts::net::POLLOUT)
            .or_insert(crate::modules::posix_consts::net::POLLOUT);
    }
    for fd in except_fds {
        merged
            .entry(*fd)
            .and_modify(|ev| *ev |= crate::modules::posix_consts::net::POLLERR)
            .or_insert(crate::modules::posix_consts::net::POLLERR);
    }

    let mut pollfds: Vec<PosixPollFd> = merged
        .iter()
        .map(|(fd, events)| PosixPollFd::new(*fd, *events))
        .collect();
    let _ = poll_mixed(&mut pollfds, retries)?;

    let mut out = PosixSelectResult {
        readable: Vec::new(),
        writable: Vec::new(),
        exceptional: Vec::new(),
    };

    for p in &pollfds {
        if (p.revents & crate::modules::posix_consts::net::POLLIN) != 0 {
            out.readable.push(p.fd);
        }
        if (p.revents & crate::modules::posix_consts::net::POLLOUT) != 0 {
            out.writable.push(p.fd);
        }
        if (p.revents & crate::modules::posix_consts::net::POLLERR) != 0 {
            out.exceptional.push(p.fd);
        }
    }

    Ok(out)
}

pub fn select_mixed_timespec(
    read_fds: &[u32],
    write_fds: &[u32],
    except_fds: &[u32],
    timeout: crate::modules::posix::time::PosixTimespec,
) -> Result<PosixSelectResult, PosixErrno> {
    if timeout.sec < 0 || timeout.nsec < 0 || timeout.nsec >= 1_000_000_000 {
        return Err(PosixErrno::Invalid);
    }

    let total_ns = (timeout.sec as u128)
        .saturating_mul(1_000_000_000u128)
        .saturating_add(timeout.nsec as u128);
    let retries = if total_ns == 0 {
        0
    } else {
        let slice_ns = crate::generated_consts::TIME_SLICE_NS as u128;
        if slice_ns == 0 {
            total_ns as usize
        } else {
            ((total_ns + slice_ns - 1) / slice_ns) as usize
        }
    };

    select_mixed(read_fds, write_fds, except_fds, retries)
}

// ── EventFD Implementation ──────────────────────────────────────────────────

struct EventFd {
    value: Mutex<u64>,
    semaphore_mode: bool,
    nonblock: bool,
}

impl File for EventFd {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        if buf.len() < 8 {
            return Err("buffer too small");
        }
        let mut val = self.value.lock();
        if *val == 0 {
            if self.nonblock {
                return Err("already empty");
            }
            // In a real kernel, we would block here.
            return Err("already empty");
        }

        let read_val = if self.semaphore_mode {
            *val -= 1;
            1
        } else {
            let res = *val;
            *val = 0;
            res
        };

        buf[..8].copy_from_slice(&read_val.to_le_bytes());
        Ok(8)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, &'static str> {
        if buf.len() < 8 {
            return Err("buffer too small");
        }
        let mut input = [0u8; 8];
        input.copy_from_slice(&buf[..8]);
        let add_val = u64::from_le_bytes(input);

        if add_val == u64::MAX {
            return Err("invalid value");
        }

        let mut val = self.value.lock();
        if u64::MAX - *val <= add_val {
            return Err("overflow");
        }
        *val += add_val;
        Ok(8)
    }

    fn poll_events(&self) -> crate::modules::vfs::PollEvents {
        let val = self.value.lock();
        let mut ev = crate::modules::vfs::PollEvents::empty();
        if *val > 0 {
            ev |= crate::modules::vfs::PollEvents::IN;
        }
        if *val < u64::MAX - 1 {
            ev |= crate::modules::vfs::PollEvents::OUT;
        }
        ev
    }

    fn as_any(&self) -> &dyn core::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }
}

pub fn eventfd_create_errno(initval: u32, flags: i32) -> Result<u32, PosixErrno> {
    let sem = (flags & 0x1) != 0; // EFD_SEMAPHORE
    let nonblock = (flags & 0x800) != 0; // O_NONBLOCK

    let evfd = EventFd {
        value: Mutex::new(initval as u64),
        semaphore_mode: sem,
        nonblock,
    };

    let fd = crate::modules::posix::fs::register_handle(
        0, // common fs
        alloc::format!("eventfd:{}", initval),
        Arc::new(Mutex::new(evfd)),
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

pub fn eventfd_set_nonblock(fd: u32, enabled: bool) -> Result<(), PosixErrno> {
    let table = crate::modules::posix::fs::FILE_TABLE.lock();
    let desc = table.get(&fd).ok_or(PosixErrno::BadFileDescriptor)?;
    let mut handle = desc.file.handle.lock();
    if let Some(eventfd) = handle.as_any_mut().downcast_mut::<EventFd>() {
        eventfd.nonblock = enabled;
        Ok(())
    } else {
        Err(PosixErrno::BadFileDescriptor)
    }
}
