use super::super::super::*;

#[cfg(not(feature = "linux_compat"))]
pub(super) fn validate_linux_msg_flags(flags: usize) -> Result<(), usize> {
    if (flags & !0xffff_ffffusize) != 0 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }
    Ok(())
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn validate_iov_len(iov_len: usize) -> Result<(), usize> {
    if iov_len > 1024 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }
    Ok(())
}

#[cfg(all(test, not(feature = "linux_compat")))]
mod tests {
    use super::*;

    #[test_case]
    fn msg_flag_validator_rejects_values_outside_linux_u32_space() {
        assert_eq!(
            validate_linux_msg_flags(1usize << 40),
            Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
        );
        assert_eq!(validate_linux_msg_flags(0xffff_ffffusize), Ok(()));
    }

    #[test_case]
    fn iov_len_validator_rejects_excessive_counts() {
        assert_eq!(validate_iov_len(1024), Ok(()));
        assert_eq!(
            validate_iov_len(1025),
            Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
        );
    }
}
