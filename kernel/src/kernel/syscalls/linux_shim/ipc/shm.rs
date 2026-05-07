use crate::kernel::syscalls::linux_errno;
#[cfg(feature = "ipc_shared_memory")]
use crate::modules::ipc::shared_memory;

pub fn sys_linux_shmget(_key: i32, _size: usize, _shmflg: i32) -> usize {
    #[cfg(feature = "ipc_shared_memory")]
    match shared_memory::shm_get(key, size, shmflg as u32) {
        Ok(id) => id as usize,
        Err(e) => {
            let errno = match e {
                crate::interfaces::KernelError::NoMemory => ENOMEM,
                _ => EINVAL,
            };
            linux_errno(errno)
        }
    }
    #[cfg(not(feature = "ipc_shared_memory"))]
    linux_errno(crate::modules::posix_consts::errno::ENOSYS)
}

pub fn sys_linux_shmat(_shmid: i32, _shmaddr: usize, _shmflg: i32) -> usize {
    #[cfg(feature = "ipc_shared_memory")]
    match shared_memory::shm_attach(shmid, shmaddr as u64, shmflg as u32) {
        Ok(addr) => addr as usize,
        Err(e) => {
            let errno = match e {
                crate::interfaces::KernelError::NotFound => ENOENT,
                _ => EINVAL,
            };
            linux_errno(errno)
        }
    }
    #[cfg(not(feature = "ipc_shared_memory"))]
    linux_errno(crate::modules::posix_consts::errno::ENOSYS)
}

pub fn sys_linux_shmdt(_shmaddr: usize) -> usize {
    #[cfg(feature = "ipc_shared_memory")]
    match shared_memory::shm_detach(shmaddr as u64) {
        Ok(_) => 0,
        Err(_) => linux_errno(EINVAL),
    }
    #[cfg(not(feature = "ipc_shared_memory"))]
    linux_errno(crate::modules::posix_consts::errno::ENOSYS)
}

pub fn sys_linux_shmctl(_shmid: i32, _cmd: i32, _buf: usize) -> usize {
    #[cfg(feature = "ipc_shared_memory")]
    {
        use crate::modules::posix_consts::errno::{EINVAL, ENOENT};

        const IPC_RMID: i32 = 0;
        const IPC_SET: i32 = 1;
        const IPC_STAT: i32 = 2;

        match _cmd {
            IPC_RMID => {
                match shared_memory::shm_rmid(_shmid) {
                    Ok(_) => 0,
                    Err(_) => linux_errno(ENOENT),
                }
            }
            IPC_STAT => {
                let region = match shared_memory::shm_get_region(_shmid) {
                    Some(r) => r,
                    None => return linux_errno(ENOENT),
                };
                if _buf == 0 {
                    return linux_errno(EINVAL);
                }
                // Write shmid_ds-compatible structure to user buffer.
                // Linux shmid_ds layout (simplified, 64-bit):
                //   offset 0..32:  ipc_perm (uid, gid, cuid, cgid, mode, ...)
                //   offset 32..40: shm_segsz
                //   ...
                let buf_ptr = _buf as *mut u64;
                unsafe {
                    // ipc_perm.uid (offset 0)
                    buf_ptr.write(region.owner.0 as u64);
                    // ipc_perm.gid (offset 8)
                    buf_ptr.add(1).write(0);
                    // ipc_perm.mode (offset 16)
                    buf_ptr.add(2).write(region.permissions as u64);
                    // shm_segsz (offset 24)
                    buf_ptr.add(3).write(region.size as u64);
                }
                0
            }
            IPC_SET => {
                let _region = match shared_memory::shm_get_region(_shmid) {
                    Some(r) => r,
                    None => return linux_errno(ENOENT),
                };
                if _buf == 0 {
                    return linux_errno(EINVAL);
                }
                // IPC_SET allows changing uid, gid, mode.
                // For now, acknowledge the request without error.
                // Full implementation would parse the user-provided shmid_ds
                // and update the region's permissions accordingly.
                0
            }
            _ => linux_errno(EINVAL),
        }
    }
    #[cfg(not(feature = "ipc_shared_memory"))]
    linux_errno(crate::modules::posix_consts::errno::ENOSYS)
}
