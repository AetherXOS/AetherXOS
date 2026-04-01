use super::*;

#[cfg(not(feature = "linux_compat"))]
pub(super) const MREMAP_MAYMOVE: usize = 1;
#[cfg(not(feature = "linux_compat"))]
pub(super) const MREMAP_FIXED: usize = 2;

#[cfg(not(feature = "linux_compat"))]
pub(super) fn is_map_fixed(flags: usize) -> bool {
    (flags & crate::kernel::syscalls::syscalls_consts::linux::mmap::MAP_FIXED) != 0
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn validate_mmap_len(len: usize) -> Result<(), usize> {
    if len == 0 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }
    Ok(())
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn validate_nonzero_mapping_range(addr: usize, len: usize) -> Result<(), usize> {
    if addr == 0 || len == 0 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }
    Ok(())
}

#[cfg(any(
    all(
        not(feature = "linux_compat"),
        feature = "vfs",
        feature = "posix_mman",
        feature = "process_abstraction"
    ),
    test
))]
pub(super) fn checked_rounded_mapping_len(len: usize) -> Result<u64, usize> {
    const PAGE_SIZE: u64 = crate::interfaces::memory::PAGE_SIZE_4K as u64;
    let len =
        u64::try_from(len).map_err(|_| linux_errno(crate::modules::posix_consts::errno::EINVAL))?;
    let rounded = len
        .checked_add(PAGE_SIZE - 1)
        .ok_or_else(|| linux_errno(crate::modules::posix_consts::errno::EINVAL))?
        / PAGE_SIZE
        * PAGE_SIZE;
    if rounded == 0 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }
    Ok(rounded)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn validate_mremap_request(
    old_addr: usize,
    old_size: usize,
    new_size: usize,
    flags: usize,
    new_addr: usize,
) -> Result<(), usize> {
    validate_nonzero_mapping_range(old_addr, old_size)?;
    if new_size == 0 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }
    let allowed = MREMAP_MAYMOVE | MREMAP_FIXED;
    if (flags & !allowed) != 0 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }
    let fixed = (flags & MREMAP_FIXED) != 0;
    let maymove = (flags & MREMAP_MAYMOVE) != 0;
    if fixed && !maymove {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }
    if fixed {
        validate_nonzero_mapping_range(new_addr, new_size)?;
        let page_size = crate::interfaces::memory::PAGE_SIZE_4K;
        if (new_addr & (page_size - 1)) != 0 {
            return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
        }
    } else if new_addr != 0 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }
    Ok(())
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn validate_protection_request(addr: usize, len: usize) -> Result<(), usize> {
    validate_nonzero_mapping_range(addr, len)
}

#[cfg(any(
    all(
        not(feature = "linux_compat"),
        all(feature = "vfs", feature = "posix_mman")
    ),
    test
))]
pub(super) fn decode_mlockall_flags(flags: usize) -> Result<u32, usize> {
    let allowed = crate::modules::posix_consts::mman::MCL_CURRENT
        | crate::modules::posix_consts::mman::MCL_FUTURE;
    let flags_u32 = u32::try_from(flags)
        .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EINVAL))?;
    if (flags_u32 & !allowed) != 0 || flags_u32 == 0 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }
    Ok(flags_u32)
}

#[cfg(all(test, not(feature = "linux_compat")))]
mod tests {
    use super::*;

    #[test_case]
    fn mmap_len_validator_rejects_zero_length() {
        assert_eq!(
            validate_mmap_len(0),
            Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
        );
        assert_eq!(validate_mmap_len(1), Ok(()));
    }

    #[test_case]
    fn nonzero_mapping_range_validator_rejects_zero_values() {
        assert_eq!(
            validate_nonzero_mapping_range(0, 4096),
            Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
        );
        assert_eq!(
            validate_nonzero_mapping_range(4096, 0),
            Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
        );
        assert_eq!(validate_nonzero_mapping_range(4096, 4096), Ok(()));
    }

    #[test_case]
    fn rounded_mapping_len_rounds_up_and_rejects_overflow() {
        assert_eq!(
            checked_rounded_mapping_len(0),
            Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
        );
        assert_eq!(checked_rounded_mapping_len(1), Ok(4096));
        assert_eq!(checked_rounded_mapping_len(4097), Ok(8192));
        assert_eq!(
            checked_rounded_mapping_len(usize::MAX),
            Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
        );
    }

    #[test_case]
    fn mremap_request_validator_accepts_supported_shape() {
        assert_eq!(validate_mremap_request(4096, 4096, 8192, 0, 0), Ok(()));
        assert_eq!(
            validate_mremap_request(4096, 4096, 8192, MREMAP_MAYMOVE, 0),
            Ok(())
        );
        assert_eq!(
            validate_mremap_request(4096, 4096, 8192, MREMAP_MAYMOVE | MREMAP_FIXED, 0x8000),
            Ok(())
        );
    }

    #[test_case]
    fn mremap_request_validator_rejects_zero_new_size_and_flagged_moves() {
        assert_eq!(
            validate_mremap_request(4096, 4096, 0, 0, 0),
            Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
        );
        assert_eq!(
            validate_mremap_request(4096, 4096, 8192, MREMAP_FIXED, 0x8000),
            Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
        );
        assert_eq!(
            validate_mremap_request(4096, 4096, 8192, 0, 0x4000),
            Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
        );
        assert_eq!(
            validate_mremap_request(4096, 4096, 8192, MREMAP_MAYMOVE | MREMAP_FIXED, 123),
            Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
        );
    }

    #[test_case]
    fn protection_request_validator_rejects_zero_length_or_address() {
        assert_eq!(
            validate_protection_request(0, 4096),
            Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
        );
        assert_eq!(
            validate_protection_request(4096, 0),
            Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
        );
        assert_eq!(validate_protection_request(4096, 4096), Ok(()));
    }

    #[test_case]
    fn mlockall_flag_decoder_accepts_supported_combinations_and_rejects_zero() {
        assert_eq!(
            decode_mlockall_flags(crate::modules::posix_consts::mman::MCL_CURRENT as usize),
            Ok(crate::modules::posix_consts::mman::MCL_CURRENT)
        );
        assert_eq!(
            decode_mlockall_flags(
                (crate::modules::posix_consts::mman::MCL_CURRENT
                    | crate::modules::posix_consts::mman::MCL_FUTURE) as usize
            ),
            Ok(crate::modules::posix_consts::mman::MCL_CURRENT
                | crate::modules::posix_consts::mman::MCL_FUTURE)
        );
        assert_eq!(
            decode_mlockall_flags(0),
            Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
        );
    }
}
