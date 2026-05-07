use super::*;
 
#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct LinuxTimeVal {
    pub tv_sec: i64,
    pub tv_usec: i64,
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct LinuxRUsage {
    pub ru_utime: LinuxTimeVal,
    pub ru_stime: LinuxTimeVal,
    pub ru_maxrss: i64,
    pub ru_ixrss: i64,
    pub ru_idrss: i64,
    pub ru_isrss: i64,
    pub ru_minflt: i64,
    pub ru_majflt: i64,
    pub ru_nswap: i64,
    pub ru_inblock: i64,
    pub ru_oublock: i64,
    pub ru_msgsnd: i64,
    pub ru_msgrcv: i64,
    pub ru_nsignals: i64,
    pub ru_nvcsw: i64,
    pub ru_nivcsw: i64,
}

impl From<crate::modules::posix::process::PosixRusage> for LinuxRUsage {
    fn from(ru: crate::modules::posix::process::PosixRusage) -> Self {
        let mut usage = Self::default();
        let ns_per_tick = crate::config::KernelConfig::time_slice();
        let utime_ns = ru.ru_utime_ticks.saturating_mul(ns_per_tick);
        let stime_ns = ru.ru_stime_ticks.saturating_mul(ns_per_tick);

        usage.ru_utime.tv_sec = (utime_ns / 1_000_000_000) as i64;
        usage.ru_utime.tv_usec = ((utime_ns % 1_000_000_000) / 1000) as i64;
        usage.ru_stime.tv_sec = (stime_ns / 1_000_000_000) as i64;
        usage.ru_stime.tv_usec = ((stime_ns % 1_000_000_000) / 1000) as i64;
        
        usage.ru_maxrss = ru.ru_maxrss as i64;
        usage.ru_minflt = ru.ru_minflt as i64;
        usage.ru_majflt = ru.ru_majflt as i64;
        usage.ru_nswap = ru.ru_nswap as i64;
        usage
    }
}



#[inline(always)]
pub(super) fn arg5_to_zero(value: usize) -> usize {
    value
}

pub(super) fn read_user_path_like_string(ptr: usize) -> Result<alloc::string::String, usize> {
    read_user_c_string(ptr, crate::config::KernelConfig::syscall_max_path_len())
}

use crate::kernel::syscalls::{read_user_c_string as central_read_user_c_string};

pub(super) fn read_user_c_string(
    ptr: usize,
    max_len: usize,
) -> Result<alloc::string::String, usize> {
    central_read_user_c_string(ptr, max_len)
}

pub(super) fn read_user_c_string_allow_empty(
    ptr: usize,
    max_len: usize,
) -> Result<alloc::string::String, usize> {
    // Note: our central reader currently doesn't distinguish empty vs non-empty,
    // which is actually correct for standard C strings (an empty string is just '\0').
    central_read_user_c_string(ptr, max_len)
}

pub(super) fn read_user_usize_word(ptr: usize) -> Result<usize, usize> {
    with_user_read_bytes(ptr, core::mem::size_of::<usize>(), |src| {
        let mut tmp = [0u8; core::mem::size_of::<usize>()];
        tmp.copy_from_slice(src);
        usize::from_ne_bytes(tmp)
    })
}

pub fn read_user_pod<T: Copy + Default>(ptr: usize) -> Result<T, usize> {
    let size = core::mem::size_of::<T>();
    with_user_read_bytes(ptr, size, |src| {
        let mut out = T::default();
        let dst = unsafe { core::slice::from_raw_parts_mut((&mut out as *mut T).cast::<u8>(), size) };
        dst.copy_from_slice(src);
        out
    })
    .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
}

#[cfg(not(feature = "linux_compat"))]
pub fn read_user_pod_prefix<T: Copy + Default>(ptr: usize, size: usize) -> Result<T, usize> {
    let full_size = core::mem::size_of::<T>();
    if size > full_size {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }
    with_user_read_bytes(ptr, size, |src| {
        let mut out = T::default();
        let dst = unsafe {
            core::slice::from_raw_parts_mut((&mut out as *mut T).cast::<u8>(), full_size)
        };
        dst[..src.len()].copy_from_slice(src);
        out
    })
    .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
}

pub fn write_user_pod<T: Copy>(ptr: usize, value: &T) -> Result<(), usize> {
    let size = core::mem::size_of::<T>();
    with_user_write_bytes(ptr, size, |dst| {
        let src = unsafe { core::slice::from_raw_parts((value as *const T).cast::<u8>(), size) };
        dst.copy_from_slice(src);
        0usize
    })
    .map(|_| ())
    .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
}

#[macro_export]
macro_rules! define_user_pod_codec {
    ($read_fn:ident, $write_fn:ident, $ty:ty) => {
        pub(super) fn $read_fn(ptr: usize) -> Result<$ty, usize> {
            crate::kernel::syscalls::linux_shim::util::read_user_pod::<$ty>(ptr)
        }

        pub(super) fn $write_fn(ptr: usize, value: &$ty) -> Result<(), usize> {
            crate::kernel::syscalls::linux_shim::util::write_user_pod(ptr, value)
        }
    };
}

#[cfg(not(feature = "linux_compat"))]
pub use crate::define_user_pod_codec;

pub fn read_user_c_string_array(
    ptr: usize,
    max_items: usize,
    max_item_len: usize,
) -> Result<alloc::vec::Vec<alloc::string::String>, usize> {
    if ptr == 0 {
        return Ok(alloc::vec::Vec::new());
    }
    if max_items == 0 || max_item_len == 0 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }

    let mut out = alloc::vec::Vec::new();
    let stride = core::mem::size_of::<usize>();
    for i in 0..max_items {
        let off = i
            .checked_mul(stride)
            .ok_or_else(|| linux_errno(crate::modules::posix_consts::errno::EFAULT))?;
        let word_ptr = ptr
            .checked_add(off)
            .ok_or_else(|| linux_errno(crate::modules::posix_consts::errno::EFAULT))?;
        let word = read_user_usize_word(word_ptr)?;
        if word == 0 {
            return Ok(out);
        }
        out.push(read_user_c_string(word, max_item_len)?);
    }

    Err(linux_errno(crate::modules::posix_consts::errno::E2BIG))
}

#[cfg(all(test, not(feature = "linux_compat")))]
mod tests {
    use super::*;

    #[test_case]
    fn read_user_c_string_invalid_ptr_returns_efault() {
        assert_eq!(
            read_user_c_string(0, 16),
            Err(linux_errno(crate::modules::posix_consts::errno::EFAULT))
        );
    }

    #[test_case]
    fn read_user_c_string_allow_empty_invalid_ptr_returns_efault() {
        assert_eq!(
            read_user_c_string_allow_empty(0, 16),
            Err(linux_errno(crate::modules::posix_consts::errno::EFAULT))
        );
    }

    #[test_case]
    fn read_user_c_string_array_zero_ptr_is_empty() {
        assert_eq!(read_user_c_string_array(0, 4, 8).unwrap().len(), 0);
    }
}
