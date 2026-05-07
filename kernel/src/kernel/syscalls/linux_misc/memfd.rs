use crate::kernel::syscalls::linux_errno;
use super::utils::*;

pub fn sys_linux_memfd_create(name_ptr: usize, flags: usize) -> usize {
    use crate::kernel::syscalls::syscalls_consts::linux::memfd_flags::{
        MFD_ALLOW_SEALING, MFD_CLOEXEC, MFD_EXEC, MFD_HUGETLB, MFD_NOEXEC_SEAL,
    };

    let known_flags =
        MFD_CLOEXEC | MFD_ALLOW_SEALING | MFD_HUGETLB | MFD_NOEXEC_SEAL | MFD_EXEC | crate::kernel::syscalls::syscalls_consts::linux::MFD_HUGE_MASK;
    if (flags & !known_flags) != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if (flags & MFD_EXEC) != 0 && (flags & MFD_NOEXEC_SEAL) != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    let raw_name = if name_ptr == 0 {
        alloc::string::String::from("memfd")
    } else {
        match read_user_c_string_compat(name_ptr, 255) {
            Ok(v) if !v.is_empty() => v,
            Ok(_) => alloc::string::String::from("memfd"),
            Err(e) => return e,
        }
    };

    #[cfg(feature = "posix_fs")]
    {
        use core::sync::atomic::{AtomicU32, Ordering};

        static NEXT_MEMFD_ID: AtomicU32 = AtomicU32::new(1);

        let id = NEXT_MEMFD_ID.fetch_add(1, Ordering::Relaxed);
        let path = alloc::format!("/.memfd-{}-{}", id, raw_name.replace('/', "_"));
        let fs_id = match crate::modules::posix::fs::default_fs_id() {
            Ok(v) => v,
            Err(e) => return linux_errno(e.code()),
        };
        match crate::modules::posix::fs::openat(fs_id, "/", &path, true) {
            Ok(fd) => fd as usize,
            Err(e) => linux_errno(e.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let id = NEXT_MEMFD_SYNTH_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        let fd = (MEMFD_FD_BASE as u32).saturating_add(id);
        MEMFD_NAME_BY_FD.lock().insert(fd, raw_name);
        fd as usize
    }
}
