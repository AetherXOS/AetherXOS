use crate::interfaces::memory::PageAllocator;
use crate::interfaces::{KernelError, KernelResult};
use crate::kernel::sync::IrqSafeMutex;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::sync::atomic::Ordering;
use lazy_static::lazy_static;

use crate::interfaces::task::{ProcessId, TaskId};
use super::common::{align_to_page_or_default, IPC_PAGE_SIZE_BYTES};

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
            // In AetherCore, 0 is the order for a single 4KB page.
            alloc.deallocate_pages(page, 0);
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

    let num_pages = align_to_page_or_default(size) / IPC_PAGE_SIZE_BYTES;
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
        size: num_pages * IPC_PAGE_SIZE_BYTES,
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
        Ok(())
    } else {
        Err(KernelError::NotFound)
    }
}

pub fn shm_attach(id: ShmId, requested_addr: u64, flags: u32) -> KernelResult<u64> {
    let region = shm_get_region(id).ok_or(KernelError::NotFound)?;
    
    let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
    let current_tid = cpu.current_task_id();
    let process = crate::kernel::launch::process_arc_by_id(
        crate::kernel::launch::process_id_by_task(current_tid).ok_or(KernelError::InvalidTask)?
    ).ok_or(KernelError::NotFound)?;

    let mut addr = requested_addr;
    if addr == 0 {
        // Simple allocator for SHM addresses (in a real kernel this would be more robust)
        addr = process.next_mapping_hint.fetch_add(region.size as u64, Ordering::SeqCst);
    }

    // Register mapping in process
    // We use map_id = 2_000_000 + id to tell the VMM this is an SHM region
    let map_id = 2_000_000 + (id as u32);
    let prot = crate::interfaces::memory::page_flags::PRESENT | 
               crate::interfaces::memory::page_flags::USER |
               (if (flags & 0o10000) == 0 { crate::interfaces::memory::page_flags::WRITABLE } else { 0 });

    process.register_mapping(map_id, addr, addr + region.size as u64, prot, flags)?;
    
    Ok(addr)
}

pub fn shm_detach(addr: u64) -> KernelResult<()> {
    let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
    let current_tid = cpu.current_task_id();
    let process = crate::kernel::launch::process_arc_by_id(
        crate::kernel::launch::process_id_by_task(current_tid).ok_or(KernelError::InvalidTask)?
    ).ok_or(KernelError::NotFound)?;

    let mut mappings = process.mappings.lock();
    if let Some(pos) = mappings.iter().position(|m| m.start == addr && m.map_id >= 2_000_000) {
        mappings.remove(pos);
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
