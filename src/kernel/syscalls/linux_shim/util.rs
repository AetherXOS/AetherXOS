use super::*;

#[inline(always)]
#[cfg(not(feature = "linux_compat"))]
pub(super) fn arg5_to_zero(value: usize) -> usize {
    value
}

#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
pub(super) fn read_user_path_like_string(ptr: usize) -> Result<alloc::string::String, usize> {
    read_user_c_string(ptr, crate::config::KernelConfig::syscall_max_path_len())
}

#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
pub(super) fn read_user_c_string(
    ptr: usize,
    max_len: usize,
) -> Result<alloc::string::String, usize> {
    if ptr < USER_SPACE_BOTTOM_INCLUSIVE || ptr >= USER_SPACE_TOP_EXCLUSIVE || max_len == 0 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EFAULT));
    }

    let mut out = alloc::vec::Vec::new();
    for i in 0..max_len {
        let Some(addr) = ptr.checked_add(i) else {
            return Err(linux_errno(crate::modules::posix_consts::errno::EFAULT));
        };
        if !user_readable_range_valid(addr, 1) {
            return Err(linux_errno(crate::modules::posix_consts::errno::EFAULT));
        }
        let b = unsafe { *(addr as *const u8) };
        if b == 0 {
            if out.is_empty() {
                return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
            }
            return alloc::string::String::from_utf8(out)
                .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EINVAL));
        }
        out.push(b);
    }

    Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
}

#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
pub(super) fn read_user_c_string_allow_empty(
    ptr: usize,
    max_len: usize,
) -> Result<alloc::string::String, usize> {
    if ptr < USER_SPACE_BOTTOM_INCLUSIVE || ptr >= USER_SPACE_TOP_EXCLUSIVE || max_len == 0 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EFAULT));
    }

    let mut out = alloc::vec::Vec::new();
    for i in 0..max_len {
        let Some(addr) = ptr.checked_add(i) else {
            return Err(linux_errno(crate::modules::posix_consts::errno::EFAULT));
        };
        if !user_readable_range_valid(addr, 1) {
            return Err(linux_errno(crate::modules::posix_consts::errno::EFAULT));
        }
        let b = unsafe { *(addr as *const u8) };
        if b == 0 {
            return alloc::string::String::from_utf8(out)
                .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EINVAL));
        }
        out.push(b);
    }

    Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
}

#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
pub(super) fn read_user_usize_word(ptr: usize) -> Result<usize, usize> {
    with_user_read_bytes(ptr, core::mem::size_of::<usize>(), |src| {
        let mut tmp = [0u8; core::mem::size_of::<usize>()];
        tmp.copy_from_slice(src);
        usize::from_ne_bytes(tmp)
    })
}

#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
pub(crate) fn read_user_c_string_array(
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
