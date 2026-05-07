use crate::kernel::syscalls::linux_errno;
use super::state::*;

pub fn sys_linux_bpf(cmd: usize, attr_ptr: usize, size: usize) -> usize {
    const BPF_CMD_MAP_CREATE: usize = 0;
    const BPF_CMD_MAP_LOOKUP_ELEM: usize = 1;
    const BPF_CMD_MAP_UPDATE_ELEM: usize = 2;
    const BPF_CMD_MAP_DELETE_ELEM: usize = 3;
    const BPF_CMD_MAP_GET_NEXT_KEY: usize = 4;
    if attr_ptr == 0 || size == 0 {
        return linux_errno(crate::modules::posix_consts::errno::EFAULT);
    }
    if matches!(
        cmd,
        BPF_CMD_MAP_LOOKUP_ELEM
            | BPF_CMD_MAP_UPDATE_ELEM
            | BPF_CMD_MAP_DELETE_ELEM
            | BPF_CMD_MAP_GET_NEXT_KEY
    ) {
        return 0;
    }
    if cmd != BPF_CMD_MAP_CREATE {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    let id = NEXT_BPF_MAP_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    BPF_MAP_IDS.lock().insert(id);
    BPF_FD_BASE.saturating_add(id as usize)
}
