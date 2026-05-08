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

    unsafe {
        let current_l4 = crate::kernel::memory::paging::active_level_4_table(hhdm);
        let new_l4 = &mut *new_l4_virt;
        new_l4.zero();

        // 2. Setup Kernel Mappings (Entries 256..512) - Shallow Copy
        for i in 256..512 {
            new_l4[i] = current_l4[i].clone();
        }

        // 3. Deep Clone User Mappings (Entries 0..256)
        for i in 0..256 {
            if current_l4[i].is_unused() {
                continue;
            }
            let _ = recursive_clone_table(hhdm, &current_l4[i], &mut new_l4[i], 4, &mut frame_alloc);
        }
    }

    // 4. Mark all user-space writable pages as COW in the PARENT (Current) space
    let pid = current_process_id().ok_or("no process")?;
    if let Some(process) = crate::kernel::launch::process_arc_by_id(pid) {
        let mappings = process.mappings.lock().clone();
        unsafe {
            let mut pmgr = crate::kernel::memory::paging::PageManager::new(
                VirtAddr::new(hhdm),
                crate::kernel::memory::paging::active_level_4_table(hhdm),
            );

            for mrec in mappings {
                if (mrec.prot & crate::interfaces::memory::page_flags::WRITABLE) != 0 && mrec.map_id < 2_000_000 {
                    let flags = (mrec.prot & !crate::interfaces::memory::page_flags::WRITABLE) 
                                | crate::interfaces::memory::page_flags::COW;
                    let _ = pmgr.remap_range(mrec.start, mrec.end, flags, &mut frame_alloc);
                }
            }
        }
    }

    Ok(new_l4_phys)
}

unsafe fn recursive_clone_table(
    hhdm: u64,
    old_entry: &x86_64::structures::paging::PageTableEntry,
    new_entry: &mut x86_64::structures::paging::PageTableEntry,
    level: u8,
    alloc: &mut crate::hal::paging::PageAllocWrapper,
) -> Result<(), &'static str> {
    use x86_64::structures::paging::PageTableFlags as Flags;
    
    if old_entry.is_unused() {
        return Ok(());
    }

    if level == 1 || old_entry.flags().contains(Flags::HUGE_PAGE) {
        // Leaf entry (4KB or Huge Page)
        let mut flags = old_entry.flags();
        // If writable, mark as COW
        if flags.contains(Flags::WRITABLE) {
            flags.remove(Flags::WRITABLE);
            flags.insert(Flags::from_bits_truncate(crate::hal::paging::COW_BIT));
        }
        new_entry.set_addr(old_entry.addr(), flags);
        return Ok(());
    }

    // Allocate new table for next level
    let frame = alloc.allocate_frame().ok_or("OOM in deep clone")?;
    let phys = frame.start_address().as_u64();
    let new_table_virt = (phys + hhdm) as *mut x86_64::structures::paging::PageTable;
    let new_table = &mut *new_table_virt;
    new_table.zero();

    let old_table_virt = (old_entry.addr().as_u64() + hhdm) as *const x86_64::structures::paging::PageTable;
    let old_table = &*old_table_virt;

    for i in 0..512 {
        let _ = recursive_clone_table(hhdm, &old_table[i], &mut new_table[i], level - 1, alloc);
    }

    new_entry.set_addr(x86_64::PhysAddr::new(phys), old_entry.flags());
    Ok(())
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
