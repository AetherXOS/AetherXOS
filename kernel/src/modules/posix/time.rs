use crate::modules::posix::PosixErrno;
use core::sync::atomic::{AtomicI64, Ordering};

static REALTIME_OFFSET_NS: AtomicI64 = AtomicI64::new(0);

use alloc::collections::BTreeMap;
use spin::Mutex;
lazy_static::lazy_static! {
    static ref ITIMER_TABLE: Mutex<BTreeMap<usize, (PosixTimeval, PosixTimeval)>> = Mutex::new(BTreeMap::new());
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PosixTimespec {
    pub sec: i64,
    pub nsec: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PosixTimeval {
    pub sec: i64,
    pub usec: i32,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PosixClockId {
    Realtime = crate::modules::posix_consts::time::CLOCK_REALTIME,
    Monotonic = crate::modules::posix_consts::time::CLOCK_MONOTONIC,
    RealtimeCoarse = crate::modules::posix_consts::time::CLOCK_REALTIME_COARSE,
    MonotonicCoarse = crate::modules::posix_consts::time::CLOCK_MONOTONIC_COARSE,
}

impl PosixClockId {
    pub const fn as_raw(self) -> i32 {
        self as i32
    }

    pub const fn from_raw(value: i32) -> Option<Self> {
        match value {
            crate::modules::posix_consts::time::CLOCK_REALTIME => Some(Self::Realtime),
            crate::modules::posix_consts::time::CLOCK_MONOTONIC => Some(Self::Monotonic),
            crate::modules::posix_consts::time::CLOCK_REALTIME_COARSE => Some(Self::RealtimeCoarse),
            crate::modules::posix_consts::time::CLOCK_MONOTONIC_COARSE => {
                Some(Self::MonotonicCoarse)
            }
            _ => None,
        }
    }
}

#[inline(always)]
fn tick_ns() -> u64 {
    let slice = crate::generated_consts::TIME_SLICE_NS;
    if slice == 0 {
        1
    } else {
        slice
    }
}

#[inline(always)]
fn tick_to_timespec(tick: u64) -> PosixTimespec {
    let slice_ns = crate::generated_consts::TIME_SLICE_NS as u128;
    if slice_ns == 0 {
        return PosixTimespec {
            sec: tick as i64,
            nsec: 0,
        };
    }

    let total_ns = (tick as u128).saturating_mul(slice_ns);
    let sec = (total_ns / 1_000_000_000u128) as i64;
    let nsec = (total_ns % 1_000_000_000u128) as i32;
    PosixTimespec { sec, nsec }
}

#[inline(always)]
fn normalize_timespec(mut sec: i64, mut nsec: i64) -> PosixTimespec {
    if nsec >= 1_000_000_000 {
        sec = sec.saturating_add(nsec / 1_000_000_000);
        nsec %= 1_000_000_000;
    } else if nsec < 0 {
        let borrow = ((-nsec) + 999_999_999) / 1_000_000_000;
        sec = sec.saturating_sub(borrow);
        nsec += borrow * 1_000_000_000;
    }

    PosixTimespec {
        sec,
        nsec: nsec as i32,
    }
}

#[inline(always)]
pub fn monotonic_timespec() -> PosixTimespec {
    let tick = crate::kernel::watchdog::global_tick();
    tick_to_timespec(tick)
}

#[inline(always)]
pub fn realtime_timespec() -> PosixTimespec {
    let base = monotonic_timespec();
    let off_ns = REALTIME_OFFSET_NS.load(Ordering::Relaxed);
    let base_ns = (base.sec as i128)
        .saturating_mul(1_000_000_000i128)
        .saturating_add(base.nsec as i128);
    let adjusted_ns = base_ns.saturating_add(off_ns as i128);
    let sec = (adjusted_ns / 1_000_000_000i128) as i64;
    let nsec = (adjusted_ns % 1_000_000_000i128) as i64;
    normalize_timespec(sec, nsec)
}

pub fn clock_gettime(clock_id: PosixClockId) -> PosixTimespec {
    match clock_id {
        PosixClockId::Realtime | PosixClockId::RealtimeCoarse => realtime_timespec(),
        PosixClockId::Monotonic | PosixClockId::MonotonicCoarse => monotonic_timespec(),
    }
}

pub fn clock_gettime64(clock_id: PosixClockId) -> PosixTimespec {
    clock_gettime(clock_id)
}

pub fn clock_gettime_raw(clock_id_raw: i32) -> Result<PosixTimespec, PosixErrno> {
    let clock_id = PosixClockId::from_raw(clock_id_raw).ok_or(PosixErrno::Invalid)?;
    Ok(clock_gettime(clock_id))
}

pub fn clock_settime(clock_id: PosixClockId, value: PosixTimespec) -> Result<(), PosixErrno> {
    validate_timespec(value)?;
    match clock_id {
        PosixClockId::Monotonic | PosixClockId::MonotonicCoarse => Err(PosixErrno::Invalid),
        PosixClockId::Realtime | PosixClockId::RealtimeCoarse => {
            let now_mono = monotonic_timespec();
            let now_ns = (now_mono.sec as i128)
                .saturating_mul(1_000_000_000i128)
                .saturating_add(now_mono.nsec as i128);
            let target_ns = (value.sec as i128)
                .saturating_mul(1_000_000_000i128)
                .saturating_add(value.nsec as i128);
            let off = target_ns.saturating_sub(now_ns);
            let off_clamped = off.clamp(i64::MIN as i128, i64::MAX as i128) as i64;
            REALTIME_OFFSET_NS.store(off_clamped, Ordering::Relaxed);
            Ok(())
        }
    }
}

pub fn clock_settime_raw(clock_id_raw: i32, value: PosixTimespec) -> Result<(), PosixErrno> {
    let clock_id = PosixClockId::from_raw(clock_id_raw).ok_or(PosixErrno::Invalid)?;
    clock_settime(clock_id, value)
}

pub fn clock_getres(clock_id: PosixClockId) -> PosixTimespec {
    let _ = clock_id;
    PosixTimespec {
        sec: 0,
        nsec: tick_ns() as i32,
    }
}

pub fn clock_getres_raw(clock_id_raw: i32) -> Result<PosixTimespec, PosixErrno> {
    let clock_id = PosixClockId::from_raw(clock_id_raw).ok_or(PosixErrno::Invalid)?;
    Ok(clock_getres(clock_id))
}

pub fn timespec_get(base: i32) -> Result<PosixTimespec, PosixErrno> {
    if base == crate::modules::posix_consts::time::TIME_UTC {
        Ok(realtime_timespec())
    } else {
        Err(PosixErrno::Invalid)
    }
}

pub fn timespec_getres(base: i32) -> Result<PosixTimespec, PosixErrno> {
    if base == crate::modules::posix_consts::time::TIME_UTC {
        Ok(clock_getres(PosixClockId::Realtime))
    } else {
        Err(PosixErrno::Invalid)
    }
}

pub fn gettimeofday() -> PosixTimeval {
    let ts = realtime_timespec();
    PosixTimeval {
        sec: ts.sec,
        usec: ts.nsec / 1_000,
    }
}

pub fn settimeofday(tv: PosixTimeval) -> Result<(), PosixErrno> {
    if tv.sec < 0 || tv.usec < 0 || tv.usec >= 1_000_000 {
        return Err(PosixErrno::Invalid);
    }
    clock_settime(
        PosixClockId::Realtime,
        PosixTimespec {
            sec: tv.sec,
            nsec: tv.usec.saturating_mul(1_000),
        },
    )
}

pub fn time_now() -> i64 {
    realtime_timespec().sec
}

fn validate_timespec(req: PosixTimespec) -> Result<(), PosixErrno> {
    if req.sec < 0 || req.nsec < 0 || req.nsec >= 1_000_000_000 {
        return Err(PosixErrno::Invalid);
    }
    Ok(())
}

fn timespec_to_ns(ts: PosixTimespec) -> u128 {
    (ts.sec as u128)
        .saturating_mul(1_000_000_000u128)
        .saturating_add(ts.nsec as u128)
}

fn ns_to_timespec(total_ns: u128) -> PosixTimespec {
    PosixTimespec {
        sec: (total_ns / 1_000_000_000u128) as i64,
        nsec: (total_ns % 1_000_000_000u128) as i32,
    }
}

pub fn nanosleep(req: PosixTimespec) -> Result<(), PosixErrno> {
    validate_timespec(req)?;

    let start = crate::kernel::watchdog::global_tick();
    let total_ns = timespec_to_ns(req);
    if total_ns == 0 {
        return Ok(());
    }

    let ticks_needed = ((total_ns + tick_ns() as u128 - 1) / tick_ns() as u128) as u64;
    let target = start.saturating_add(ticks_needed);

    while crate::kernel::watchdog::global_tick() < target {
        crate::kernel::rt_preemption::request_forced_reschedule();
    }
    Ok(())
}

pub fn nanosleep_with_rem(req: PosixTimespec) -> Result<PosixTimespec, PosixErrno> {
    nanosleep(req)?;
    Ok(PosixTimespec { sec: 0, nsec: 0 })
}

pub fn clock_nanosleep(
    clock_id: PosixClockId,
    flags: i32,
    req: PosixTimespec,
) -> Result<Option<PosixTimespec>, PosixErrno> {
    validate_timespec(req)?;

    let supported = crate::modules::posix_consts::time::TIMER_ABSTIME;
    if (flags & !supported) != 0 {
        return Err(PosixErrno::Invalid);
    }

    if (flags & crate::modules::posix_consts::time::TIMER_ABSTIME) != 0 {
        let now = clock_gettime(clock_id);
        let now_ns = timespec_to_ns(now);
        let req_ns = timespec_to_ns(req);
        if req_ns > now_ns {
            nanosleep(ns_to_timespec(req_ns - now_ns))?;
        }
        Ok(None)
    } else {
        nanosleep(req)?;
        Ok(None)
    }
}

pub fn clock_nanosleep_raw(
    clock_id_raw: i32,
    flags: i32,
    req: PosixTimespec,
) -> Result<Option<PosixTimespec>, PosixErrno> {
    let clock_id = PosixClockId::from_raw(clock_id_raw).ok_or(PosixErrno::Invalid)?;
    clock_nanosleep(clock_id, flags, req)
}

pub fn usleep(usec: u64) -> Result<(), PosixErrno> {
    let sec = (usec / 1_000_000) as i64;
    let nsec = ((usec % 1_000_000) * 1_000) as i32;
    nanosleep(PosixTimespec { sec, nsec })
}

pub fn sleep(sec: u32) -> Result<(), PosixErrno> {
    nanosleep(PosixTimespec {
        sec: sec as i64,
        nsec: 0,
    })
}
pub fn getitimer(pid: usize) -> (PosixTimeval, PosixTimeval) {
    let table = ITIMER_TABLE.lock();
    *table.get(&pid).unwrap_or(&(
        PosixTimeval { sec: 0, usec: 0 },
        PosixTimeval { sec: 0, usec: 0 },
    ))
}

pub fn setitimer(
    pid: usize,
    interval: PosixTimeval,
    value: PosixTimeval,
) -> (PosixTimeval, PosixTimeval) {
    let old = getitimer(pid);
    let mut table = ITIMER_TABLE.lock();
    if value.sec == 0 && value.usec == 0 {
        table.remove(&pid);
    } else {
        table.insert(pid, (interval, value));
    }
    old
}
