use crate::kernel::syscalls::{with_user_read_bytes, with_user_write_bytes};

pub fn sys_linux_prlimit64(
    pid: usize,
    resource: usize,
    new_limit: usize,
    old_limit: usize,
) -> usize {
    crate::require_posix_process!((pid, resource, new_limit, old_limit) => {
        // Get current value first
                let current = crate::modules::posix::process::prlimit(pid, resource as i32, None);
                if let Ok((soft, hard)) = current {
                    if old_limit != 0 {
                        let _ = with_user_write_bytes(old_limit, 16, |dst| {
                            dst[0..8].copy_from_slice(&soft.to_ne_bytes());
                            dst[8..16].copy_from_slice(&hard.to_ne_bytes());
                            0
                        });
                    }
                }
                if new_limit != 0 {
                    let mut new_soft = 0u64;
                    let mut new_hard = 0u64;
                    let _ = with_user_read_bytes(new_limit, 16, |src| {
                        new_soft = u64::from_ne_bytes(src[0..8].try_into().unwrap_or([0;8]));
                        new_hard = u64::from_ne_bytes(src[8..16].try_into().unwrap_or([0;8]));
                        0
                    });
                    let _ = crate::modules::posix::process::prlimit(pid, resource as i32, Some((new_soft, new_hard)));
                }
                0
    })
}
pub fn sys_linux_getrlimit(resource: usize, rlim_ptr: usize) -> usize {
    sys_linux_prlimit64(0, resource, 0, rlim_ptr)
}

pub fn sys_linux_setrlimit(resource: usize, rlim_ptr: usize) -> usize {
    sys_linux_prlimit64(0, resource, rlim_ptr, 0)
}
