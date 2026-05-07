use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::sync::atomic::Ordering;
use aethercore_common::units::PAGE_SIZE_4K;

use crate::interfaces::task::TaskId;

pub const IPC_PAGE_SIZE_BYTES: usize = PAGE_SIZE_4K;


#[cfg(feature = "ipc_unix_domain")]
#[allow(unused_imports)]
pub use crate::generated_consts::IPC_UNIX_SOCKET_QUEUE_LIMIT;


#[cfg(feature = "ipc_futex")]
#[allow(unused_imports)]
pub use crate::generated_consts::IPC_FUTEX_WAKE_EVENT_LIMIT;

#[inline(always)]
#[cfg(feature = "ipc_shared_memory")]
pub fn align_to_page_or_default(size: usize) -> usize {
    if size == 0 {
        IPC_PAGE_SIZE_BYTES
    } else {
        size.saturating_add(IPC_PAGE_SIZE_BYTES - 1) & !(IPC_PAGE_SIZE_BYTES - 1)
    }
}

#[inline(always)]
#[allow(dead_code)]
pub fn current_task_id_or_kernel() -> TaskId {
    unsafe {
        crate::kernel::cpu_local::CpuLocal::try_get()
            .map(|cpu| TaskId(cpu.current_task.load(Ordering::Relaxed)))
            .unwrap_or(TaskId(0))
    }
}

#[inline(always)]
pub fn bounded_push_bytes(
    queue: &mut VecDeque<Vec<u8>>,
    payload: &[u8],
    queue_limit: usize,
) -> bool {
    if queue.len() >= queue_limit {
        return false;
    }
    queue.push_back(payload.to_vec());
    true
}

#[inline(always)]
pub fn wake_one_task(wait_queue: &crate::kernel::sync::WaitQueue) {
    if let Some(tid) = wait_queue.wake_one() {
        crate::kernel::task::wake_task(tid);
    }
}

#[inline(always)]
pub fn wake_tasks_with_mask(wait_queue: &crate::kernel::sync::WaitQueue, mask: u32, max: usize) -> usize {
    let mut woken = 0;
    while woken < max {
        if let Some(tid) = wait_queue.wake_one_with_mask(mask) {
            crate::kernel::task::wake_task(tid);
            woken += 1;
        } else {
            break;
        }
    }
    woken
}

#[inline(always)]
pub fn suspend_on(wait_queue: &crate::kernel::sync::WaitQueue) {
    crate::kernel::task::suspend_current_task(wait_queue);
}

#[inline(always)]
pub fn suspend_on_with_mask(wait_queue: &crate::kernel::sync::WaitQueue, mask: u32) {
    crate::kernel::task::suspend_current_task_with_mask(wait_queue, mask);
}

