use super::super::*;

const LINUX_GRND_NONBLOCK: usize = 0x0001;
const LINUX_GRND_RANDOM: usize = 0x0002;
const LINUX_GRND_INSECURE: usize = 0x0004;
const LINUX_GETRANDOM_FLAGS_MASK: usize =
    LINUX_GRND_NONBLOCK | LINUX_GRND_RANDOM | LINUX_GRND_INSECURE;

pub fn sys_linux_uname(ptr: UserPtr<u8>) -> usize {
    let sysname = b"Linux\0";
    let release_str = crate::config::KernelConfig::linux_release();
    let version_str = crate::config::KernelConfig::linux_version();
    let machine = b"x86_64\0";

    let mut release = release_str.as_bytes().to_vec();
    release.push(0);
    let mut version = version_str.as_bytes().to_vec();
    version.push(0);

    #[cfg(feature = "posix_process")]
    let nodename_str = crate::modules::posix::process::get_hostname();
    #[cfg(not(feature = "posix_process"))]
    let nodename_str = alloc::string::String::from("aethercore");

    let mut nodename = nodename_str.into_bytes();
    nodename.push(0);

    #[cfg(feature = "posix_process")]
    let domainname_str = crate::modules::posix::process::get_domainname();
    #[cfg(not(feature = "posix_process"))]
    let domainname_str = alloc::string::String::from("localdomain");

    let mut domainname = domainname_str.into_bytes();
    domainname.push(0);

    const UTSNAME_LEN: usize = 65;

    with_user_write_bytes(ptr.addr, UTSNAME_LEN * 6, |dst| {
        dst.fill(0);

        // Helper to copy strings into fixed size buffers
        let mut copy_to_buf = |index: usize, data: &[u8]| {
            let offset = index * UTSNAME_LEN;
            let len = data.len().min(UTSNAME_LEN - 1); // reserve null byte
            dst[offset..offset + len].copy_from_slice(&data[..len]);
        };

        copy_to_buf(0, sysname);
        copy_to_buf(1, &nodename);
        copy_to_buf(2, &release);
        copy_to_buf(3, &version);
        copy_to_buf(4, machine);
        copy_to_buf(5, &domainname);
        0
    })
    .unwrap_or_else(|_| linux_eacces())
}

pub fn sys_linux_getrandom(buf: UserPtr<u8>, buflen: usize, flags: usize) -> usize {
    if (flags & !LINUX_GETRANDOM_FLAGS_MASK) != 0 {
        return linux_inval();
    }
    if buf.is_null() || buflen == 0 {
        return 0;
    }

    // Better entropy using RDTSC and XORShift
    let _ = with_user_write_bytes(buf.addr, buflen, |dst| {
        let mut state = crate::hal::cpu::rdtsc() ^ 0x2545F4914F6CDD1D;
        for chunk in dst.chunks_mut(8) {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let bytes = state.to_ne_bytes();
            for (i, v) in chunk.iter_mut().enumerate() {
                *v = bytes[i % 8];
            }
        }
        0
    });
    buflen
}

pub fn sys_linux_sysinfo(info_ptr: UserPtr<LinuxSysinfo>) -> usize {
    let ticks = crate::kernel::watchdog::global_tick();
    let ns_per_tick = crate::config::KernelConfig::time_slice();
    let uptime = (ticks * ns_per_tick / 1_000_000_000) as i64;

    // We use the constants from BitmapAllocator
    let total_ram = (crate::modules::allocators::bitmap_pmm::PMM_TOTAL_PAGES * 4096) as u64;

    // Count free pages from PMM telemetry
    let free_pages = crate::modules::allocators::bitmap_pmm::get_free_pages() as u64;
    let free_ram = free_pages * 4096;

    #[cfg(feature = "posix_process")]
    let procs = crate::modules::posix::process::process_count() as u16;
    #[cfg(not(feature = "posix_process"))]
    let procs = 1u16;

    let loads = crate::kernel::watchdog::load_avg();

    let info = LinuxSysinfo {
        uptime,
        loads: [loads[0], loads[1], loads[2]],
        totalram: total_ram,
        freeram: free_ram,
        sharedram: 0,
        bufferram: 0,
        totalswap: 0,
        freeswap: 0,
        procs,
        pad: 0,
        totalhigh: 0,
        freehigh: 0,
        mem_unit: 1,
        _f: [],
    };

    info_ptr.write(&info).map(|_| 0).unwrap_or_else(|e| e)
}

pub fn sys_linux_times(tms_ptr: UserPtr<LinuxTimes>) -> usize {
    let ticks = crate::kernel::watchdog::global_tick();
    let ns_per_tick = crate::config::KernelConfig::time_slice();
    let total_ns = ticks * ns_per_tick;

    // Linux expect times in "clock ticks" which is USER_HZ (usually 100)
    let user_hz = 100;
    let user_ticks = (total_ns * user_hz / 1_000_000_000) as usize;

    if !tms_ptr.is_null() {
        #[cfg(feature = "posix_process")]
        let (utime_ticks, stime_ticks) = match crate::modules::posix::process::getrusage(0) {
            // RUSAGE_SELF
            Ok(ru) => (ru.ru_utime_ticks, ru.ru_stime_ticks),
            Err(_) => (ticks, 0),
        };
        #[cfg(not(feature = "posix_process"))]
        let (utime_ticks, stime_ticks) = (ticks, 0);

        let tms = LinuxTimes {
            tms_utime: (utime_ticks * ns_per_tick * user_hz / 1_000_000_000) as i64,
            tms_stime: (stime_ticks * ns_per_tick * user_hz / 1_000_000_000) as i64,
            tms_cutime: 0,
            tms_cstime: 0,
        };
        let _ = tms_ptr.write(&tms);
    }
    user_ticks
}

pub fn sys_linux_syslog(type_: usize, buf: UserPtr<u8>, len: usize) -> usize {
    match type_ {
        3 => {
            // Read all messages remaining in ring buffer (0 = none)
            if buf.is_null() || len == 0 {
                return 0;
            }
            let mut tmp = alloc::vec::Vec::with_capacity(len);
            tmp.resize(len, 0);
            let read = crate::kernel::log::read_to_buffer(&mut tmp);
            if let Err(e) = buf.write_bytes(&tmp[..read]) {
                return e;
            }
            read
        }
        10 => {
            // Get size of whole buffer
            crate::kernel::log::get_total_size()
        }
        _ => 0,
    }
}

pub fn sys_linux_sethostname(ptr: UserPtr<u8>, len: usize) -> usize {
    crate::require_posix_process!((ptr, len) => {
        if len == 0 || len > 256 {
                    return linux_inval();
                }
                let mut name = alloc::vec![0u8; len];
                if let Err(e) = ptr.read_bytes(&mut name) {
                    return e;
                }
                if let Ok(s) = core::str::from_utf8(&name) {
                    if let Err(e) = crate::modules::posix::process::sethostname(s) {
                        return linux_errno(e as i32);
                    }
                    0
                } else {
                    linux_inval()
                }
    })
}

pub fn sys_linux_setdomainname(ptr: UserPtr<u8>, len: usize) -> usize {
    crate::require_posix_process!((ptr, len) => {
        if len == 0 || len > 256 {
                    return linux_inval();
                }
                let mut name = alloc::vec![0u8; len];
                if let Err(e) = ptr.read_bytes(&mut name) {
                    return e;
                }
                if let Ok(s) = core::str::from_utf8(&name) {
                    if let Err(e) = crate::modules::posix::process::setdomainname(s) {
                        return linux_errno(e as i32);
                    }
                    0
                } else {
                    linux_inval()
                }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn getrandom_invalid_flags_return_einval() {
        assert_eq!(sys_linux_getrandom(UserPtr::new(0), 16, 0x8000), linux_inval());
    }
}
