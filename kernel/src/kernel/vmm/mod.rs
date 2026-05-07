// only compile when paging support exists
#![cfg(feature = "paging_enable")]

use crate::kernel::sync::IrqSafeMutex;
use crate::modules::allocators::selector::ActivePageAllocator;
use x86_64::VirtAddr;

pub(crate) static GLOBAL_PAGE_ALLOC: IrqSafeMutex<ActivePageAllocator> =
    IrqSafeMutex::new(ActivePageAllocator::new());



/// Called on a page-fault IRQ. If the faulting address lies within
/// one of the current process's mmap mappings, we materialize the page
/// and initialize contents (zero for anonymous, read from file for file-backed).
pub fn clone_current_address_space() -> Result<u64, &'static str> {
    let hhdm = crate::hal::hhdm_offset().ok_or("no hhdm")?;
    let mut frame_alloc = crate::hal::paging::PageAllocWrapper;

    // 1. Allocate a new Level 4 Page Table
    let new_l4_frame = frame_alloc
        .allocate_frame()
        .ok_or("failed to allocate l4 frame")?;
    let new_l4_phys = new_l4_frame.start_address().as_u64();
    let new_l4_virt = (new_l4_phys + hhdm) as *mut x86_64::structures::paging::PageTable;

    // 2. Clear and setup kernel mappings (higher half)
    unsafe {
        let current_l4 = crate::kernel::memory::paging::active_level_4_table(hhdm);
        let new_l4 = &mut *new_l4_virt;
        new_l4.zero();

        // Copy kernel mappings (entries 256..512 for x86_64)
        for i in 256..512 {
            new_l4[i] = current_l4[i].clone();
        }
    }

    // 3. Mark all user-space writable pages as COW in BOTH parent and child
    let pid = current_process_id().ok_or("no process")?;
    let process = crate::kernel::launch::process_arc_by_id(pid).ok_or("no process arc")?;
    let mappings = process.mappings.lock().clone();

    unsafe {
        let mut pmgr = crate::kernel::memory::paging::PageManager::new(
            VirtAddr::new(hhdm),
            crate::kernel::memory::paging::active_level_4_table(hhdm),
        );

        for mrec in mappings {
            // We only COW writable regions that are not shared
            if (mrec.prot & crate::interfaces::memory::page_flags::WRITABLE) != 0 && mrec.map_id < 2_000_000 {
                // Remap range with COW bit set in current (parent) space
                let flags = mrec.prot | crate::interfaces::memory::page_flags::COW;
                let _ = pmgr.remap_range(mrec.start, mrec.end, flags, &mut frame_alloc);
            }
        }

        // 4. Copy user-space page table structures to the child
        // For x86_64, we can just copy entries 0..256
        let current_l4 = crate::kernel::memory::paging::active_level_4_table(hhdm);
        let new_l4 = &mut *new_l4_virt;
        for i in 0..256 {
            new_l4[i] = current_l4[i].clone();
        }
    }

    Ok(new_l4_phys)
}

fn current_process_id() -> Option<crate::interfaces::task::ProcessId> {
    let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
    let current_tid = crate::interfaces::task::TaskId(
        cpu.current_task.load(core::sync::atomic::Ordering::Relaxed),
    );
    crate::kernel::launch::process_id_by_task(current_tid)
}

pub fn init() {
    // Page fault handling is now centralized in crate::kernel::memory::demand_paging
    // via the IDT exception handler. No dispatcher registration needed here.
}
