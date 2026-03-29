// only compile when paging support exists
#![cfg(feature = "paging_enable")]

use crate::interfaces::cpu::CpuRegisters;
use crate::interfaces::dispatcher::Dispatcher;
use crate::interfaces::memory::PageAllocator;
use crate::kernel::sync::IrqSafeMutex;
use crate::modules::allocators::selector::ActivePageAllocator;
use x86_64::structures::paging::{FrameAllocator, PhysFrame, Size4KiB};
use x86_64::PhysAddr;
use x86_64::VirtAddr;

static GLOBAL_PAGE_ALLOC: IrqSafeMutex<ActivePageAllocator> =
    IrqSafeMutex::new(ActivePageAllocator::new());

/// Thin adapter implementing `FrameAllocator` by delegating to the global
/// page allocator.  The implementation locks the mutex on each call so that
/// the allocator may be invoked from different CPUs.
pub struct PageAllocWrapper;

// FrameAllocator trait is unsafe
unsafe impl FrameAllocator<Size4KiB> for PageAllocWrapper {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        let mut alloc = GLOBAL_PAGE_ALLOC.lock();
        alloc
            .allocate_pages(0)
            .map(|addr| PhysFrame::containing_address(PhysAddr::new(addr as u64)))
    }
}

/// Called on a page-fault IRQ (vector 14).  If the faulting address lies within
/// one of the current process's mmap mappings, we materialize the entire region
/// and initialize contents (zero for anonymous, read from file for file-backed).
fn vmm_page_fault_handler(_irq: u8) {
    // avoid unused imports if paging not enabled (function gated by above attribute)
    let fault_addr = crate::hal::cpu::ArchCpuRegisters::read_page_fault_addr();
    // simple user-space range check copied from syscalls.rs
    const USER_SPACE_BOTTOM_INCLUSIVE: u64 = 0x1000;
    const USER_SPACE_TOP_EXCLUSIVE: u64 = 0x0000_8000_0000_0000;
    if fault_addr < USER_SPACE_BOTTOM_INCLUSIVE || fault_addr >= USER_SPACE_TOP_EXCLUSIVE {
        return;
    }

    // grab the current process id
    // replicate current_process_id logic locally to avoid privacy issues
    let pid = {
        let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
        let current_tid = crate::interfaces::task::TaskId(
            cpu.current_task.load(core::sync::atomic::Ordering::Relaxed),
        );
        match crate::kernel::launch::process_id_by_task(current_tid) {
            Some(p) => p.0,
            None => return,
        }
    };

    let process =
        match crate::kernel::launch::process_arc_by_id(crate::interfaces::task::ProcessId(pid)) {
            Some(p) => p,
            None => return,
        };

    // find mapping record containing fault_addr
    let map_rec = {
        let maps = process.mappings.lock();
        maps.iter()
            .find(|m| fault_addr >= m.start && fault_addr < m.end)
            .cloned()
    };

    let mrec = match map_rec {
        Some(r) => r,
        None => return,
    };

    // create PageManager for current address space
    if let Some(hhdm) = crate::hal::hhdm_offset() {
        unsafe {
            let lvl4 = crate::kernel::memory::paging::active_level_4_table(VirtAddr::new(hhdm));
            let mut pmgr =
                crate::kernel::memory::paging::PageManager::new(VirtAddr::new(hhdm), lvl4);
            let mut frame_alloc = PageAllocWrapper;

            // materialize entire region for simplicity
            let _ = crate::kernel::module_loader::materialize_virtual_mapping_range(
                mrec.start,
                mrec.end,
                mrec.prot,
                &mut pmgr,
                &mut frame_alloc,
            );

            // fill contents
            let page_size = crate::interfaces::memory::PAGE_SIZE_4K as u64;
            let mut off = 0u64;
            while off < (mrec.end - mrec.start) {
                let page_va = mrec.start + off;
                let kernel_va = hhdm + page_va;
                let ptr = kernel_va as *mut u8;
                if mrec.map_id >= 2_000_000 {
                    // shared memory: resolve from shm module
                    if let Some(shm) =
                        crate::modules::ipc::shared_memory::shm_get_region(mrec.map_id as i32)
                    {
                        // For SHM, we don't need to read from file or zero, we map the EXISTING frames.
                        // However, we need to do this mapping correctly in the page table.
                        // apply_shm_mapping can do this for us.
                        let _ = pmgr.apply_shm_mapping(
                            mrec.start,
                            mrec.end,
                            &shm.physical_pages,
                            flags,
                            &mut frame_alloc,
                        );
                        return; // Done for this range
                    }
                } else if mrec.map_id >= 1_000_000 {
                    // anonymous: zero
                    core::ptr::write_bytes(ptr, 0, page_size as usize);
                } else {
                    // file-backed: read from posix mman
                    let slice = core::slice::from_raw_parts_mut(ptr, page_size as usize);
                    let _ =
                        crate::modules::posix::mman::mmap_read(mrec.map_id, slice, off as usize);
                }
                off += page_size;
            }
        }
    }
}

/// Initialize the virtual memory manager.  Must be called after the dispatcher
/// is set up (e.g. in kernel_runtime) so that we can register our page fault
/// handler.
pub fn init() {
    #[cfg(feature = "dispatcher")]
    {
        let irq = 14u8;
        unsafe {
            // idt::DISPATCHER is private; we register via the public function
            // exposed on the dispatcher module by obtaining the dispatcher again.
            // There's no getter so we simply re-create a handle; the ActiveDispatcher
            // type wraps a global shared state internally so cloning is cheap.
            let disp = crate::modules::dispatcher::selector::ActiveDispatcher::new();
            disp.register_handler(irq, vmm_page_fault_handler);
        }
    }
}
