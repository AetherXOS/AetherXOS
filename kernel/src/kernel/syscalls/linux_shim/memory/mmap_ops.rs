#[cfg(all(feature = "vfs", feature = "posix_mman"))]
use super::mapping_helpers::resolve_map_id_from_addr;
#[cfg(all(
    not(feature = "linux_compat"),
    feature = "vfs",
    feature = "posix_mman",
    feature = "process_abstraction"
))]
use super::mmap_support::checked_rounded_mapping_len;
#[cfg(not(feature = "linux_compat"))]
use super::mmap_support::is_map_fixed;
use super::mmap_support::{
    validate_mmap_len, validate_mremap_request, validate_nonzero_mapping_range,
    validate_protection_request,
};
use super::*;

#[cfg(all(
    not(feature = "linux_compat"),
    feature = "vfs",
    feature = "posix_mman",
    feature = "process_abstraction"
))]
fn register_mapping_for_current_process(
    requested_addr: usize,
    map_id: u32,
    len: usize,
    prot: u32,
    flags: u32,
) -> Result<Option<usize>, usize> {
    let Some(pid) = current_process_id() else {
        return Ok(None);
    };
    let Some(process) =
        crate::kernel::launch::process_arc_by_id(crate::interfaces::task::ProcessId(pid))
    else {
        return Ok(None);
    };
    let rounded_len = checked_rounded_mapping_len(len)?;
    let start = if is_map_fixed(flags as usize) {
        let start = requested_addr as u64;
        let end = start + rounded_len;
        let overlaps = process.overlapping_mappings(start, end);
        for record in overlaps {
            process.remove_mapping(record.map_id);
            let _ = crate::modules::posix::mman::munmap(record.map_id);
        }
        start
    } else {
        let Ok(start) = process.allocate_user_vaddr(len) else {
            return Ok(None);
        };
        start
    };
    let end = start + rounded_len;
    let _ = process.register_mapping(map_id, start, end, prot, flags);
    let cg = process
        .cgroup_id
        .load(core::sync::atomic::Ordering::Relaxed);
    if !crate::kernel::cgroups::cgroup_charge_memory(cg, len as u64) {
        process.remove_mapping(map_id);
        let _ = crate::modules::posix::mman::munmap(map_id);
        return Err(linux_errno(crate::modules::posix_consts::errno::ENOMEM));
    }

    Ok(Some(start as usize))
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_mmap(
    _addr: usize,
    len: usize,
    prot: usize,
    flags: usize,
    fd: usize,
    offset: usize,
) -> usize {
    if let Err(err) = validate_mmap_len(len) {
        return err;
    }
    if is_map_fixed(flags) {
        if let Err(err) = validate_nonzero_mapping_range(_addr, len) {
            return err;
        }
        let page_size = crate::interfaces::memory::PAGE_SIZE_4K;
        if (_addr & (page_size - 1)) != 0 {
            return linux_errno(crate::modules::posix_consts::errno::EINVAL);
        }
    }

    if (flags as u32) & crate::modules::posix_consts::mman::MAP_ANONYMOUS != 0 {
        #[cfg(all(
            feature = "vfs",
            feature = "posix_mman",
            feature = "process_abstraction"
        ))]
        {
            match crate::modules::posix::mman::mmap_anonymous(len, prot as u32, flags as u32) {
                Ok(map_id) => {
                    match register_mapping_for_current_process(
                        _addr,
                        map_id,
                        len,
                        prot as u32,
                        flags as u32,
                    ) {
                        Ok(Some(start)) => return start,
                        Ok(None) => {}
                        Err(err) => return err,
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
            linux_errno(crate::modules::posix_consts::errno::ENOMEM)
        }
    } else {
        #[cfg(all(feature = "vfs", feature = "posix_mman"))]
        {
            if (fd as isize) < 0 {
                return linux_errno(crate::modules::posix_consts::errno::EBADF);
            }

            let fs_id = match crate::modules::posix::fs::fd_fs_context(fd as u32) {
                Ok(id) => id,
                Err(err) => return linux_errno(err.code()),
            };

            let path = match crate::modules::posix::fs::fd_path(fd as u32) {
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
                    match register_mapping_for_current_process(
                        _addr,
                        map_id,
                        len,
                        prot as u32,
                        flags as u32,
                    ) {
                        Ok(Some(start)) => return start,
                        Ok(None) => {}
                        Err(err) => return err,
                    }
                    map_id as usize
                }
                Err(err) => linux_errno(err.code()),
            }
        }

        #[cfg(not(all(feature = "vfs", feature = "posix_mman")))]
        {
            let _ = (_addr, len, prot, flags, fd, offset);
            linux_errno(crate::modules::posix_consts::errno::ENOMEM)
        }
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_munmap(addr: usize, len: usize) -> usize {
    if let Err(err) = validate_nonzero_mapping_range(addr, len) {
        return err;
    }

    #[cfg(all(feature = "vfs", feature = "posix_mman"))]
    {
        let map_id = resolve_map_id_from_addr(addr);
        match crate::modules::posix::mman::munmap(map_id) {
            Ok(()) => {
                if let Some(pid) = current_process_id() {
                    if let Some(process) = crate::kernel::launch::process_arc_by_id(
                        crate::interfaces::task::ProcessId(pid),
                    ) {
                        let removed = process.remove_mapping_record(map_id);
                        let bytes = removed
                            .map(|r| r.end.saturating_sub(r.start))
                            .unwrap_or(len as u64);
                        if bytes > 0 {
                            let cg = process
                                .cgroup_id
                                .load(core::sync::atomic::Ordering::Relaxed);
                            crate::kernel::cgroups::cgroup_uncharge_memory(cg, bytes);
                        }
                    }
                }
                0
            }
            Err(err) => linux_errno(err.code()),
        }
    }

    #[cfg(not(all(feature = "vfs", feature = "posix_mman")))]
    {
        let _ = (addr, len);
        0
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_mprotect(addr: usize, _len: usize, prot: usize) -> usize {
    if let Err(err) = validate_protection_request(addr, _len) {
        return err;
    }

    #[cfg(all(feature = "vfs", feature = "posix_mman"))]
    {
        let map_id = resolve_map_id_from_addr(addr);
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

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_mremap(
    old_addr: usize,
    old_size: usize,
    new_size: usize,
    flags: usize,
    new_addr: usize,
) -> usize {
    if let Err(err) = validate_mremap_request(old_addr, old_size, new_size, flags, new_addr) {
        return err;
    }

    #[cfg(all(feature = "vfs", feature = "posix_mman"))]
    {
        let map_id = resolve_map_id_from_addr(old_addr);
        match crate::modules::posix::mman::mremap(map_id, new_size) {
            Ok(()) => {
                if let Some(pid) = current_process_id() {
                    if let Some(process) = crate::kernel::launch::process_arc_by_id(
                        crate::interfaces::task::ProcessId(pid),
                    ) {
                        if let Some(mut rec) = process.remove_mapping_record(map_id) {
                            let rounded_len = match checked_rounded_mapping_len(new_size) {
                                Ok(v) => v,
                                Err(err) => return err,
                            };
                            if (flags & super::mmap_support::MREMAP_FIXED) != 0 {
                                let new_start = new_addr as u64;
                                let new_end = new_start + rounded_len;
                                for overlap in process.overlapping_mappings(new_start, new_end) {
                                    if overlap.map_id != map_id {
                                        process.remove_mapping(overlap.map_id);
                                        let _ = crate::modules::posix::mman::munmap(overlap.map_id);
                                    }
                                }
                                rec.start = new_start;
                            }
                            let new_end = rec.start + rounded_len;
                            rec.end = new_end;
                            let _ = process.register_mapping(
                                rec.map_id, rec.start, rec.end, rec.prot, rec.flags,
                            );
                            return rec.start as usize;
                        }
                    }
                }
                old_addr
            }
            Err(err) => linux_errno(err.code()),
        }
    }

    #[cfg(not(all(feature = "vfs", feature = "posix_mman")))]
    {
        let _ = (old_addr, old_size, new_size, flags, new_addr);
        old_addr
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_madvise(addr: usize, len: usize, advice: usize) -> usize {
    if let Err(err) = validate_protection_request(addr, len) {
        return err;
    }

    #[cfg(all(feature = "vfs", feature = "posix_mman"))]
    {
        let map_id = resolve_map_id_from_addr(addr);
        match crate::modules::posix::mman::madvise(map_id, advice as i32) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }

    #[cfg(not(all(feature = "vfs", feature = "posix_mman")))]
    {
        let _ = (addr, len, advice);
        0
    }
}

#[cfg(all(test, not(feature = "linux_compat")))]
mod tests {
    use super::*;

    #[test_case]
    fn mmap_rejects_zero_length() {
        assert_eq!(
            sys_linux_mmap(0, 0, 0, 0, 0, 0),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }

    #[test_case]
    fn memory_ops_reject_zero_addresses() {
        assert_eq!(
            sys_linux_munmap(0, 4096),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
        assert_eq!(
            sys_linux_munmap(4096, 0),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
        assert_eq!(
            sys_linux_mprotect(0, 4096, 0),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
        assert_eq!(
            sys_linux_mprotect(4096, 0, 0),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
        assert_eq!(
            sys_linux_madvise(0, 4096, 0),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
        assert_eq!(
            sys_linux_madvise(4096, 0, 0),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }

    #[test_case]
    fn mremap_rejects_zero_sizes_and_addresses() {
        assert_eq!(
            sys_linux_mremap(0, 4096, 8192, 0, 0),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
        assert_eq!(
            sys_linux_mremap(4096, 0, 8192, 0, 0),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
        assert_eq!(
            sys_linux_mremap(4096, 4096, 0, 0, 0),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }

    #[test_case]
    fn mremap_rejects_nonzero_flags_or_new_addr() {
        assert_eq!(
            sys_linux_mremap(4096, 4096, 8192, 1, 0),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
        assert_eq!(
            sys_linux_mremap(4096, 4096, 8192, 0, 0x2000),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }
}
