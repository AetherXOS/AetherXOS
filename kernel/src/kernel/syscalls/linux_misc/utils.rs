use super::*;
use crate::kernel::syscalls::user_access::with_user_read_bytes;
use super::types::*;

#[inline]
pub fn validate_timespec_compat(ts: LinuxTimespecCompat) -> bool {
    ts.tv_sec >= 0 && ts.tv_nsec >= 0 && ts.tv_nsec < 1_000_000_000
}

#[inline]
pub fn timespec_to_ns(ts: LinuxTimespecCompat) -> Option<u128> {
    if !validate_timespec_compat(ts) {
        return None;
    }
    Some((ts.tv_sec as u128).saturating_mul(1_000_000_000u128).saturating_add(ts.tv_nsec as u128))
}

#[inline]
pub fn ns_to_timespec(ns: u128) -> LinuxTimespecCompat {
    LinuxTimespecCompat {
        tv_sec: (ns / 1_000_000_000u128) as i64,
        tv_nsec: (ns % 1_000_000_000u128) as i64,
    }
}

#[inline]
pub fn monotonic_now_ns() -> u128 {
    #[cfg(feature = "posix_time")]
    {
        if let Ok(ts) = crate::modules::posix::time::clock_gettime_raw(
            crate::modules::posix_consts::time::CLOCK_MONOTONIC,
        ) {
            return (ts.sec as u128)
                .saturating_mul(1_000_000_000u128)
                .saturating_add(ts.nsec as u128);
        }
    }
    let tick_ns = core::cmp::max(crate::generated_consts::TIME_SLICE_NS as u128, 1u128);
    (crate::hal::cpu::rdtsc() as u128).saturating_mul(tick_ns)
}

pub fn read_user_struct<T: Copy + Default>(ptr: usize) -> Result<T, usize> {
    crate::kernel::syscalls::linux_shim::util::read_user_pod(ptr)
}

pub fn write_user_struct<T: Copy>(ptr: usize, value: &T) -> Result<(), usize> {
    crate::kernel::syscalls::linux_shim::util::write_user_pod(ptr, value)
}

pub fn read_user_c_string_compat(ptr: usize, max_len: usize) -> Result<alloc::string::String, usize> {
    if ptr == 0 || max_len == 0 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EFAULT));
    }
    let mut out = alloc::vec::Vec::new();
    for i in 0..max_len {
        let Some(addr) = ptr.checked_add(i) else {
            return Err(linux_errno(crate::modules::posix_consts::errno::EFAULT));
        };
        let b = with_user_read_bytes(addr, 1, |src| src[0])
            .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))?;
        if b == 0 {
            return alloc::string::String::from_utf8(out)
                .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EINVAL));
        }
        out.push(b);
    }
    Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
}

pub fn read_u64_from_user(ptr: usize) -> Result<u64, usize> {
    with_user_read_bytes(ptr, core::mem::size_of::<u64>(), |src| {
        let mut bytes = [0u8; core::mem::size_of::<u64>()];
        bytes.copy_from_slice(src);
        u64::from_ne_bytes(bytes)
    })
    .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
}

pub fn read_itimerspec_from_user(ptr: usize) -> Result<LinuxItimerspecCompat, usize> {
    read_user_struct::<LinuxItimerspecCompat>(ptr)
}

pub fn write_itimerspec_to_user(ptr: usize, spec: LinuxItimerspecCompat) -> usize {
    write_user_struct::<LinuxItimerspecCompat>(ptr, &spec)
        .map(|_| 0usize)
        .unwrap_or_else(|err| err)
}
