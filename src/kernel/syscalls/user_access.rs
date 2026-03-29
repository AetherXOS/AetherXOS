use super::*;
use crate::kernel::syscalls::syscalls_user::{
    user_range_valid, user_word_aligned, UserAccessFault, UserAccessMode,
};

#[cfg(target_arch = "x86_64")]
use x86_64::structures::paging::{PageTable, PageTableFlags};

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn entry_user_fault(flags: PageTableFlags, mode: UserAccessMode) -> Option<UserAccessFault> {
    if !flags.contains(PageTableFlags::PRESENT) {
        return Some(UserAccessFault::NotPresent);
    }
    if !flags.contains(PageTableFlags::USER_ACCESSIBLE) {
        return Some(UserAccessFault::NotUserAccessible);
    }
    if matches!(mode, UserAccessMode::Write) && !flags.contains(PageTableFlags::WRITABLE) {
        return Some(UserAccessFault::NotWritable);
    }
    None
}

#[cfg(target_arch = "x86_64")]
fn phys_to_virt_table(phys: u64, hhdm: u64) -> Option<&'static PageTable> {
    let virt = phys.checked_add(hhdm)?;
    let ptr = virt as *const PageTable;
    Some(unsafe { &*ptr })
}

#[cfg(target_arch = "x86_64")]
fn user_page_access_fault(addr: usize, mode: UserAccessMode) -> Option<UserAccessFault> {
    let Some(hhdm) = crate::hal::x86_64::hhdm_offset() else {
        return Some(UserAccessFault::HhdmMissing);
    };

    let virt = x86_64::VirtAddr::new(addr as u64);
    let root_frame_addr = crate::hal::cpu::ArchCpuRegisters::read_page_table_root();

    let Some(p4) = phys_to_virt_table(root_frame_addr, hhdm) else {
        return Some(UserAccessFault::PageTableUnavailable);
    };
    let p4e = &p4[virt.p4_index()];
    let p4f = p4e.flags();
    if let Some(fault) = entry_user_fault(p4f, mode) {
        return Some(fault);
    }

    let Some(p3) = phys_to_virt_table(p4e.addr().as_u64(), hhdm) else {
        return Some(UserAccessFault::PageTableUnavailable);
    };
    let p3e = &p3[virt.p3_index()];
    let p3f = p3e.flags();
    if let Some(fault) = entry_user_fault(p3f, mode) {
        return Some(fault);
    }
    if p3f.contains(PageTableFlags::HUGE_PAGE) {
        return None;
    }

    let Some(p2) = phys_to_virt_table(p3e.addr().as_u64(), hhdm) else {
        return Some(UserAccessFault::PageTableUnavailable);
    };
    let p2e = &p2[virt.p2_index()];
    let p2f = p2e.flags();
    if let Some(fault) = entry_user_fault(p2f, mode) {
        return Some(fault);
    }
    if p2f.contains(PageTableFlags::HUGE_PAGE) {
        return None;
    }

    let Some(p1) = phys_to_virt_table(p2e.addr().as_u64(), hhdm) else {
        return Some(UserAccessFault::PageTableUnavailable);
    };
    let p1e = &p1[virt.p1_index()];
    entry_user_fault(p1e.flags(), mode)
}

#[cfg(not(target_arch = "x86_64"))]
fn user_page_access_fault(_addr: usize, _mode: UserAccessMode) -> Option<UserAccessFault> {
    if _addr >= NON_X86_USER_SPACE_TOP_EXCLUSIVE {
        return Some(UserAccessFault::NotUserAccessible);
    }
    None
}

pub(crate) fn user_access_range_check_with(
    ptr: usize,
    len: usize,
    mode: UserAccessMode,
    mut page_access_check: impl FnMut(usize, UserAccessMode) -> Option<UserAccessFault>,
) -> Result<(), UserAccessFault> {
    if !user_range_valid(ptr, len) {
        return Err(UserAccessFault::InvalidRange);
    }

    let Some(end_inclusive) = ptr.checked_add(len - 1) else {
        return Err(UserAccessFault::Overflow);
    };
    let mut page = ptr & !(PAGE_SIZE - 1);
    let last_page = end_inclusive & !(PAGE_SIZE - 1);

    loop {
        if let Some(fault) = page_access_check(page, mode) {
            return Err(fault);
        }
        if page == last_page {
            break;
        }
        let Some(next) = page.checked_add(PAGE_SIZE) else {
            return Err(UserAccessFault::Overflow);
        };
        page = next;
    }

    Ok(())
}

#[cfg(test)]
pub(crate) fn user_access_range_valid_with(
    ptr: usize,
    len: usize,
    mode: UserAccessMode,
    mut page_access_check: impl FnMut(usize, UserAccessMode) -> bool,
) -> bool {
    user_access_range_check_with(ptr, len, mode, |page, access_mode| {
        if page_access_check(page, access_mode) {
            None
        } else {
            Some(UserAccessFault::NotPresent)
        }
    })
    .is_ok()
}

pub(crate) fn user_access_range_valid(ptr: usize, len: usize, mode: UserAccessMode) -> bool {
    user_access_range_check_with(ptr, len, mode, user_page_access_fault).is_ok()
}

#[inline(always)]
pub(crate) fn user_readable_range_valid(ptr: usize, len: usize) -> bool {
    user_access_range_valid(ptr, len, UserAccessMode::Read)
}

#[inline(always)]
pub(crate) fn user_writable_range_valid(ptr: usize, len: usize) -> bool {
    user_access_range_valid(ptr, len, UserAccessMode::Write)
}

#[inline(always)]
pub(crate) fn with_user_read_bytes<T>(
    ptr: usize,
    len: usize,
    f: impl FnOnce(&[u8]) -> T,
) -> Result<T, usize> {
    if !user_readable_range_valid(ptr, len) {
        return Err(user_access_denied_arg());
    }
    let slice = unsafe { core::slice::from_raw_parts(ptr as *const u8, len) };
    Ok(f(slice))
}

#[inline(always)]
pub(crate) fn with_user_read_bounded_bytes<T>(
    ptr: usize,
    len: usize,
    max_len: usize,
    f: impl FnOnce(&[u8]) -> T,
) -> Result<T, usize> {
    if len == 0 || len > max_len {
        return Err(invalid_arg());
    }
    with_user_read_bytes(ptr, len, f)
}

#[inline(always)]
#[allow(dead_code)]
pub(crate) fn with_user_write_bytes<T>(
    ptr: usize,
    len: usize,
    f: impl FnOnce(&mut [u8]) -> T,
) -> Result<T, usize> {
    if len == 0 || !user_writable_range_valid(ptr, len) {
        return Err(user_access_denied_arg());
    }
    let slice = unsafe { core::slice::from_raw_parts_mut(ptr as *mut u8, len) };
    Ok(f(slice))
}

#[inline(always)]
pub(crate) fn with_user_write_words<T>(
    ptr: usize,
    len: usize,
    required_words: usize,
    f: impl FnOnce(&mut [usize]) -> T,
) -> Result<T, usize> {
    let required = required_bytes(required_words);
    if !user_word_aligned(ptr) {
        return Err(user_word_unaligned_denied_arg());
    }
    if len < required || !user_writable_range_valid(ptr, len) {
        return Err(user_access_denied_arg());
    }
    let words = unsafe { core::slice::from_raw_parts_mut(ptr as *mut usize, required_words) };
    Ok(f(words))
}

#[inline(always)]
pub(crate) fn with_user_write_words_exact<T>(
    ptr: usize,
    len: usize,
    words_len: usize,
    f: impl FnOnce(&mut [usize]) -> T,
) -> Result<T, usize> {
    let required = required_bytes(words_len);
    if !user_word_aligned(ptr) {
        return Err(user_word_unaligned_denied_arg());
    }
    if len < required || !user_writable_range_valid(ptr, len) {
        return Err(user_access_denied_arg());
    }
    let words = unsafe { core::slice::from_raw_parts_mut(ptr as *mut usize, words_len) };
    Ok(f(words))
}

#[cfg(feature = "vfs")]
#[inline(always)]
pub(super) fn with_user_vfs_path<T>(
    path_ptr: usize,
    path_len: usize,
    f: impl FnOnce(&[u8]) -> T,
) -> Result<T, usize> {
    with_user_read_bounded_bytes(
        path_ptr,
        path_len,
        crate::config::KernelConfig::vfs_max_mount_path(),
        f,
    )
}

#[inline(always)]
pub(crate) fn invalid_arg() -> usize {
    SYSCALL_INVALID_ARGS.fetch_add(1, Ordering::Relaxed);
    SYSCALL_ERR_INVALID_ARG
}

#[inline(always)]
pub(super) fn user_access_denied_arg() -> usize {
    SYSCALL_USER_ACCESS_DENIED.fetch_add(1, Ordering::Relaxed);
    SYSCALL_ERR_USER_ACCESS_DENIED
}

#[inline(always)]
pub(super) fn user_word_unaligned_denied_arg() -> usize {
    SYSCALL_USER_WORD_UNALIGNED_DENIED.fetch_add(1, Ordering::Relaxed);
    user_access_denied_arg()
}

#[inline(always)]
pub(crate) fn permission_denied_arg() -> usize {
    SYSCALL_USER_ACCESS_DENIED.fetch_add(1, Ordering::Relaxed);
    SYSCALL_ERR_PERMISSION_DENIED
}

#[inline(always)]
pub(crate) fn require_control_plane_access(resource: u64) -> Result<(), usize> {
    if crate::modules::security::check_control_plane_access(resource) {
        Ok(())
    } else {
        Err(permission_denied_arg())
    }
}
