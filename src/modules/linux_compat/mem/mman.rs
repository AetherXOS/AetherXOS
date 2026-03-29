use super::super::*;

#[cfg(all(
    feature = "vfs",
    feature = "posix_mman",
    feature = "process_abstraction"
))]
fn map_and_register(map_id: u32, len: usize, prot: u32, flags: u32) -> Option<usize> {
    if let Some(pid) = current_process_id() {
        if let Some(process) =
            crate::kernel::launch::process_arc_by_id(crate::interfaces::task::ProcessId(pid))
        {
            if let Ok(start) = process.allocate_user_vaddr(len) {
                let page_size = crate::interfaces::memory::PAGE_SIZE_4K as u64;
                let end = start + (((len as u64 + page_size - 1) / page_size) * page_size);
                let _ = process.register_mapping(map_id, start, end, prot, flags);
                return Some(start as usize);
            }
        }
    }
    None
}

#[inline(always)]
fn resolve_map_id_from_addr(addr: usize) -> u32 {
    let mut map_id = addr as u32;
    #[cfg(feature = "process_abstraction")]
    if let Some(pid) = current_process_id() {
        if let Some(process) =
            crate::kernel::launch::process_arc_by_id(crate::interfaces::task::ProcessId(pid))
        {
            if let Some(record) = process.lookup_mapping(addr as u64) {
                map_id = record.map_id;
            }
        }
    }
    map_id
}

pub fn sys_linux_mmap(
    _addr: UserPtr<u8>,
    len: usize,
    prot: usize,
    flags: usize,
    fd: Fd,
    offset: usize,
) -> usize {
    if len == 0 {
        return linux_inval();
    }

    let fd_val = fd.as_usize();
    if fd_val == linux::FB_FD {
        if let Some(fb) = crate::hal::framebuffer() {
            return fb.address.as_ptr().unwrap() as usize;
        }
    }

    // MAP_ANONYMOUS mapping
    if (flags & linux::mmap::MAP_ANONYMOUS) != 0 {
        #[cfg(all(
            feature = "vfs",
            feature = "posix_mman",
            feature = "process_abstraction"
        ))]
        {
            match crate::modules::posix::mman::mmap_anonymous(len, prot as u32, flags as u32) {
                Ok(map_id) => {
                    if let Some(start) = map_and_register(map_id, len, prot as u32, flags as u32) {
                        return start;
                    }
                    map_id as usize
                }
                Err(err) => linux_errno(err.code()),
            }
        }
        #[cfg(not(all(
            feature = "vfs",
            feature = "posix_mman",
            feature = "process_abstraction"
        )))]
        {
            let _ = (len, prot, flags, fd, offset);
            // Soft fallback for environments without full MMU backend.
            // Return a deterministic pseudo-address aligned to 4K.
            0x4000_0000usize
        }
    } else {
        #[cfg(all(feature = "vfs", feature = "posix_mman"))]
        {
            if fd.0 < 0 {
                return linux_errno(crate::modules::posix_consts::errno::EBADF);
            }

            let fs_id = match crate::modules::posix::fs::fd_fs_context(fd.as_u32()) {
                Ok(id) => id,
                Err(err) => return linux_errno(err.code()),
            };

            let path = match crate::modules::posix::fs::fd_path(fd.as_u32()) {
                Ok(p) => p,
                Err(err) => return linux_errno(err.code()),
            };

            match crate::modules::posix::mman::mmap_file(
                fs_id,
                &path,
                offset,
                len,
                prot as u32,
                flags as u32,
            ) {
                Ok(map_id) => {
                    #[cfg(feature = "process_abstraction")]
                    if let Some(start) = map_and_register(map_id, len, prot as u32, flags as u32) {
                        return start;
                    }
                    map_id as usize
                }
                Err(err) => linux_errno(err.code()),
            }
        }

        #[cfg(not(all(feature = "vfs", feature = "posix_mman")))]
        {
            let _ = (_addr, len, prot, flags, fd, offset);
            0x4000_0000usize
        }
    }
}

pub fn sys_linux_munmap(addr: UserPtr<u8>, _len: usize) -> usize {
    if addr.is_null() {
        return linux_inval();
    }

    #[cfg(all(feature = "vfs", feature = "posix_mman"))]
    {
        let addr_val = addr.addr;
        let map_id = resolve_map_id_from_addr(addr_val);
        #[cfg(feature = "process_abstraction")]
        if let Some(pid) = current_process_id() {
            if let Some(process) =
                crate::kernel::launch::process_arc_by_id(crate::interfaces::task::ProcessId(pid))
            {
                process.remove_mapping(map_id);
            }
        }
        match crate::modules::posix::mman::munmap(map_id) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }

    #[cfg(not(all(feature = "vfs", feature = "posix_mman")))]
    {
        let _ = (addr, _len);
        0
    }
}

pub fn sys_linux_mprotect(addr: UserPtr<u8>, _len: usize, prot: usize) -> usize {
    if addr.is_null() {
        return linux_inval();
    }

    #[cfg(all(feature = "vfs", feature = "posix_mman"))]
    {
        let map_id = resolve_map_id_from_addr(addr.addr);
        match crate::modules::posix::mman::mprotect(map_id, prot as u32) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }

    #[cfg(not(all(feature = "vfs", feature = "posix_mman")))]
    {
        let _ = (addr, _len, prot);
        0
    }
}

pub fn sys_linux_mremap(
    old_addr: UserPtr<u8>,
    old_size: usize,
    new_size: usize,
    _flags: usize,
) -> usize {
    #[cfg(all(feature = "vfs", feature = "posix_mman"))]
    {
        let map_id = resolve_map_id_from_addr(old_addr.addr);
        let _ = old_size;
        match crate::modules::posix::mman::mremap(map_id, new_size) {
            Ok(()) => old_addr.addr, // return same address
            Err(e) => linux_errno(e.code()),
        }
    }
    #[cfg(not(all(feature = "vfs", feature = "posix_mman")))]
    {
        let _ = (old_addr, old_size, new_size, _flags);
        old_addr.addr
    }
}

pub fn sys_linux_madvise(addr: UserPtr<u8>, length: usize, advice: usize) -> usize {
    #[cfg(all(feature = "vfs", feature = "posix_mman"))]
    {
        let _ = length;
        let map_id = resolve_map_id_from_addr(addr.addr);
        match crate::modules::posix::mman::madvise(map_id, advice as i32) {
            Ok(()) => 0,
            Err(e) => linux_errno(e.code()),
        }
    }
    #[cfg(not(all(feature = "vfs", feature = "posix_mman")))]
    {
        let _ = (addr, length, advice);
        0
    }
}

pub fn sys_linux_mlock(addr: UserPtr<u8>, len: usize) -> usize {
    if addr.is_null() || len == 0 {
        return linux_inval();
    }
    #[cfg(all(feature = "vfs", feature = "posix_mman"))]
    {
        let map_id = resolve_map_id_from_addr(addr.addr);
        match crate::modules::posix::mman::mlock(map_id) {
            Ok(()) => 0,
            Err(e) => linux_errno(e.code()),
        }
    }
    #[cfg(not(all(feature = "vfs", feature = "posix_mman")))]
    {
        let _ = (addr, len);
        0
    }
}

pub fn sys_linux_munlock(addr: UserPtr<u8>, len: usize) -> usize {
    if addr.is_null() || len == 0 {
        return linux_inval();
    }
    #[cfg(all(feature = "vfs", feature = "posix_mman"))]
    {
        let map_id = resolve_map_id_from_addr(addr.addr);
        match crate::modules::posix::mman::munlock(map_id) {
            Ok(()) => 0,
            Err(e) => linux_errno(e.code()),
        }
    }
    #[cfg(not(all(feature = "vfs", feature = "posix_mman")))]
    {
        let _ = (addr, len);
        0
    }
}

pub fn sys_linux_mlockall(flags: usize) -> usize {
    #[cfg(feature = "posix_mman")]
    {
        match crate::modules::posix::mman::mlockall(flags as u32) {
            Ok(()) => 0,
            Err(e) => linux_errno(e.code()),
        }
    }
    #[cfg(not(feature = "posix_mman"))]
    {
        let _ = flags;
        0
    }
}

pub fn sys_linux_munlockall() -> usize {
    #[cfg(feature = "posix_mman")]
    {
        crate::modules::posix::mman::munlockall();
        0
    }
    #[cfg(not(feature = "posix_mman"))]
    {
        0
    }
}
