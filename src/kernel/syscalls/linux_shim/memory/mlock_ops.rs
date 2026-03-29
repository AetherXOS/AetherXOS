#[cfg(all(feature = "vfs", feature = "posix_mman"))]
use super::mapping_helpers::resolve_map_id_from_addr;
#[cfg(any(
    all(
        not(feature = "linux_compat"),
        all(feature = "vfs", feature = "posix_mman")
    ),
    test
))]
use super::mmap_support::decode_mlockall_flags;
use super::mmap_support::validate_nonzero_mapping_range;
#[cfg(any(all(feature = "vfs", feature = "posix_mman"), test))]
use super::*;

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_mlock(addr: usize, len: usize) -> usize {
    if let Err(err) = validate_nonzero_mapping_range(addr, len) {
        return err;
    }

    #[cfg(all(feature = "vfs", feature = "posix_mman"))]
    {
        let map_id = resolve_map_id_from_addr(addr);
        match crate::modules::posix::mman::mlock(map_id) {
            Ok(()) => 0,
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
pub(crate) fn sys_linux_munlock(addr: usize, len: usize) -> usize {
    if let Err(err) = validate_nonzero_mapping_range(addr, len) {
        return err;
    }

    #[cfg(all(feature = "vfs", feature = "posix_mman"))]
    {
        let map_id = resolve_map_id_from_addr(addr);
        match crate::modules::posix::mman::munlock(map_id) {
            Ok(()) => 0,
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
pub(crate) fn sys_linux_mlockall(flags: usize) -> usize {
    #[cfg(all(feature = "vfs", feature = "posix_mman"))]
    {
        let flags_u32 = match decode_mlockall_flags(flags) {
            Ok(v) => v,
            Err(err) => return err,
        };
        match crate::modules::posix::mman::mlockall(flags_u32) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }

    #[cfg(not(all(feature = "vfs", feature = "posix_mman")))]
    {
        let _ = flags;
        0
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_munlockall() -> usize {
    #[cfg(all(feature = "vfs", feature = "posix_mman"))]
    {
        crate::modules::posix::mman::munlockall();
        0
    }

    #[cfg(not(all(feature = "vfs", feature = "posix_mman")))]
    {
        0
    }
}

#[cfg(all(test, not(feature = "linux_compat")))]
mod tests {
    use super::*;

    #[test_case]
    fn lock_ops_share_mapping_range_validator_contract() {
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
    fn mlock_rejects_zero_addr_or_len() {
        assert_eq!(
            sys_linux_mlock(0, 4096),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
        assert_eq!(
            sys_linux_mlock(4096, 0),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }

    #[test_case]
    fn munlock_rejects_zero_addr_or_len() {
        assert_eq!(
            sys_linux_munlock(0, 4096),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
        assert_eq!(
            sys_linux_munlock(4096, 0),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }

    #[test_case]
    fn mlockall_rejects_invalid_flag_sets() {
        assert_eq!(
            sys_linux_mlockall(0),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
        assert_eq!(
            sys_linux_mlockall(usize::MAX),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }

    #[test_case]
    fn mlockall_flag_decoder_accepts_supported_combinations() {
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
    }

    #[cfg(all(
        feature = "linux_shim_noop_mlock",
        not(all(feature = "vfs", feature = "posix_mman"))
    ))]
    #[test_case]
    fn mlock_family_returns_success_when_noop_feature_is_enabled() {
        assert_eq!(sys_linux_mlock(0x1000, 0x1000), 0);
        assert_eq!(sys_linux_munlock(0x1000, 0x1000), 0);
        assert_eq!(
            sys_linux_mlockall(crate::modules::posix_consts::mman::MCL_CURRENT as usize),
            0
        );
        assert_eq!(sys_linux_munlockall(), 0);
    }

    #[cfg(all(
        not(feature = "linux_shim_noop_mlock"),
        not(all(feature = "vfs", feature = "posix_mman"))
    ))]
    #[test_case]
    fn mlock_family_uses_soft_success_without_noop_feature() {
        assert_eq!(sys_linux_mlock(0x1000, 0x1000), 0);
        assert_eq!(sys_linux_munlock(0x1000, 0x1000), 0);
        assert_eq!(
            sys_linux_mlockall(crate::modules::posix_consts::mman::MCL_CURRENT as usize),
            0
        );
        assert_eq!(sys_linux_munlockall(), 0);
    }
}
