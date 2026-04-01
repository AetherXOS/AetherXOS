#[cfg(all(not(feature = "linux_compat"), feature = "posix_process"))]
use super::wait_support::{
    decode_wait_options, decode_waitid_target, should_write_wait_status, write_wait_status,
    write_waitid_info,
};
#[cfg(not(feature = "linux_compat"))]
use crate::kernel::syscalls::linux_errno;

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_wait4(
    pid: isize,
    wstatus_ptr: usize,
    options: usize,
    _rusage_ptr: usize,
) -> usize {
    #[cfg(feature = "posix_process")]
    {
        let nohang = match decode_wait_options(options) {
            Ok(v) => v,
            Err(err) => return err,
        };
        match crate::modules::posix::process::waitpid(pid as usize, nohang) {
            Ok(Some(child_pid)) => {
                if should_write_wait_status(wstatus_ptr) {
                    if let Err(err) = write_wait_status(wstatus_ptr, 0) {
                        return err;
                    }
                }
                child_pid
            }
            Ok(None) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_process"))]
    {
        let _ = (pid, wstatus_ptr, options, _rusage_ptr);
        linux_errno(crate::modules::posix_consts::errno::ECHILD)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_waitid(idtype: usize, id: usize, infop: usize, options: usize) -> usize {
    #[cfg(feature = "posix_process")]
    {
        let pid = match decode_waitid_target(idtype, id) {
            Ok(v) => v,
            Err(err) => return err,
        };
        let nohang = match decode_wait_options(options) {
            Ok(v) => v,
            Err(err) => return err,
        };
        match crate::modules::posix::process::waitpid(pid, nohang) {
            Ok(Some(child_pid)) => {
                if should_write_wait_status(infop) {
                    if let Err(err) = write_waitid_info(infop, child_pid) {
                        return err;
                    }
                }
                0
            }
            Ok(None) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_process"))]
    {
        let _ = (idtype, id, infop, options);
        linux_errno(crate::modules::posix_consts::errno::ECHILD)
    }
}

#[cfg(all(test, not(feature = "linux_compat"), feature = "posix_process"))]
mod tests {
    use super::*;

    #[test_case]
    fn wait4_invalid_status_pointer_returns_efault() {
        assert_eq!(
            sys_linux_wait4(-1, 1, 0, 0),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[test_case]
    fn waitid_invalid_info_pointer_returns_efault() {
        assert_eq!(
            sys_linux_waitid(0, 0, 1, 0),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[test_case]
    fn waitid_rejects_invalid_idtype() {
        assert_eq!(
            sys_linux_waitid(99, 0, 0, 0),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }

    #[test_case]
    fn wait4_nohang_without_children_returns_zero() {
        assert_eq!(sys_linux_wait4(-1, 0, 1, 0), 0);
    }

    #[test_case]
    fn waitid_nohang_without_children_returns_zero() {
        assert_eq!(sys_linux_waitid(0, 0, 0, 1), 0);
    }

    #[test_case]
    fn wait4_nonblocking_invalid_status_pointer_still_reports_efault() {
        assert_eq!(
            sys_linux_wait4(-1, 1, 1, 0),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[test_case]
    fn waitid_rejects_unknown_option_bits() {
        assert_eq!(
            sys_linux_waitid(0, 0, 0, 0x2),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }

    #[test_case]
    fn wait4_rejects_unknown_option_bits() {
        assert_eq!(
            sys_linux_wait4(-1, 0, 0x2, 0),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }
}
