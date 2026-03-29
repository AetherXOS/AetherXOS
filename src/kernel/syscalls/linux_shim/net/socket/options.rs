use super::super::super::*;
#[cfg(feature = "posix_net")]
use super::addr::write_sockaddr_in;
#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
use crate::kernel::syscalls::linux_shim::util::{read_user_pod, write_user_pod};

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
fn read_sockopt_len(optlen_ptr: usize) -> Result<usize, usize> {
    read_user_pod::<u32>(optlen_ptr).map(|v| v as usize)
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
fn read_sockopt_value(optval_ptr: usize, optlen: usize) -> Result<u64, usize> {
    if optlen == 0 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }

    if optlen >= core::mem::size_of::<u64>() {
        read_user_pod::<u64>(optval_ptr)
    } else if optlen >= core::mem::size_of::<u32>() {
        read_user_pod::<u32>(optval_ptr).map(u64::from)
    } else {
        Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
    }
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
fn write_sockopt_value(
    optval_ptr: usize,
    optlen_ptr: usize,
    caller_len: usize,
    value: u64,
) -> Result<(), usize> {
    if optval_ptr == 0 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EFAULT));
    }

    let out_len = caller_len.min(core::mem::size_of::<u64>());
    let wr_val = with_user_write_bytes(optval_ptr, out_len, |dst| {
        dst.copy_from_slice(&value.to_ne_bytes()[..out_len]);
        0usize
    })
    .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT));
    if wr_val != 0 {
        return Err(wr_val);
    }

    write_user_pod(optlen_ptr, &(out_len as u32))
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_getsockname(fd: usize, addr_ptr: usize, len_ptr: usize) -> usize {
    #[cfg(feature = "posix_net")]
    {
        let addr = match crate::modules::libnet::posix_getsockname_errno(fd as u32) {
            Ok(v) => v,
            Err(err) => return linux_errno(err.code()),
        };
        write_sockaddr_in(addr_ptr, len_ptr, addr)
    }
    #[cfg(not(feature = "posix_net"))]
    {
        let _ = (fd, addr_ptr, len_ptr);
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_getpeername(fd: usize, addr_ptr: usize, len_ptr: usize) -> usize {
    #[cfg(feature = "posix_net")]
    {
        let addr = match crate::modules::libnet::posix_getpeername_errno(fd as u32) {
            Ok(v) => v,
            Err(err) => return linux_errno(err.code()),
        };
        write_sockaddr_in(addr_ptr, len_ptr, addr)
    }
    #[cfg(not(feature = "posix_net"))]
    {
        let _ = (fd, addr_ptr, len_ptr);
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_setsockopt(
    fd: usize,
    level: usize,
    optname: usize,
    optval_ptr: usize,
    optlen: usize,
) -> usize {
    #[cfg(feature = "posix_net")]
    {
        let value = match read_sockopt_value(optval_ptr, optlen) {
            Ok(value) => value,
            Err(err) => return err,
        };

        match crate::modules::posix::net::setsockopt_raw(
            fd as u32,
            level as i32,
            optname as i32,
            value,
        ) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_net"))]
    {
        let _ = (fd, level, optname, optval_ptr, optlen);
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_getsockopt(
    fd: usize,
    level: usize,
    optname: usize,
    optval_ptr: usize,
    optlen_ptr: usize,
) -> usize {
    #[cfg(feature = "posix_net")]
    {
        let _efault = linux_errno(crate::modules::posix_consts::errno::EFAULT);
        let caller_len = match read_sockopt_len(optlen_ptr) {
            Ok(len) => len,
            Err(err) => return err,
        };
        if caller_len == 0 {
            return linux_errno(crate::modules::posix_consts::errno::EINVAL);
        }

        let value = match crate::modules::posix::net::getsockopt_raw(
            fd as u32,
            level as i32,
            optname as i32,
        ) {
            Ok(v) => v,
            Err(err) => return linux_errno(err.code()),
        };

        match write_sockopt_value(optval_ptr, optlen_ptr, caller_len, value) {
            Ok(()) => 0,
            Err(err) => err,
        }
    }
    #[cfg(not(feature = "posix_net"))]
    {
        let _ = (fd, level, optname, optval_ptr, optlen_ptr);
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}

#[cfg(all(test, not(feature = "linux_compat")))]
mod tests {
    use super::*;

    #[cfg(feature = "posix_net")]
    #[test_case]
    fn read_sockopt_value_rejects_short_lengths() {
        assert_eq!(
            read_sockopt_value(0, core::mem::size_of::<u16>()),
            Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
        );
    }

    #[cfg(feature = "posix_net")]
    #[test_case]
    fn read_sockopt_len_invalid_pointer_returns_efault() {
        assert_eq!(
            read_sockopt_len(0),
            Err(linux_errno(crate::modules::posix_consts::errno::EFAULT))
        );
    }

    #[test_case]
    fn setsockopt_invalid_value_pointer_returns_efault() {
        assert_eq!(
            sys_linux_setsockopt(
                0,
                crate::modules::posix_consts::net::SOL_SOCKET as usize,
                crate::modules::posix_consts::net::SO_REUSEADDR as usize,
                0,
                core::mem::size_of::<u32>(),
            ),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[test_case]
    fn getsockopt_invalid_length_pointer_returns_efault() {
        assert_eq!(
            sys_linux_getsockopt(
                0,
                crate::modules::posix_consts::net::SOL_SOCKET as usize,
                crate::modules::posix_consts::net::SO_TYPE as usize,
                0,
                0,
            ),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[test_case]
    fn getsockopt_zero_length_pointer_value_returns_efault_before_semantics() {
        assert_eq!(
            sys_linux_getsockopt(
                0,
                crate::modules::posix_consts::net::SOL_SOCKET as usize,
                crate::modules::posix_consts::net::SO_TYPE as usize,
                1,
                0,
            ),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[cfg(feature = "posix_net")]
    #[test_case]
    fn read_sockopt_value_zero_length_returns_einval() {
        assert_eq!(
            read_sockopt_value(0, 0),
            Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
        );
    }

    #[cfg(feature = "posix_net")]
    #[test_case]
    fn write_sockopt_value_successfully_clamps_and_updates_length() {
        let mut out = [0u8; 8];
        let mut len = 4u32;
        assert_eq!(
            write_sockopt_value(
                out.as_mut_ptr() as usize,
                (&mut len as *mut u32) as usize,
                len as usize,
                0x1122_3344_5566_7788,
            ),
            Ok(())
        );
        assert_eq!(len, 4);
        assert_eq!(&out[..4], &0x1122_3344_5566_7788u64.to_ne_bytes()[..4]);
    }

    #[cfg(feature = "posix_net")]
    #[test_case]
    fn write_sockopt_value_rejects_null_output_pointer() {
        let mut len = 8u32;
        assert_eq!(
            write_sockopt_value(0, (&mut len as *mut u32) as usize, 8, 1),
            Err(linux_errno(crate::modules::posix_consts::errno::EFAULT))
        );
    }
}
