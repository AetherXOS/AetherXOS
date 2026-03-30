use core::sync::atomic::Ordering;

use crate::interfaces::task::TaskId;

#[cfg(feature = "ipc_shared_memory")]
pub const IPC_PAGE_SIZE_BYTES: usize = 4096;

#[cfg(any(feature = "ipc_ring_buffer", feature = "ipc_zero_copy"))]
pub use crate::generated_consts::IPC_RING_BUFFER_SIZE_BYTES;

#[cfg(feature = "ipc_unix_domain")]
#[allow(unused_imports)]
pub use crate::generated_consts::IPC_UNIX_SOCKET_QUEUE_LIMIT;

#[cfg(feature = "ipc_binder")]
pub use crate::generated_consts::IPC_BINDER_MAX_OBJECTS;

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
