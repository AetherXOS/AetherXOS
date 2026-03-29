use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

use super::{map_net_errno, PosixErrno, PosixPollEvents, PosixPollFd};

static NEXT_EPOLL_ID: AtomicU32 = AtomicU32::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpollCtlOp {
    Add,
    Del,
    Mod,
}

impl EpollCtlOp {
    pub const fn from_raw(raw: i32) -> Option<Self> {
        match raw {
            crate::modules::posix_consts::net::EPOLL_CTL_ADD => Some(Self::Add),
            crate::modules::posix_consts::net::EPOLL_CTL_DEL => Some(Self::Del),
            crate::modules::posix_consts::net::EPOLL_CTL_MOD => Some(Self::Mod),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EpollEvent {
    pub fd: u32,
    pub events: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EpollTimeout {
    pub sec: i64,
    pub nsec: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EpollWatchState {
    events: PosixPollEvents,
    edge_triggered: bool,
    was_ready: bool,
}

lazy_static! {
    static ref EPOLL_REGISTRY: Mutex<BTreeMap<u32, BTreeMap<u32, EpollWatchState>>> =
        Mutex::new(BTreeMap::new());
}

fn epoll_events_to_poll(events: u32) -> PosixPollEvents {
    let mut out = PosixPollEvents::empty();
    if (events & crate::modules::posix_consts::net::EPOLLIN) != 0 {
        out.insert(PosixPollEvents::IN);
    }
    if (events & crate::modules::posix_consts::net::EPOLLOUT) != 0 {
        out.insert(PosixPollEvents::OUT);
    }
    if (events & crate::modules::posix_consts::net::EPOLLERR) != 0 {
        out.insert(PosixPollEvents::ERR);
    }
    if (events & crate::modules::posix_consts::net::EPOLLHUP) != 0 {
        out.insert(PosixPollEvents::HUP);
    }
    if out.is_empty() {
        PosixPollEvents::IN | PosixPollEvents::OUT
    } else {
        out
    }
}

fn epoll_events_edge(events: u32) -> bool {
    (events & crate::modules::posix_consts::net::EPOLLET) != 0
}

fn poll_events_to_epoll(events: PosixPollEvents) -> u32 {
    let mut out = 0u32;
    if events.contains(PosixPollEvents::IN) {
        out |= crate::modules::posix_consts::net::EPOLLIN;
    }
    if events.contains(PosixPollEvents::OUT) {
        out |= crate::modules::posix_consts::net::EPOLLOUT;
    }
    if events.contains(PosixPollEvents::ERR) {
        out |= crate::modules::posix_consts::net::EPOLLERR;
    }
    if events.contains(PosixPollEvents::HUP) {
        out |= crate::modules::posix_consts::net::EPOLLHUP;
    }
    out
}

fn retries_from_timeout(timeout: Option<EpollTimeout>) -> Result<usize, PosixErrno> {
    let Some(ts) = timeout else {
        return Ok(0);
    };

    if ts.sec < 0 || ts.nsec < 0 || ts.nsec >= 1_000_000_000 {
        return Err(PosixErrno::Invalid);
    }

    let total_ns = (ts.sec as u128)
        .saturating_mul(1_000_000_000u128)
        .saturating_add(ts.nsec as u128);
    if total_ns == 0 {
        return Ok(0);
    }

    let slice_ns = crate::generated_consts::TIME_SLICE_NS as u128;
    let tick_ns = if slice_ns == 0 { 1 } else { slice_ns };
    Ok(((total_ns + tick_ns - 1) / tick_ns) as usize)
}

pub fn epoll_create1(flags: i32) -> Result<u32, PosixErrno> {
    if (flags & !crate::modules::posix_consts::net::EPOLL_CLOEXEC) != 0 {
        return Err(PosixErrno::Invalid);
    }
    let epfd = NEXT_EPOLL_ID.fetch_add(1, Ordering::Relaxed);
    EPOLL_REGISTRY.lock().insert(epfd, BTreeMap::new());
    Ok(epfd)
}

pub fn epoll_ctl(epfd: u32, op_raw: i32, fd: u32, events: u32) -> Result<(), PosixErrno> {
    let op = EpollCtlOp::from_raw(op_raw).ok_or(PosixErrno::Invalid)?;
    let mut registry = EPOLL_REGISTRY.lock();
    let watched = registry
        .get_mut(&epfd)
        .ok_or(PosixErrno::BadFileDescriptor)?;
    let state = EpollWatchState {
        events: epoll_events_to_poll(events),
        edge_triggered: epoll_events_edge(events),
        was_ready: false,
    };
    match op {
        EpollCtlOp::Add => {
            if watched.contains_key(&fd) {
                return Err(PosixErrno::AlreadyExists);
            }
            watched.insert(fd, state);
            Ok(())
        }
        EpollCtlOp::Mod => {
            if !watched.contains_key(&fd) {
                return Err(PosixErrno::NoEntry);
            }
            watched.insert(fd, state);
            Ok(())
        }
        EpollCtlOp::Del => {
            if watched.remove(&fd).is_none() {
                return Err(PosixErrno::NoEntry);
            }
            Ok(())
        }
    }
}

pub fn epoll_wait(
    epfd: u32,
    max_events: usize,
    retries: usize,
) -> Result<Vec<EpollEvent>, PosixErrno> {
    if max_events == 0 {
        return Err(PosixErrno::Invalid);
    }

    let watched_snapshot: Vec<(u32, EpollWatchState)> = {
        let registry = EPOLL_REGISTRY.lock();
        let watched = registry.get(&epfd).ok_or(PosixErrno::BadFileDescriptor)?;
        watched.iter().map(|(fd, state)| (*fd, *state)).collect()
    };

    if watched_snapshot.is_empty() {
        return Ok(Vec::new());
    }

    for _ in 0..=retries {
        let mut pollfds: Vec<PosixPollFd> = watched_snapshot
            .iter()
            .map(|(fd, state)| PosixPollFd::new(*fd, state.events))
            .collect();

        crate::modules::libnet::posix_poll_errno(&mut pollfds, 0).map_err(map_net_errno)?;
        let mut ready: Vec<EpollEvent> = Vec::new();

        {
            let mut registry = EPOLL_REGISTRY.lock();
            let watched = registry
                .get_mut(&epfd)
                .ok_or(PosixErrno::BadFileDescriptor)?;

            for polled in pollfds.into_iter() {
                if let Some(state) = watched.get_mut(&polled.fd) {
                    let is_ready = !polled.revents.is_empty();
                    if !is_ready {
                        state.was_ready = false;
                        continue;
                    }

                    if state.edge_triggered && state.was_ready {
                        continue;
                    }

                    state.was_ready = true;
                    ready.push(EpollEvent {
                        fd: polled.fd,
                        events: poll_events_to_epoll(polled.revents),
                    });
                }
            }
        }

        if !ready.is_empty() {
            if ready.len() > max_events {
                ready.truncate(max_events);
            }
            return Ok(ready);
        }
    }

    Ok(Vec::new())
}

pub fn epoll_pwait(
    epfd: u32,
    max_events: usize,
    retries: usize,
    sigmask: Option<u64>,
) -> Result<Vec<EpollEvent>, PosixErrno> {
    #[cfg(feature = "posix_signal")]
    let old_mask = if let Some(mask) = sigmask {
        Some(
            crate::modules::posix::signal::sigprocmask(
                crate::modules::posix::signal::SigmaskHow::SetMask,
                Some(mask),
            )
            .map_err(|_| PosixErrno::Invalid)?,
        )
    } else {
        None
    };

    let result = epoll_wait(epfd, max_events, retries);

    #[cfg(feature = "posix_signal")]
    if let Some(old) = old_mask {
        let _ = crate::modules::posix::signal::sigprocmask(
            crate::modules::posix::signal::SigmaskHow::SetMask,
            Some(old),
        );
    }

    #[cfg(not(feature = "posix_signal"))]
    let _ = sigmask;

    result
}

pub fn epoll_pwait2(
    epfd: u32,
    max_events: usize,
    timeout: Option<EpollTimeout>,
    sigmask: Option<u64>,
) -> Result<Vec<EpollEvent>, PosixErrno> {
    let retries = retries_from_timeout(timeout)?;
    epoll_pwait(epfd, max_events, retries, sigmask)
}

pub fn epoll_wait_timeout_ms(
    epfd: u32,
    max_events: usize,
    timeout_ms: u64,
) -> Result<Vec<EpollEvent>, PosixErrno> {
    let sec = (timeout_ms / 1000) as i64;
    let nsec = ((timeout_ms % 1000) * 1_000_000) as i32;
    epoll_pwait2(epfd, max_events, Some(EpollTimeout { sec, nsec }), None)
}

pub fn epoll_close(epfd: u32) -> Result<(), PosixErrno> {
    let removed = EPOLL_REGISTRY.lock().remove(&epfd);
    if removed.is_none() {
        return Err(PosixErrno::BadFileDescriptor);
    }
    Ok(())
}

#[inline(always)]
pub fn epoll_ctl_typed(epfd: u32, op: EpollCtlOp, fd: u32, events: u32) -> Result<(), PosixErrno> {
    let raw = match op {
        EpollCtlOp::Add => crate::modules::posix_consts::net::EPOLL_CTL_ADD,
        EpollCtlOp::Del => crate::modules::posix_consts::net::EPOLL_CTL_DEL,
        EpollCtlOp::Mod => crate::modules::posix_consts::net::EPOLL_CTL_MOD,
    };
    epoll_ctl(epfd, raw, fd, events)
}

pub fn await_readable(fd: u32, retries: usize) -> Result<(), PosixErrno> {
    let mut pollfd = [PosixPollFd::new(fd, PosixPollEvents::IN)];
    for _ in 0..=retries {
        let ready =
            crate::modules::libnet::posix_poll_errno(&mut pollfd, 0).map_err(map_net_errno)?;
        if ready > 0 && pollfd[0].revents.contains(PosixPollEvents::IN) {
            return Ok(());
        }
    }
    Err(PosixErrno::Again)
}

pub fn await_writable(fd: u32, retries: usize) -> Result<(), PosixErrno> {
    let mut pollfd = [PosixPollFd::new(fd, PosixPollEvents::OUT)];
    for _ in 0..=retries {
        let ready =
            crate::modules::libnet::posix_poll_errno(&mut pollfd, 0).map_err(map_net_errno)?;
        if ready > 0 && pollfd[0].revents.contains(PosixPollEvents::OUT) {
            return Ok(());
        }
    }
    Err(PosixErrno::Again)
}

pub fn on_close_fd(fd: u32) {
    for watched in EPOLL_REGISTRY.lock().values_mut() {
        watched.remove(&fd);
    }
}
