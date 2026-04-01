use super::super::*;
use alloc::collections::BTreeMap;
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    static ref BRK_BY_PID: Mutex<BTreeMap<usize, usize>> = Mutex::new(BTreeMap::new());
}

/// `brk(2)` — legacy process heap end management.
pub fn sys_linux_brk(addr: usize) -> usize {
    let pid = current_process_id().unwrap_or(0);
    let mut table = BRK_BY_PID.lock();
    let current = table.entry(pid).or_insert(linux::BRK_START as usize);

    if addr == 0 || addr == *current {
        return *current;
    }
    if addr < linux::BRK_START as usize {
        return *current;
    }
    if addr >= crate::hal::syscalls_consts::USER_SPACE_TOP_EXCLUSIVE {
        return *current;
    }

    *current = addr;
    addr
}
