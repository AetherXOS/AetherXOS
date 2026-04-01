use super::super::*;
use crate::modules::linux_compat::config::LinuxCompatConfig;

/// `futex(2)` — Fast Userspace Mutex.
pub mod futex_op {
    pub const FUTEX_WAIT: usize = 0;
    pub const FUTEX_WAKE: usize = 1;
    pub const FUTEX_FD: usize = 2;
    pub const FUTEX_REQUEUE: usize = 3;
    pub const FUTEX_CMP_REQUEUE: usize = 4;
    pub const FUTEX_WAKE_OP: usize = 5;
    pub const FUTEX_LOCK_PI: usize = 6;
    pub const FUTEX_UNLOCK_PI: usize = 7;
    pub const FUTEX_TRYLOCK_PI: usize = 8;
    pub const FUTEX_WAIT_BITSET: usize = 9;
    pub const FUTEX_WAKE_BITSET: usize = 10;
    pub const FUTEX_WAIT_REQUEUE_PI: usize = 11;
    pub const FUTEX_CMP_REQUEUE_PI: usize = 12;

    pub const FUTEX_PRIVATE_FLAG: usize = 128;
    pub const FUTEX_CLOCK_REALTIME: usize = 256;
    pub const FUTEX_CMD_MASK: usize = !(FUTEX_PRIVATE_FLAG | FUTEX_CLOCK_REALTIME);
    pub const FUTEX_BITSET_MATCH_ANY: u32 = 0xFFFF_FFFF;
}

const FUTEX_WORD_ALIGN_MASK: usize = 0x3;
const MILLIS_PER_SECOND: i64 = 1_000;
const NANOS_PER_MILLISECOND: i64 = 1_000_000;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxFutexWaitv {
    pub val: u64,
    pub uaddr: u64,
    pub flags: u32,
    pub __reserved: u32,
}

#[inline(always)]
fn read_futex_word(uaddr: usize) -> Option<u32> {
    if uaddr == 0 || (uaddr & FUTEX_WORD_ALIGN_MASK) != 0 {
        return None;
    }
    let ptr = uaddr as *const core::sync::atomic::AtomicU32;
    unsafe { Some((*ptr).load(core::sync::atomic::Ordering::Relaxed)) }
}

#[inline(always)]
fn futex_key_hint_for_scope(uaddr: usize, is_private: bool) -> usize {
    if !is_private {
        return 0;
    }
    let pid = crate::kernel::launch::current_process_arc()
        .map(|p| p.id.0 as u64)
        .unwrap_or(0);
    let mixed = ((uaddr as u64) << 32) ^ pid;
    mixed as usize
}

pub fn sys_linux_futex(
    uaddr: usize,
    op: usize,
    val: usize,
    utime_ptr: UserPtr<types::LinuxTimespec>,
    uaddr2: usize,
    val3: usize,
) -> usize {
    let cmd = op & futex_op::FUTEX_CMD_MASK;
    let is_private = (op & futex_op::FUTEX_PRIVATE_FLAG) != 0;
    let scoped_key_hint = futex_key_hint_for_scope(uaddr, is_private);

    match cmd {
        futex_op::FUTEX_WAIT | futex_op::FUTEX_WAIT_BITSET => {
            let bitset = if cmd == futex_op::FUTEX_WAIT_BITSET {
                val3 as u32
            } else {
                futex_op::FUTEX_BITSET_MATCH_ANY
            };
            if bitset == 0 {
                return linux_inval();
            }

            let timeout_ms = if !utime_ptr.is_null() {
                match utime_ptr.read() {
                    Ok(ts) => ts
                        .tv_sec
                        .saturating_mul(MILLIS_PER_SECOND)
                        .saturating_add(ts.tv_nsec / NANOS_PER_MILLISECOND)
                        as usize,
                    Err(_) => return linux_fault(),
                }
            } else {
                0
            };

            if !utime_ptr.is_null() && timeout_ms == 0 {
                return linux_errno(crate::modules::posix_consts::errno::ETIMEDOUT);
            }

            if let Some(w) = read_futex_word(uaddr) {
                if w != val as u32 {
                    return linux_errno(crate::modules::posix_consts::errno::EAGAIN);
                }
            } else {
                return linux_fault();
            }

            let _ = timeout_ms;
            let rc = sys_futex_wait(uaddr, val, scoped_key_hint ^ bitset as usize);
            if rc == 0 {
                0
            } else {
                // Current kernel futex backend reports mismatch via non-zero return.
                linux_errno(crate::modules::posix_consts::errno::EAGAIN)
            }
        }

        futex_op::FUTEX_WAKE | futex_op::FUTEX_WAKE_BITSET => {
            let bitset = if cmd == futex_op::FUTEX_WAKE_BITSET {
                val3 as u32
            } else {
                futex_op::FUTEX_BITSET_MATCH_ANY
            };
            if bitset == 0 {
                return linux_inval();
            }
            let woke = sys_futex_wake(uaddr, val, scoped_key_hint ^ bitset as usize);
            if woke == !0 {
                0
            } else {
                woke
            }
        }

        futex_op::FUTEX_REQUEUE | futex_op::FUTEX_CMP_REQUEUE => {
            if cmd == futex_op::FUTEX_CMP_REQUEUE {
                if let Some(w) = read_futex_word(uaddr) {
                    if w != val3 as u32 {
                        return linux_errno(crate::modules::posix_consts::errno::EAGAIN);
                    }
                } else {
                    return linux_fault();
                }
            }
            if uaddr2 == 0 || (uaddr2 & FUTEX_WORD_ALIGN_MASK) != 0 {
                return linux_fault();
            }
            let _ = utime_ptr;
            // Best-effort compatibility: we can wake the source waiters, but without a true
            // kernel requeue primitive we must not spuriously wake the target futex waiters.
            let woke = sys_futex_wake(
                uaddr,
                val,
                scoped_key_hint ^ futex_op::FUTEX_BITSET_MATCH_ANY as usize,
            );
            let _ = val3;
            woke
        }

        futex_op::FUTEX_WAKE_OP => {
            let target_key_hint = futex_key_hint_for_scope(uaddr2, is_private);
            let woke = sys_futex_wake(uaddr, val, scoped_key_hint);
            let woke2 = sys_futex_wake(uaddr2, val3, target_key_hint);
            woke + woke2
        }

        _ => linux_inval(),
    }
}

pub fn sys_linux_futex_waitv(
    waiters_ptr: UserPtr<LinuxFutexWaitv>,
    nr: usize,
    flags: usize,
    _timeout: UserPtr<types::LinuxTimespec>,
) -> usize {
    if flags != 0 || nr == 0 || nr > LinuxCompatConfig::FUTEX_WAITV_MAX {
        return linux_inval();
    }
    let scoped_private_hint = futex_key_hint_for_scope(0, true);
    for i in 0..nr {
        let w = match waiters_ptr.add(i).read() {
            Ok(v) => v,
            Err(e) => return e,
        };
        if w.uaddr == 0 || (w.uaddr as usize & FUTEX_WORD_ALIGN_MASK) != 0 {
            return linux_fault();
        }
        if let Some(cur) = read_futex_word(w.uaddr as usize) {
            if cur != w.val as u32 {
                return i;
            }
        } else {
            return linux_fault();
        }
        let private = (w.flags as usize & futex_op::FUTEX_PRIVATE_FLAG) != 0;
        let key_hint = if private {
            scoped_private_hint ^ (w.uaddr as usize)
        } else {
            0
        };
        let _ = sys_futex_wait(w.uaddr as usize, w.val as usize, key_hint);
    }
    0
}

pub fn sys_linux_set_robust_list(head: usize, len: usize) -> usize {
    if len != LinuxCompatConfig::ROBUST_LIST_HEAD_SIZE {
        return linux_inval();
    }
    let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
    let current_tid = cpu.current_task.load(core::sync::atomic::Ordering::Relaxed);
    crate::kernel::syscalls::set_robust_list_for_tid(current_tid, head, len);
    0
}

pub fn sys_linux_get_robust_list(
    pid: i32,
    head_ptr: UserPtr<usize>,
    len_ptr: UserPtr<usize>,
) -> usize {
    if head_ptr.is_null() || len_ptr.is_null() {
        return linux_fault();
    }

    let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
    let current_tid = cpu.current_task.load(core::sync::atomic::Ordering::Relaxed) as i32;
    let target_tid = if pid == 0 { current_tid } else { pid };
    if pid != 0 && pid != current_tid {
        return linux_eperm();
    }

    if crate::kernel::task::get_task(crate::interfaces::task::TaskId(target_tid as usize)).is_none()
    {
        return linux_errno(crate::modules::posix_consts::errno::ESRCH);
    }

    let (head, len) = crate::kernel::syscalls::robust_list_for_tid(target_tid as usize)
        .unwrap_or((0, LinuxCompatConfig::ROBUST_LIST_HEAD_SIZE));
    let _ = head_ptr.write(&head);
    let _ = len_ptr.write(&len);
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn futex_waitv_rejects_invalid_bounds() {
        assert_eq!(
            sys_linux_futex_waitv(UserPtr::new(0), 0, 0, UserPtr::new(0)),
            linux_inval()
        );
        assert_eq!(
            sys_linux_futex_waitv(
                UserPtr::new(0),
                LinuxCompatConfig::FUTEX_WAITV_MAX + 1,
                0,
                UserPtr::new(0),
            ),
            linux_inval()
        );
        assert_eq!(
            sys_linux_futex_waitv(UserPtr::new(0), 1, 1, UserPtr::new(0)),
            linux_inval()
        );
    }

    #[test_case]
    fn robust_list_len_is_validated() {
        assert_eq!(
            sys_linux_set_robust_list(0, LinuxCompatConfig::ROBUST_LIST_HEAD_SIZE + 1),
            linux_inval()
        );
        assert_eq!(
            sys_linux_set_robust_list(0x1000, LinuxCompatConfig::ROBUST_LIST_HEAD_SIZE),
            0
        );
    }

    #[test_case]
    fn futex_wait_bitset_rejects_zero_mask() {
        assert_eq!(
            sys_linux_futex(
                0x1000,
                futex_op::FUTEX_WAIT_BITSET,
                0,
                UserPtr::new(0),
                0,
                0
            ),
            linux_inval()
        );
    }

    #[test_case]
    fn futex_wake_bitset_rejects_zero_mask() {
        assert_eq!(
            sys_linux_futex(
                0x1000,
                futex_op::FUTEX_WAKE_BITSET,
                1,
                UserPtr::new(0),
                0,
                0
            ),
            linux_inval()
        );
    }

    #[test_case]
    fn futex_requeue_rejects_unaligned_target() {
        assert_eq!(
            sys_linux_futex(
                0x1000,
                futex_op::FUTEX_REQUEUE,
                1,
                UserPtr::new(0),
                0x1002,
                0
            ),
            linux_fault()
        );
    }
}
