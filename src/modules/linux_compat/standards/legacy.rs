use super::super::*;
use crate::modules::linux_compat::config::LinuxCompatConfig;

/// `ipc(2)` — System V IPC multiplexer (Legacy/32-bit style).
/// This is used by older glibc/libc versions.
pub fn sys_linux_ipc(
    call: usize,
    first: usize,
    second: usize,
    third: usize,
    ptr: UserPtr<u8>,
    fifth: usize,
) -> usize {
    // Call types (from Linux kernel):
    // SEMOP: 1, SEMGET: 2, SEMCTL: 3, MSGSND: 11, MSGRCV: 12, MSGGET: 13, MSGCTL: 14, SHMAT: 21, SHMDT: 22, SHMGET: 23, SHMCTL: 24
    match call {
        1 => {
            // SEMOP
            crate::modules::linux_compat::ipc::sys_linux_semop(first as i32, ptr.cast(), second)
        }
        2 => {
            // SEMGET
            crate::modules::linux_compat::ipc::sys_linux_semget(
                first as i32,
                second as i32,
                third as i32,
            )
        }
        3 => {
            // SEMCTL
            crate::modules::linux_compat::ipc::sys_linux_semctl(
                first as i32,
                second as i32,
                third as i32,
                fifth,
            )
        }
        11 => {
            // MSGSND
            crate::modules::linux_compat::ipc::sys_linux_msgsnd(
                first as i32,
                ptr.cast(),
                second,
                third as i32,
            )
        }
        12 => {
            // MSGRCV
            // For MSGRCV, fifth is often the type
            crate::modules::linux_compat::ipc::sys_linux_msgrcv(
                first as i32,
                ptr.cast(),
                second,
                fifth as i64,
                third as i32,
            )
        }
        13 => {
            // MSGGET
            crate::modules::linux_compat::ipc::sys_linux_msgget(first as i32, second as i32)
        }
        14 => {
            // MSGCTL (minimal compatibility)
            // Queue control is not yet fully modeled in linux_compat; return success for
            // commonly tolerated control paths and EINVAL for unknown commands.
            match second as i32 {
                0 | 1 | 2 => 0,
                _ => linux_inval(),
            }
        }
        21 => {
            // SHMAT
            // Note: address returned in 'third' or returned value
            crate::modules::linux_compat::ipc::sys_linux_shmat(
                first as i32,
                ptr.cast(),
                second as i32,
            )
        }
        22 => {
            // SHMDT
            crate::modules::linux_compat::ipc::sys_linux_shmdt(ptr.cast())
        }
        23 => {
            // SHMGET
            crate::modules::linux_compat::ipc::sys_linux_shmget(first as i32, second, third as i32)
        }
        24 => {
            // SHMCTL
            crate::modules::linux_compat::ipc::sys_linux_shmctl(
                first as i32,
                second as i32,
                ptr,
            )
        }
        _ => {
            linux_trace!("Legacy IPC multiplexer call {} not fully implemented", call);
            linux_inval()
        }
    }
}

/// `select(2)` — Legacy version (superseded by pselect6).
pub fn sys_linux_select(
    n: usize,
    readfds: UserPtr<u8>,
    writefds: UserPtr<u8>,
    exceptfds: UserPtr<u8>,
    timeout: UserPtr<u8>,
) -> usize {
    // Proxy directly to the modern Linux-compat select path.
    crate::modules::linux_compat::net::sys_linux_select(
        n,
        readfds.cast(),
        writefds.cast(),
        exceptfds.cast(),
        timeout.cast(),
    )
}
