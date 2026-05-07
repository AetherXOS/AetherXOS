use crate::kernel::syscalls::{linux_errno, with_user_read_bytes, with_user_write_bytes};

pub fn sys_linux_membarrier(cmd: usize, _flags: usize, _cpu_id: usize) -> usize {
    const MEMBARRIER_CMD_QUERY: usize = 0;
    const MEMBARRIER_CMD_GLOBAL: usize = 1 << 0;
    const MEMBARRIER_CMD_GLOBAL_EXPEDITED: usize = 1 << 1;
    const MEMBARRIER_CMD_REGISTER_GLOBAL_EXPEDITED: usize = 1 << 2;
    const MEMBARRIER_CMD_PRIVATE_EXPEDITED: usize = 1 << 3;
    const MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED: usize = 1 << 4;
    const MEMBARRIER_CMD_PRIVATE_EXPEDITED_SYNC_CORE: usize = 1 << 5;

    let supported = MEMBARRIER_CMD_GLOBAL
        | MEMBARRIER_CMD_GLOBAL_EXPEDITED
        | MEMBARRIER_CMD_REGISTER_GLOBAL_EXPEDITED
        | MEMBARRIER_CMD_PRIVATE_EXPEDITED
        | MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED
        | MEMBARRIER_CMD_PRIVATE_EXPEDITED_SYNC_CORE;

    if cmd == MEMBARRIER_CMD_QUERY {
        return supported;
    }
    if (cmd & supported) != 0 {
        return 0;
    }
    linux_errno(crate::modules::posix_consts::errno::EINVAL)
}

pub fn sys_linux_rseq(
    rseq_ptr: usize,
    rseq_len: usize,
    flags: usize,
    _sig: usize,
) -> usize {
    // Many modern runtimes probe rseq during startup; accepting valid registration avoids
    // brittle early-process failures while still rejecting malformed descriptors.
    if flags != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if rseq_ptr == 0 || rseq_len < 32 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    // Enforce user-space memory reachability up-front so later rseq reads/writes have
    // deterministic EFAULT behavior instead of deferred faults.
    if with_user_read_bytes(rseq_ptr, rseq_len, |_| 0usize).is_err() {
        return linux_errno(crate::modules::posix_consts::errno::EFAULT);
    }
    if with_user_write_bytes(rseq_ptr, rseq_len, |_| 0usize).is_err() {
        return linux_errno(crate::modules::posix_consts::errno::EFAULT);
    }
    0
}
