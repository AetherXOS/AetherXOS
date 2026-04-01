/// IPC Namespace — isolated System V IPC objects.
///
/// Each IPC namespace has independent semaphore sets, message queues,
/// and shared memory segments.
use super::{alloc_ns_id, NsId};
use core::sync::atomic::{AtomicU32, Ordering};

/// A single IPC namespace.
pub struct IpcNamespace {
    pub id: NsId,
    /// Next key for semaphore/message/shm IDs within this namespace.
    next_key: AtomicU32,
    /// Number of semaphore sets.
    pub sem_count: u32,
    /// Number of message queues.
    pub msg_count: u32,
    /// Number of shared memory segments.
    pub shm_count: u32,
}

impl IpcNamespace {
    /// Create the root IPC namespace.
    pub fn root() -> Self {
        Self {
            id: alloc_ns_id(),
            next_key: AtomicU32::new(1),
            sem_count: 0,
            msg_count: 0,
            shm_count: 0,
        }
    }

    /// Create a new empty IPC namespace.
    pub fn new() -> Self {
        Self {
            id: alloc_ns_id(),
            next_key: AtomicU32::new(1),
            sem_count: 0,
            msg_count: 0,
            shm_count: 0,
        }
    }

    /// Allocate a namespace-local IPC key.
    pub fn alloc_key(&self) -> u32 {
        self.next_key.fetch_add(1, Ordering::Relaxed)
    }
}
