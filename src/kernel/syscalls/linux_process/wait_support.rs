#[cfg(all(not(feature = "linux_compat"), feature = "posix_process"))]
use crate::kernel::syscalls::linux_errno;
#[cfg(all(not(feature = "linux_compat"), feature = "posix_process"))]
use crate::kernel::syscalls::with_user_write_bytes;

#[cfg(all(not(feature = "linux_compat"), feature = "posix_process"))]
const LINUX_WAIT_NOHANG: usize = 0x1;
#[cfg(all(not(feature = "linux_compat"), feature = "posix_process"))]
const LINUX_SIGINFO_LEN: usize = 128;
#[cfg(all(not(feature = "linux_compat"), feature = "posix_process"))]
const LINUX_SIGINFO_SIGNO_OFFSET: usize = 0;
#[cfg(all(not(feature = "linux_compat"), feature = "posix_process"))]
const LINUX_SIGINFO_PID_OFFSET: usize = 16;

#[cfg(all(not(feature = "linux_compat"), feature = "posix_process"))]
pub(super) fn decode_wait_options(options: usize) -> Result<bool, usize> {
    if (options & !LINUX_WAIT_NOHANG) != 0 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }
    Ok((options & LINUX_WAIT_NOHANG) != 0)
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_process"))]
pub(super) fn decode_waitid_target(idtype: usize, id: usize) -> Result<usize, usize> {
    match idtype {
        0 => Ok(usize::MAX),
        1 | 2 => Ok(id),
        _ => Err(linux_errno(crate::modules::posix_consts::errno::EINVAL)),
    }
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_process"))]
pub(super) fn should_write_wait_status(ptr: usize) -> bool {
    ptr != 0
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_process"))]
pub(super) fn write_wait_status(status_ptr: usize, status: u32) -> Result<(), usize> {
    with_user_write_bytes(status_ptr, core::mem::size_of::<u32>(), |dst| {
        dst.copy_from_slice(&status.to_ne_bytes());
        0
    })
    .map(|_| ())
    .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_process"))]
pub(super) fn write_waitid_info(infop: usize, child_pid: usize) -> Result<(), usize> {
    with_user_write_bytes(infop, LINUX_SIGINFO_LEN, |dst| {
        dst.fill(0);
        dst[LINUX_SIGINFO_SIGNO_OFFSET..LINUX_SIGINFO_SIGNO_OFFSET + 4]
            .copy_from_slice(&17u32.to_ne_bytes());
        dst[LINUX_SIGINFO_PID_OFFSET..LINUX_SIGINFO_PID_OFFSET + 4]
            .copy_from_slice(&(child_pid as u32).to_ne_bytes());
        0
    })
    .map(|_| ())
    .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
}

#[cfg(all(test, not(feature = "linux_compat"), feature = "posix_process"))]
fn decode_waitid_info_bytes(raw: &[u8; LINUX_SIGINFO_LEN]) -> (u32, u32) {
    let signo = u32::from_ne_bytes(
        raw[LINUX_SIGINFO_SIGNO_OFFSET..LINUX_SIGINFO_SIGNO_OFFSET + 4]
            .try_into()
            .expect("signo bytes"),
    );
    let pid = u32::from_ne_bytes(
        raw[LINUX_SIGINFO_PID_OFFSET..LINUX_SIGINFO_PID_OFFSET + 4]
            .try_into()
            .expect("pid bytes"),
    );
    (signo, pid)
}

#[cfg(all(test, not(feature = "linux_compat"), feature = "posix_process"))]
mod tests {
    use super::*;

    #[test_case]
    fn wait_option_decoder_rejects_unknown_bits() {
        assert_eq!(
            decode_wait_options(0x2).unwrap_err(),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }

    #[test_case]
    fn waitid_target_decoder_accepts_supported_idtypes() {
        assert_eq!(decode_waitid_target(0, 77), Ok(usize::MAX));
        assert_eq!(decode_waitid_target(1, 77), Ok(77));
        assert_eq!(decode_waitid_target(2, 88), Ok(88));
    }

    #[test_case]
    fn wait_status_writer_helper_matches_null_pointer_convention() {
        assert!(!should_write_wait_status(0));
        assert!(should_write_wait_status(4));
    }

    #[test_case]
    fn wait_status_writer_roundtrips_exit_status() {
        let mut raw = [0u8; core::mem::size_of::<u32>()];
        assert_eq!(write_wait_status(raw.as_mut_ptr() as usize, 0x1234), Ok(()));
        assert_eq!(u32::from_ne_bytes(raw), 0x1234);
    }

    #[test_case]
    fn wait_status_writer_reports_efault_for_invalid_pointer() {
        assert_eq!(
            write_wait_status(1, 0),
            Err(linux_errno(crate::modules::posix_consts::errno::EFAULT))
        );
    }

    #[test_case]
    fn waitid_info_writer_reports_efault_for_invalid_pointer() {
        assert_eq!(
            write_waitid_info(1, 42),
            Err(linux_errno(crate::modules::posix_consts::errno::EFAULT))
        );
    }

    #[test_case]
    fn waitid_info_writer_encodes_siginfo_shape() {
        let mut raw = [0u8; LINUX_SIGINFO_LEN];
        assert_eq!(write_waitid_info(raw.as_mut_ptr() as usize, 42), Ok(()));
        let (signo, pid) = decode_waitid_info_bytes(&raw);
        assert_eq!(signo, 17);
        assert_eq!(pid, 42);
        assert!(raw[20..].iter().all(|byte| *byte == 0));
    }

    #[test_case]
    fn waitid_target_decoder_rejects_unknown_idtypes() {
        assert_eq!(
            decode_waitid_target(99, 7),
            Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
        );
    }
}
