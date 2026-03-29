#[cfg(not(feature = "linux_compat"))]
use super::{linux_errno, with_user_write_bytes};
#[cfg(not(feature = "linux_compat"))]
use core::sync::atomic::{AtomicU64, Ordering};

#[repr(C)]
#[cfg(not(feature = "linux_compat"))]
struct LinuxSysinfo {
    uptime: i64,
    loads: [u64; 3],
    totalram: u64,
    freeram: u64,
    sharedram: u64,
    bufferram: u64,
    totalswap: u64,
    freeswap: u64,
    procs: u16,
    pad: u16,
    totalhigh: u64,
    freehigh: u64,
    mem_unit: u32,
}

#[cfg(not(feature = "linux_compat"))]
fn linux_getrandom_cap() -> usize {
    const MIN_CAP: usize = 256;
    const MAX_CAP: usize = 64 * 1024;

    crate::config::KernelConfig::network_loopback_queue_limit().clamp(MIN_CAP, MAX_CAP)
}

#[cfg(not(feature = "linux_compat"))]
static GETRANDOM_STATE: AtomicU64 = AtomicU64::new(0x6A09E667F3BCC909);

#[inline(always)]
#[cfg(not(feature = "linux_compat"))]
fn mix64(mut v: u64) -> u64 {
    v ^= v >> 33;
    v = v.wrapping_mul(0xff51afd7ed558ccd);
    v ^= v >> 33;
    v = v.wrapping_mul(0xc4ceb9fe1a85ec53);
    v ^= v >> 33;
    v
}

#[inline(always)]
#[cfg(not(feature = "linux_compat"))]
fn next_getrandom_word(state: &mut u64) -> u64 {
    // xorshift64* step with odd-state guard.
    if *state == 0 {
        *state = 0x9E3779B97F4A7C15;
    }
    let mut x = *state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    *state = x;
    x.wrapping_mul(0x2545F4914F6CDD1D)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_gettimeofday(tv_ptr: usize) -> usize {
    #[cfg(feature = "posix_time")]
    {
        let spec = match crate::modules::posix::time::clock_gettime_raw(0) {
            Ok(v) => v,
            Err(err) => return linux_errno(err.code()),
        };
        with_user_write_bytes(tv_ptr, 16, |dst| {
            dst[0..8].copy_from_slice(&spec.sec.to_ne_bytes());
            let usec = (spec.nsec as i64) / 1000;
            dst[8..16].copy_from_slice(&usec.to_ne_bytes());
            0
        })
        .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
    }
    #[cfg(not(feature = "posix_time"))]
    {
        let _ = tv_ptr;
        0
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_time(tloc: usize) -> usize {
    #[cfg(feature = "posix_time")]
    {
        let spec = match crate::modules::posix::time::clock_gettime_raw(0) {
            Ok(v) => v,
            Err(_) => return 0,
        };
        let secs = spec.sec as usize;
        if tloc != 0 {
            let wrote = with_user_write_bytes(tloc, 8, |dst| {
                dst.copy_from_slice(&(secs as u64).to_ne_bytes());
                0
            });
            if wrote.is_err() {
                return linux_errno(crate::modules::posix_consts::errno::EFAULT);
            }
        }
        secs
    }
    #[cfg(not(feature = "posix_time"))]
    {
        let _ = tloc;
        0
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_getcpu(cpu_ptr: usize, node_ptr: usize) -> usize {
    let cpu_id = crate::kernel::cpu_local::CpuLocal::try_id().unwrap_or(0) as u32;
    if cpu_ptr != 0 {
        let wrote = with_user_write_bytes(cpu_ptr, 4, |dst| {
            dst.copy_from_slice(&cpu_id.to_ne_bytes());
            0
        });
        if wrote.is_err() {
            return linux_errno(crate::modules::posix_consts::errno::EFAULT);
        }
    }
    if node_ptr != 0 {
        let wrote = with_user_write_bytes(node_ptr, 4, |dst| {
            dst.copy_from_slice(&0u32.to_ne_bytes());
            0
        });
        if wrote.is_err() {
            return linux_errno(crate::modules::posix_consts::errno::EFAULT);
        }
    }
    0
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_sysinfo(info_ptr: usize) -> usize {
    let total = core::mem::size_of::<LinuxSysinfo>();
    with_user_write_bytes(info_ptr, total, |dst| {
        dst.fill(0);
        let uptime: i64 = 1;
        dst[0..8].copy_from_slice(&uptime.to_ne_bytes());
        let totalram: u64 = 256 * 1024 * 1024;
        dst[32..40].copy_from_slice(&totalram.to_ne_bytes());
        let freeram: u64 = 128 * 1024 * 1024;
        dst[40..48].copy_from_slice(&freeram.to_ne_bytes());
        let procs: u16 = 1;
        dst[72..74].copy_from_slice(&procs.to_ne_bytes());
        let mem_unit: u32 = 1;
        dst[84..88].copy_from_slice(&mem_unit.to_ne_bytes());
        0
    })
    .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_getrandom(buf_ptr: usize, buflen: usize, _flags: usize) -> usize {
    if buflen == 0 {
        return 0;
    }
    let buf_len = core::cmp::min(buflen, linux_getrandom_cap());
    let (cpu_id, tid) = unsafe {
        crate::kernel::cpu_local::CpuLocal::try_get()
            .map(|cpu| {
                (
                    cpu.cpu_id.0 as u32,
                    cpu.current_task.load(Ordering::Relaxed) as u64,
                )
            })
            .unwrap_or((0, 0))
    };

    // Mix per-call context and monotonic state to avoid deterministic same-seed outputs.
    let mut seed = ((cpu_id as u64) << 32) ^ tid ^ (buf_ptr as u64) ^ (buf_len as u64);
    #[cfg(target_arch = "x86_64")]
    {
        seed ^= unsafe { core::arch::x86_64::_rdtsc() };
    }
    seed ^= GETRANDOM_STATE.fetch_add(0x9E3779B97F4A7C15, Ordering::Relaxed);
    let mut prng_state = mix64(seed);

    with_user_write_bytes(buf_ptr, buf_len, |dst| {
        let mut offset = 0usize;
        while offset < dst.len() {
            let word = next_getrandom_word(&mut prng_state).to_le_bytes();
            let take = core::cmp::min(8, dst.len() - offset);
            dst[offset..offset + take].copy_from_slice(&word[..take]);
            offset += take;
        }
        GETRANDOM_STATE.store(mix64(prng_state ^ seed), Ordering::Relaxed);
        buf_len
    })
    .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
}

#[cfg(all(test, not(feature = "linux_compat")))]
mod tests {
    use super::*;

    #[test_case]
    fn time_invalid_pointer_returns_efault() {
        #[cfg(feature = "posix_time")]
        {
            assert_eq!(
                sys_linux_time(0x1),
                linux_errno(crate::modules::posix_consts::errno::EFAULT)
            );
        }
    }

    #[test_case]
    fn getcpu_invalid_cpu_pointer_returns_efault() {
        assert_eq!(
            sys_linux_getcpu(0x1, 0),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[test_case]
    fn gettimeofday_invalid_pointer_returns_efault() {
        #[cfg(feature = "posix_time")]
        {
            assert_eq!(
                sys_linux_gettimeofday(0x1),
                linux_errno(crate::modules::posix_consts::errno::EFAULT)
            );
        }
    }

    #[test_case]
    fn getrandom_zero_length_is_noop() {
        assert_eq!(sys_linux_getrandom(0, 0, 0), 0);
    }

    #[test_case]
    fn getrandom_invalid_pointer_returns_efault() {
        assert_eq!(
            sys_linux_getrandom(0x1, 16, 0),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[test_case]
    fn sysinfo_invalid_pointer_returns_efault() {
        assert_eq!(
            sys_linux_sysinfo(0x1),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }
}
