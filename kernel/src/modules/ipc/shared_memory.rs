use crate::interfaces::{KernelError, KernelResult};
use crate::kernel::sync::IrqSafeMutex;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicUsize, Ordering};
use lazy_static::lazy_static;

use crate::interfaces::task::{ProcessId, TaskId};

/// Key used by shmget(2) to identify a shared memory segment.
pub type ShmKey = i32;
/// Identifier returned by shmget(2), used by shmat/shmdt.
pub type ShmId = i32;

pub const IPC_PRIVATE: ShmKey = 0;

/// RAII guard for physical pages of a shared memory segment.
/// Automatically frees pages when the last reference is dropped.
#[derive(Debug)]
pub struct ShmPages {
    pages: alloc::vec::Vec<usize>,
}

impl ShmPages {
    pub fn new(pages: alloc::vec::Vec<usize>) -> Self {
        Self { pages }
    }
}

impl Drop for ShmPages {
    fn drop(&mut self) {
        let mut alloc = crate::modules::allocators::selector::ActivePageAllocator::new();
        for &page in &self.pages {
            // Free the physical page.
            // In HyperCore, 0 is the order for a single 4KB page.
            alloc.free_pages(page, 0);
        }
    }
}

#[derive(Debug, Clone)]
pub struct ShmRegion {
    pub id: ShmId,
    pub key: ShmKey,
    pub size: usize,
    pub owner: ProcessId,
    pub creator_tid: TaskId,
    /// RAII-wrapped physical pages.
    pub physical_pages: Arc<ShmPages>,
    pub permissions: u32,
}

struct ShmState {
    regions: BTreeMap<ShmId, ShmRegion>,
    key_to_id: BTreeMap<ShmKey, ShmId>,
    next_id: i32,
}

lazy_static! {
    static ref SHM_MANAGER: IrqSafeMutex<ShmState> = IrqSafeMutex::new(ShmState {
        regions: BTreeMap::new(),
        key_to_id: BTreeMap::new(),
        next_id: 2000000,
    });
}

pub fn shm_get(key: ShmKey, size: usize, flags: u32) -> KernelResult<ShmId> {
    let mut state = SHM_MANAGER.lock();

    if key != IPC_PRIVATE {
        if let Some(&id) = state.key_to_id.get(&key) {
            return Ok(id);
        }
    }

    let id = state.next_id;
    state.next_id += 1;

    let num_pages = (size + 4095) / 4096;
    let mut pages = alloc::vec::Vec::with_capacity(num_pages);

    {
        let mut alloc = crate::modules::allocators::selector::ActivePageAllocator::new();
        for _ in 0..num_pages {
            let page = alloc.allocate_pages(0).ok_or(KernelError::NoMemory)?;
            pages.push(page);
        }
    }

    let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
    let tid = cpu.current_task_id();
    let pid = crate::kernel::launch::process_id_by_task(tid)
        .map(|p| p.0)
        .unwrap_or(0);

    let region = ShmRegion {
        id,
        key,
        size: num_pages * 4096,
        owner: ProcessId(pid),
        creator_tid: tid,
        physical_pages: Arc::new(ShmPages::new(pages)),
        permissions: flags & 0o777,
    };

    if key != IPC_PRIVATE {
        state.key_to_id.insert(key, id);
    }
    state.regions.insert(id, region);

    Ok(id)
}

pub fn shm_get_region(id: ShmId) -> Option<ShmRegion> {
    SHM_MANAGER.lock().regions.get(&id).cloned()
}

pub fn shm_rmid(id: ShmId) -> KernelResult<()> {
    let mut state = SHM_MANAGER.lock();
    if let Some(region) = state.regions.remove(&id) {
        if region.key != IPC_PRIVATE {
            state.key_to_id.remove(&region.key);
        }
        // The physical pages will be freed automatically when all processes
        // detach and the reference count of Arc<ShmPages> becomes zero.
        Ok(())
    } else {
        Err(KernelError::NotFound)
    }
}

impl core::ops::Deref for ShmPages {
    type Target = [usize];
    fn deref(&self) -> &Self::Target {
        &self.pages
    }
}
