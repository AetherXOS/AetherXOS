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
    let revents = poll_one_vfs(fd, crate::modules::vfs::PollEvents::from_bits_truncate(events as u32))?;
    Ok((revents.bits() as u16) & events)
}

pub fn poll_one_vfs(fd: u32, _events: crate::modules::vfs::PollEvents) -> Result<crate::modules::vfs::PollEvents, PosixErrno> {
    let table = crate::modules::posix::fs::FILE_TABLE.lock();
    let desc = table.get(&fd).ok_or(PosixErrno::BadFileDescriptor)?;
    let handle = desc.file.handle.lock();
    Ok(handle.poll_events())
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
    wait_queue: Arc<crate::kernel::sync::WaitQueue>,
}

impl File for EventFd {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        if buf.len() < 8 {
            return Err("buffer too small");
        }
        loop {
            let mut val = self.value.lock();
            if *val > 0 {
                let read_val = if self.semaphore_mode {
                    *val -= 1;
                    1
                } else {
                    let res = *val;
                    *val = 0;
                    res
                };
                
                // If we consumed something and it was previously full, wake up writers
                if *val < u64::MAX - 1 {
                    self.wait_queue.wake_all();
                }

                buf[..8].copy_from_slice(&read_val.to_le_bytes());
                return Ok(8);
            }

            if self.nonblock {
                return Err("EAGAIN");
            }

            drop(val);
            self.wait_queue.wait();
        }
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, &'static str> {
        if buf.len() < 8 {
            return Err("buffer too small");
        }
        let mut input = [0u8; 8];
        input.copy_from_slice(&buf[..8]);
        let add_val = u64::from_le_bytes(input);

        if add_val == u64::MAX {
            return Err("EINVAL");
        }

        loop {
            let mut val = self.value.lock();
            if u64::MAX - *val > add_val {
                *val += add_val;
                self.wait_queue.wake_all();
                return Ok(8);
            }

            if self.nonblock {
                return Err("EAGAIN");
            }

            drop(val);
            self.wait_queue.wait();
        }
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

    fn wait_queue(&self) -> Option<Arc<crate::kernel::sync::WaitQueue>> {
        Some(self.wait_queue.clone())
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
    let cloexec = (flags & 0x80000) != 0; // O_CLOEXEC

    let evfd = alloc::boxed::Box::new(EventFd {
        value: Mutex::new(initval as u64),
        semaphore_mode: sem,
        nonblock,
        wait_queue: Arc::new(crate::kernel::sync::WaitQueue::new()),
    });

    let fs_id = *crate::modules::posix::fs::SHM_FS_ID;
    let fd = crate::modules::posix::fs::register_handle(
        fs_id,
        alloc::format!("eventfd:{}", initval),
        Arc::new(Mutex::new(crate::modules::posix::fs::BoxedFile { inner: evfd })),
        cloexec,
    );

    if nonblock {
        let _ = crate::modules::posix::fs::fcntl_set_status_flags(
            fd,
            crate::modules::posix_consts::net::O_NONBLOCK as u32,
        );
    }
    Ok(fd)
}

pub fn eventfd_set_nonblock(fd: u32, enabled: bool) -> Result<(), PosixErrno> {
    let table = crate::modules::posix::fs::FILE_TABLE.lock();
    let desc = table.get(&fd).ok_or(PosixErrno::BadFileDescriptor)?;
    let mut handle = desc.file.handle.lock();
    
    let res = if let Some(bf) = handle.as_any_mut().downcast_mut::<crate::modules::posix::fs::BoxedFile>() {
        if let Some(eventfd) = bf.inner.as_any_mut().downcast_mut::<EventFd>() {
            eventfd.nonblock = enabled;
            Ok(())
        } else {
            Err(PosixErrno::BadFileDescriptor)
        }
    } else {
        Err(PosixErrno::BadFileDescriptor)
    };
    res
}
