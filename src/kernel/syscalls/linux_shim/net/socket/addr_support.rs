#[cfg(feature = "posix_net")]
use crate::kernel::syscalls::{linux_errno, with_user_read_bytes, with_user_write_bytes};

#[cfg(feature = "posix_net")]
pub(super) fn read_sockaddr_len(len_ptr: usize) -> Result<usize, usize> {
    with_user_read_bytes(len_ptr, core::mem::size_of::<u32>(), |src| {
        u32::from_ne_bytes([src[0], src[1], src[2], src[3]]) as usize
    })
    .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
}

#[cfg(feature = "posix_net")]
pub(super) fn write_sockaddr_len(len_ptr: usize, len: usize) -> usize {
    with_user_write_bytes(len_ptr, core::mem::size_of::<u32>(), |dst| {
        dst.copy_from_slice(&(len as u32).to_ne_bytes());
        0usize
    })
    .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
}

#[cfg(all(test, feature = "posix_net"))]
mod tests {
    use super::*;

    #[test_case]
    fn sockaddr_len_reader_reports_efault_for_invalid_pointer() {
        assert_eq!(
            read_sockaddr_len(0),
            Err(linux_errno(crate::modules::posix_consts::errno::EFAULT))
        );
    }

    #[test_case]
    fn sockaddr_len_writer_reports_efault_for_invalid_pointer() {
        assert_eq!(
            write_sockaddr_len(0, 16),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }
}
