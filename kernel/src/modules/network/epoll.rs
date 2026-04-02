/// epoll / poll / select — I/O event notification framework.
///
/// Implements Linux-compatible event-driven I/O multiplexing:
/// - `epoll_create` / `epoll_ctl` / `epoll_wait`
/// - `poll` / `ppoll`
/// - `select` / `pselect6`
///
/// ## Architecture
///
/// An `EpollInstance` maintains a set of monitored file descriptors. Each FD
/// has an associated interest set (EPOLLIN, EPOLLOUT, etc.) and a readiness
/// callback. The kernel event loop or socket layer notifies epoll when FDs
/// become ready.
///
/// ## Configuration
///
/// | Key                       | Default | Description                       |
/// |---------------------------|---------|-----------------------------------|
/// | `epoll_max_events`        | 1024    | Max events per epoll_wait call    |
/// | `epoll_max_fds_per_inst`  | 4096    | Max FDs per epoll instance        |
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

// ─── Telemetry ───────────────────────────────────────────────────────

static EPOLL_CREATE_CALLS: AtomicU64 = AtomicU64::new(0);
static EPOLL_CTL_CALLS: AtomicU64 = AtomicU64::new(0);
static EPOLL_WAIT_CALLS: AtomicU64 = AtomicU64::new(0);
static POLL_CALLS: AtomicU64 = AtomicU64::new(0);
static SELECT_CALLS: AtomicU64 = AtomicU64::new(0);
static EVENTS_DELIVERED: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
pub struct EventPollStats {
    pub epoll_creates: u64,
    pub epoll_ctls: u64,
    pub epoll_waits: u64,
    pub poll_calls: u64,
    pub select_calls: u64,
    pub events_delivered: u64,
}

pub fn eventpoll_stats() -> EventPollStats {
    EventPollStats {
        epoll_creates: EPOLL_CREATE_CALLS.load(Ordering::Relaxed),
        epoll_ctls: EPOLL_CTL_CALLS.load(Ordering::Relaxed),
        epoll_waits: EPOLL_WAIT_CALLS.load(Ordering::Relaxed),
        poll_calls: POLL_CALLS.load(Ordering::Relaxed),
        select_calls: SELECT_CALLS.load(Ordering::Relaxed),
        events_delivered: EVENTS_DELIVERED.load(Ordering::Relaxed),
    }
}

// ─── Event Flags ─────────────────────────────────────────────────────

bitflags::bitflags! {
    /// Event interest/readiness flags (matches Linux epoll bits).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct EpollEvents: u32 {
        /// Data available for reading.
        const EPOLLIN      = 0x001;
        /// Urgent data available (OOB).
        const EPOLLPRI     = 0x002;
        /// Ready for writing.
        const EPOLLOUT     = 0x004;
        /// Error condition.
        const EPOLLERR     = 0x008;
        /// Hang up.
        const EPOLLHUP     = 0x010;
        /// Read half of socket was shut down.
        const EPOLLRDHUP   = 0x2000;
        /// Edge-triggered mode.
        const EPOLLET      = 1 << 31;
        /// One-shot: disable after one event delivery.
        const EPOLLONESHOT = 1 << 30;
    }
}

// ─── Epoll Control Operations ────────────────────────────────────────

/// Operations for `epoll_ctl`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpollOp {
    /// Add an FD to the interest list.
    Add,
    /// Modify the events/data for an existing FD.
    Mod,
    /// Remove an FD from the interest list.
    Del,
}

// ─── Epoll Event ─────────────────────────────────────────────────────

/// An event returned by epoll_wait or poll.
#[derive(Debug, Clone, Copy)]
pub struct EpollEvent {
    /// The events that occurred.
    pub events: EpollEvents,
    /// User-supplied data associated with this FD.
    pub data: u64,
}

// ─── Epoll Interest Entry ────────────────────────────────────────────

/// Internal tracking of a monitored FD.
struct EpollInterest {
    /// File descriptor number.
    #[allow(dead_code)]
    fd: i32,
    /// Interested events mask.
    events: EpollEvents,
    /// User-supplied data.
    data: u64,
    /// Current readiness (set by the event source).
    ready: EpollEvents,
    /// Whether this interest has been disabled (EPOLLONESHOT fired).
    disabled: bool,
}

// ─── Epoll Instance ──────────────────────────────────────────────────

/// Unique epoll instance identifier.
pub type EpollFd = i32;

static NEXT_EPOLL_FD: AtomicU64 = AtomicU64::new(1000);

fn alloc_epoll_fd() -> EpollFd {
    NEXT_EPOLL_FD.fetch_add(1, Ordering::Relaxed) as EpollFd
}

/// A single epoll instance (created by `epoll_create`).
pub struct EpollInstance {
    /// Epoll file descriptor (identifies this instance).
    pub epfd: EpollFd,
    /// Monitored FDs: fd → interest entry.
    interests: BTreeMap<i32, EpollInterest>,
    /// Maximum FDs per instance.
    max_fds: usize,
}

impl EpollInstance {
    pub fn new(max_fds: usize) -> Self {
        EPOLL_CREATE_CALLS.fetch_add(1, Ordering::Relaxed);
        let max_fds =
            max_fds.min(crate::config::KernelConfig::network_epoll_max_fds_per_instance());
        Self {
            epfd: alloc_epoll_fd(),
            interests: BTreeMap::new(),
            max_fds,
        }
    }

    /// Add/modify/remove an FD from the interest list.
    pub fn ctl(
        &mut self,
        op: EpollOp,
        fd: i32,
        event: Option<EpollEvent>,
    ) -> Result<(), EpollError> {
        EPOLL_CTL_CALLS.fetch_add(1, Ordering::Relaxed);
        match op {
            EpollOp::Add => {
                if self.interests.contains_key(&fd) {
                    return Err(EpollError::AlreadyExists);
                }
                if self.interests.len() >= self.max_fds {
                    return Err(EpollError::TooManyFds);
                }
                let ev = event.ok_or(EpollError::InvalidArg)?;
                self.interests.insert(
                    fd,
                    EpollInterest {
                        fd,
                        events: ev.events,
                        data: ev.data,
                        ready: EpollEvents::empty(),
                        disabled: false,
                    },
                );
                Ok(())
            }
            EpollOp::Mod => {
                let entry = self.interests.get_mut(&fd).ok_or(EpollError::NotFound)?;
                let ev = event.ok_or(EpollError::InvalidArg)?;
                entry.events = ev.events;
                entry.data = ev.data;
                entry.disabled = false;
                Ok(())
            }
            EpollOp::Del => {
                self.interests.remove(&fd).ok_or(EpollError::NotFound)?;
                Ok(())
            }
        }
    }

    /// Notify readiness for an FD (called by the event source, e.g. network stack).
    pub fn notify_ready(&mut self, fd: i32, events: EpollEvents) {
        if let Some(entry) = self.interests.get_mut(&fd) {
            if !entry.disabled {
                entry.ready |= events;
            }
        }
    }

    /// Clear readiness for an FD (edge-triggered: after delivery).
    pub fn clear_ready(&mut self, fd: i32) {
        if let Some(entry) = self.interests.get_mut(&fd) {
            entry.ready = EpollEvents::empty();
        }
    }

    /// Wait for events. Returns up to `max_events` ready events.
    /// In a real kernel this would block; here we do a synchronous scan.
    pub fn wait(&mut self, max_events: usize) -> Vec<EpollEvent> {
        EPOLL_WAIT_CALLS.fetch_add(1, Ordering::Relaxed);
        let max_events = max_events
            .max(1)
            .min(crate::config::KernelConfig::network_epoll_max_events());
        let mut result = Vec::new();
        for entry in self.interests.values_mut() {
            if result.len() >= max_events {
                break;
            }
            if entry.disabled {
                continue;
            }
            let triggered = entry.ready & entry.events;
            if !triggered.is_empty() {
                result.push(EpollEvent {
                    events: triggered,
                    data: entry.data,
                });
                EVENTS_DELIVERED.fetch_add(1, Ordering::Relaxed);
                if entry.events.contains(EpollEvents::EPOLLET) {
                    entry.ready = EpollEvents::empty();
                }
                if entry.events.contains(EpollEvents::EPOLLONESHOT) {
                    entry.disabled = true;
                }
            }
        }
        result
    }

    /// Number of monitored FDs.
    pub fn fd_count(&self) -> usize {
        self.interests.len()
    }
}

// ─── Epoll Error ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpollError {
    AlreadyExists,
    NotFound,
    InvalidArg,
    TooManyFds,
}

// ─── Epoll Manager ──────────────────────────────────────────────────

/// Global epoll instance registry.
pub struct EpollManager {
    instances: BTreeMap<EpollFd, EpollInstance>,
}

impl EpollManager {
    pub fn new() -> Self {
        Self {
            instances: BTreeMap::new(),
        }
    }

    /// Create a new epoll instance. Returns the epoll FD.
    pub fn create(&mut self, max_fds: usize) -> EpollFd {
        let inst = EpollInstance::new(max_fds);
        let epfd = inst.epfd;
        self.instances.insert(epfd, inst);
        epfd
    }

    /// Get mutable reference to an epoll instance.
    pub fn get_mut(&mut self, epfd: EpollFd) -> Option<&mut EpollInstance> {
        self.instances.get_mut(&epfd)
    }

    /// Destroy an epoll instance.
    pub fn close(&mut self, epfd: EpollFd) -> bool {
        self.instances.remove(&epfd).is_some()
    }
}

// ─── Poll (struct pollfd) ────────────────────────────────────────────

/// A single poll file descriptor entry (mirrors `struct pollfd`).
#[derive(Debug, Clone, Copy)]
pub struct PollFd {
    pub fd: i32,
    /// Requested events.
    pub events: i16,
    /// Returned events (filled by kernel).
    pub revents: i16,
}

/// Standard poll event constants.
pub const POLLIN: i16 = 0x001;
pub const POLLPRI: i16 = 0x002;
pub const POLLOUT: i16 = 0x004;
pub const POLLERR: i16 = 0x008;
pub const POLLHUP: i16 = 0x010;
pub const POLLNVAL: i16 = 0x020;

/// Execute a poll operation over a set of file descriptors.
/// Returns the number of FDs with events.
///
/// The `readiness_fn` callback checks if a given fd has events:
/// `readiness_fn(fd) -> i16` returns the current ready events for the fd.
pub fn do_poll<F>(fds: &mut [PollFd], readiness_fn: F) -> usize
where
    F: Fn(i32) -> i16,
{
    POLL_CALLS.fetch_add(1, Ordering::Relaxed);
    let mut count = 0;
    for pfd in fds.iter_mut() {
        let ready = readiness_fn(pfd.fd);
        pfd.revents = ready & (pfd.events | POLLERR | POLLHUP | POLLNVAL);
        if pfd.revents != 0 {
            count += 1;
        }
    }
    count
}

// ─── Select (fd_set) ─────────────────────────────────────────────────

/// Maximum file descriptor number for select (like FD_SETSIZE).
pub const FD_SETSIZE: usize = 1024;

/// Bitmap-based file descriptor set for select().
#[derive(Clone)]
pub struct FdSet {
    bits: [u64; FD_SETSIZE / 64],
}

impl FdSet {
    pub fn new() -> Self {
        Self {
            bits: [0u64; FD_SETSIZE / 64],
        }
    }

    pub fn set(&mut self, fd: i32) {
        if (fd as usize) < FD_SETSIZE {
            let word = fd as usize / 64;
            let bit = fd as usize % 64;
            self.bits[word] |= 1u64 << bit;
        }
    }

    pub fn clear(&mut self, fd: i32) {
        if (fd as usize) < FD_SETSIZE {
            let word = fd as usize / 64;
            let bit = fd as usize % 64;
            self.bits[word] &= !(1u64 << bit);
        }
    }

    pub fn is_set(&self, fd: i32) -> bool {
        if (fd as usize) < FD_SETSIZE {
            let word = fd as usize / 64;
            let bit = fd as usize % 64;
            self.bits[word] & (1u64 << bit) != 0
        } else {
            false
        }
    }

    pub fn zero(&mut self) {
        self.bits = [0u64; FD_SETSIZE / 64];
    }

    /// Iterate over set FDs.
    pub fn iter_set(&self) -> impl Iterator<Item = i32> + '_ {
        (0..FD_SETSIZE as i32).filter(|&fd| self.is_set(fd))
    }
}

/// Execute a select operation.
/// Returns the total number of ready FDs across all sets.
///
/// `readiness_fn(fd) -> (readable, writable, exceptional)`
pub fn do_select<F>(
    nfds: i32,
    readfds: &mut Option<FdSet>,
    writefds: &mut Option<FdSet>,
    exceptfds: &mut Option<FdSet>,
    readiness_fn: F,
) -> usize
where
    F: Fn(i32) -> (bool, bool, bool),
{
    SELECT_CALLS.fetch_add(1, Ordering::Relaxed);
    let mut count = 0;

    // Build result sets.
    let mut r_result = FdSet::new();
    let mut w_result = FdSet::new();
    let mut e_result = FdSet::new();

    for fd in 0..nfds {
        let interested_r = readfds.as_ref().is_some_and(|s| s.is_set(fd));
        let interested_w = writefds.as_ref().is_some_and(|s| s.is_set(fd));
        let interested_e = exceptfds.as_ref().is_some_and(|s| s.is_set(fd));

        if !interested_r && !interested_w && !interested_e {
            continue;
        }

        let (readable, writable, exceptional) = readiness_fn(fd);

        if interested_r && readable {
            r_result.set(fd);
            count += 1;
        }
        if interested_w && writable {
            w_result.set(fd);
            count += 1;
        }
        if interested_e && exceptional {
            e_result.set(fd);
            count += 1;
        }
    }

    if let Some(r) = readfds {
        *r = r_result;
    }
    if let Some(w) = writefds {
        *w = w_result;
    }
    if let Some(e) = exceptfds {
        *e = e_result;
    }

    count
}

#[cfg(test)]
#[path = "epoll/tests.rs"]
mod tests;
